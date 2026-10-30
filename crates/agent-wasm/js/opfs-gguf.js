// opfs-gguf.js: downloads one GGUF file into the origin private file system
// (OPFS) and hands back a FileSystemSyncAccessHandle on it, which
// `Agent.load` reads at offsets, one tensor at a time. Neither JS nor wasm
// memory ever holds the whole file. Run it in a dedicated Web Worker: sync
// access handles exist nowhere else.
//
// The file is fetched with HTTP Range requests of `chunkBytes` each; every
// piece of a response is written at its offset as it arrives. After each
// chunk the file is flushed and a small progress record, `<name>.json` next
// to `<name>`, is rewritten with the number of bytes on disk, so a download
// cut short resumes from the last whole chunk. A server that ignores Range
// sends the whole file, which is written from byte 0. When the caller gives
// the file's SHA-256, the bytes are hashed as they arrive and the hash state
// is kept in the record with the byte count, so a resumed download never
// reads the file back.
//
// Layout: one OPFS directory per caller-chosen `dir` (default "models"),
// holding `<name>` and `<name>.json`.
//
// When the browser has no usable OPFS (Safari private browsing, quota
// errors, no sync access handles), `opfsUnavailable()` says why and the
// caller keeps loading the file into memory.

// Why OPFS cannot be used here, or null when it can: 'safari-private',
// 'quota-exceeded', 'no-sync-handle' or 'generic'. Probes by calling
// getDirectory(), which throws in Safari private browsing.
export async function opfsUnavailable() {
  if (!('storage' in navigator) || typeof navigator.storage.getDirectory !== 'function') {
    return 'generic';
  }
  try {
    await navigator.storage.getDirectory();
  } catch (err) {
    const isSafari = /^((?!chrome|android).)*safari/i.test(navigator.userAgent);
    if (isSafari && err.name === 'SecurityError') return 'safari-private';
    if (err.name === 'QuotaExceededError') return 'quota-exceeded';
    return 'generic';
  }
  if (typeof FileSystemFileHandle === 'undefined' || typeof FileSystemFileHandle.prototype.createSyncAccessHandle !== 'function') {
    return 'no-sync-handle';
  }
  return null;
}

// Asks the browser to keep this origin's storage under storage pressure.
// StorageManager.persist() exists only on a window, so the page calls this;
// in a worker it reports whether storage is already persistent. Firefox
// answers through a permission prompt, so the promise can stay pending until
// the visitor clicks: never wait for it before loading.
export async function askPersist() {
  try {
    if (typeof navigator.storage.persist === 'function') return await navigator.storage.persist();
    if (typeof navigator.storage.persisted === 'function') return await navigator.storage.persisted();
  } catch (_) { /* best effort */ }
  return false;
}

async function readRecord(dir, name) {
  try {
    const file = await (await dir.getFileHandle(`${name}.json`)).getFile();
    return JSON.parse(await file.text());
  } catch (_) {
    return null;
  }
}

async function writeRecord(dir, name, record) {
  const h = await (await dir.getFileHandle(`${name}.json`, { create: true })).createSyncAccessHandle();
  try {
    const bytes = new TextEncoder().encode(JSON.stringify(record));
    h.truncate(0);
    h.write(bytes, { at: 0 });
    h.flush();
  } finally {
    h.close();
  }
}

// The total length from `Content-Range: bytes a-b/total`, or null.
function rangeTotal(r) {
  const m = /\/(\d+)\s*$/.exec(r.headers.get('content-range') || '');
  return m ? Number(m[1]) : null;
}

function rangeStart(r) {
  const m = /^bytes\s+(\d+)-/.exec(r.headers.get('content-range') || '');
  return m ? Number(m[1]) : null;
}

// Writes a response body at `at`, piece by piece, hashing it into `hash`
// when there is one. Returns the end offset.
async function writeBody(handle, body, at, hash, onBytes) {
  const reader = body.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) return at;
    if (hash) hash.update(value);
    let off = 0;
    while (off < value.length) {
      const n = handle.write(off ? value.subarray(off) : value, { at: at + off });
      if (n <= 0) throw new Error('OPFS write wrote nothing (storage full?)');
      off += n;
    }
    at += value.length;
    onBytes(at);
  }
}

// downloadGguf(url, { name, dir, sha256, chunkBytes, onProgress })
//
// - name: the file name in OPFS (default: the last path segment of url).
// - dir: the OPFS directory (default 'models').
// - sha256: the file's known SHA-256 in hex (a Hugging Face file's LFS
//   sha256), checked when the download completes; without it the check is
//   the size the server announced and the GGUF magic.
// - chunkBytes: bytes per Range request (default 64 MiB, a multiple of 64).
// - onProgress(loaded, total).
//
// Resolves to { handle, size }: an open FileSystemSyncAccessHandle on the
// complete, checked file. Pass it to Agent.load, then handle.close().
// A file already complete and checked is opened without any request.
export async function downloadGguf(url, opts = {}) {
  const {
    name = decodeURIComponent(new URL(url, self.location.href).pathname.split('/').pop()),
    dir: dirName = 'models',
    sha256 = null,
    chunkBytes = 64 << 20,
    onProgress = () => {},
  } = opts;

  const root = await navigator.storage.getDirectory();
  const dir = await root.getDirectoryHandle(dirName, { create: true });
  let record = await readRecord(dir, name);
  const handle = await (await dir.getFileHandle(name, { create: true })).createSyncAccessHandle();
  try {
    const same = record && record.url === url && record.sha256 === sha256;
    if (same && record.complete && record.verified && handle.getSize() === record.size) {
      onProgress(record.size, record.size);
      return { handle, size: record.size };
    }
    const fresh = () => ({ url, sha256, size: null, etag: null, have: 0, hash: null, complete: false, verified: false });
    if (!same || handle.getSize() < record.have || record.complete) {
      record = fresh();
      handle.truncate(0);
    }

    while (!record.complete) {
      const start = record.have;
      let hash = sha256 ? Sha256.resume(record.hash, start) : null;
      const end = record.size ? Math.min(start + chunkBytes, record.size) - 1 : start + chunkBytes - 1;
      const r = await fetch(url, { headers: { Range: `bytes=${start}-${end}` }, cache: 'no-store' });
      const etag = r.headers.get('etag');
      if (r.status === 206) {
        const total = rangeTotal(r);
        const changed = (record.size && total !== record.size) || (record.etag && etag && etag !== record.etag);
        if (changed || rangeStart(r) !== start) {
          // The file on the server is not the one partly on disk: restart.
          await r.body?.cancel();
          record = fresh();
          handle.truncate(0);
          continue;
        }
        record.size = total;
        record.etag = etag;
        const at = await writeBody(handle, r.body, start, hash, (b) => onProgress(b, total));
        if (at !== end + 1) throw new Error(`GET ${url} bytes ${start}-${end}: got ${at - start} bytes`);
        record.have = at;
      } else if (r.status === 200) {
        // No Range support: the whole file, from byte 0.
        const total = Number(r.headers.get('content-length')) || null;
        handle.truncate(0);
        const whole = sha256 ? new Sha256() : null;
        const at = await writeBody(handle, r.body, 0, whole, (b) => onProgress(b, total));
        if (whole) hash = whole;
        if (total && at !== total) throw new Error(`GET ${url}: got ${at} of ${total} bytes`);
        record.size = at;
        record.etag = etag;
        record.have = at;
      } else if (r.status === 416 && record.size && start >= record.size) {
        record.have = record.size;
      } else {
        throw new Error(`GET ${url} (bytes ${start}-${end}): HTTP ${r.status}`);
      }
      handle.flush();
      record.complete = record.have === record.size;
      record.hash = hash && !record.complete ? hash.state() : null;
      if (record.complete && hash) record.digest = hash.hex();
      await writeRecord(dir, name, record);
    }

    if (handle.getSize() !== record.size) throw new Error(`${name}: ${handle.getSize()} bytes on disk, expected ${record.size}`);
    const magic = new Uint8Array(4);
    handle.read(magic, { at: 0 });
    if (String.fromCharCode(...magic) !== 'GGUF') {
      await dropFile(dir, name, handle);
      throw new Error(`${name}: not a GGUF file`);
    }
    if (sha256 && record.digest !== sha256.toLowerCase()) {
      await dropFile(dir, name, handle);
      throw new Error(`${name}: SHA-256 ${record.digest}, expected ${sha256}`);
    }
    record.verified = true;
    await writeRecord(dir, name, record);
    return { handle, size: record.size };
  } catch (err) {
    try { handle.close(); } catch (_) {}
    throw err;
  }
}

async function dropFile(dir, name, handle) {
  try { handle.close(); } catch (_) {}
  await dir.removeEntry(name).catch(() => {});
  await dir.removeEntry(`${name}.json`).catch(() => {});
}

const K = new Uint32Array([
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
]);

// Incremental SHA-256 (FIPS 180-4), for files too large for one
// crypto.subtle.digest call.
class Sha256 {
  constructor(h = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19], length = 0) {
    this.h = new Int32Array(h);
    this.w = new Int32Array(64);
    this.tail = new Uint8Array(64);
    this.tailLen = 0;
    this.length = length;
  }

  // The state after a whole number of 64-byte blocks, as plain numbers.
  state() {
    if (this.tailLen) throw new Error('Sha256.state() mid-block');
    return Array.from(this.h);
  }

  // A hash that continues from `state()` taken after `length` bytes (a
  // fresh one at byte 0).
  static resume(state, length) {
    if (length === 0) return new Sha256();
    if (!state) throw new Error('no saved hash state to resume from');
    return new Sha256(state, length);
  }

  update(bytes) {
    this.length += bytes.length;
    let i = 0;
    if (this.tailLen) {
      const n = Math.min(64 - this.tailLen, bytes.length);
      this.tail.set(bytes.subarray(0, n), this.tailLen);
      this.tailLen += n;
      i = n;
      if (this.tailLen < 64) return;
      this.block(this.tail, 0);
      this.tailLen = 0;
    }
    for (; i + 64 <= bytes.length; i += 64) this.block(bytes, i);
    if (i < bytes.length) {
      this.tail.set(bytes.subarray(i));
      this.tailLen = bytes.length - i;
    }
  }

  block(p, o) {
    const w = this.w;
    for (let t = 0; t < 16; t++, o += 4) w[t] = (p[o] << 24) | (p[o + 1] << 16) | (p[o + 2] << 8) | p[o + 3];
    for (let t = 16; t < 64; t++) {
      const a = w[t - 15], b = w[t - 2];
      const s0 = ((a >>> 7) | (a << 25)) ^ ((a >>> 18) | (a << 14)) ^ (a >>> 3);
      const s1 = ((b >>> 17) | (b << 15)) ^ ((b >>> 19) | (b << 13)) ^ (b >>> 10);
      w[t] = (w[t - 16] + s0 + w[t - 7] + s1) | 0;
    }
    const h = this.h;
    let a = h[0], b = h[1], c = h[2], d = h[3], e = h[4], f = h[5], g = h[6], hh = h[7];
    for (let t = 0; t < 64; t++) {
      const S1 = ((e >>> 6) | (e << 26)) ^ ((e >>> 11) | (e << 21)) ^ ((e >>> 25) | (e << 7));
      const t1 = (hh + S1 + ((e & f) ^ (~e & g)) + K[t] + w[t]) | 0;
      const S0 = ((a >>> 2) | (a << 30)) ^ ((a >>> 13) | (a << 19)) ^ ((a >>> 22) | (a << 10));
      const t2 = (S0 + ((a & b) ^ (a & c) ^ (b & c))) | 0;
      hh = g; g = f; f = e; e = (d + t1) | 0; d = c; c = b; b = a; a = (t1 + t2) | 0;
    }
    h[0] = (h[0] + a) | 0; h[1] = (h[1] + b) | 0; h[2] = (h[2] + c) | 0; h[3] = (h[3] + d) | 0;
    h[4] = (h[4] + e) | 0; h[5] = (h[5] + f) | 0; h[6] = (h[6] + g) | 0; h[7] = (h[7] + hh) | 0;
  }

  hex() {
    const bits = this.length * 8;
    const pad = new Uint8Array(((this.tailLen + 9 + 63) & ~63) - this.tailLen);
    pad[0] = 0x80;
    const dv = new DataView(pad.buffer);
    dv.setUint32(pad.length - 8, Math.floor(bits / 2 ** 32));
    dv.setUint32(pad.length - 4, bits >>> 0);
    const len = this.length;
    this.update(pad);
    this.length = len;
    return Array.from(this.h, (x) => (x >>> 0).toString(16).padStart(8, '0')).join('');
  }
}
