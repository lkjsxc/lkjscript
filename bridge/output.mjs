// The entire range is validated before any byte is read. Each chunk is copied
// before the synchronous write and no guest pointer or byte buffer is retained.
import { writeSync } from 'node:fs';
export function writeBytes(memory, pointer, length, line, write = writeSync) {
  try {
    if (typeof pointer !== 'bigint' || typeof length !== 'bigint' || pointer < 0n || length < 0n) {
      throw new Error('invalid byte range');
    }
    const extent = BigInt(memory.buffer.byteLength);
    if (pointer > extent || length > extent - pointer) throw new Error('byte range exceeds memory');
    let at = Number(pointer), remaining = Number(length);
    while (remaining !== 0) {
      // This is a streaming chunk size, not a limit on accepted output length.
      const count = Math.min(remaining, 64 * 1024);
      const bytes = Buffer.from(new Uint8Array(memory.buffer, at, count));
      let written = 0;
      while (written < bytes.length) {
        const n = write(1, bytes, written, bytes.length - written);
        if (!Number.isInteger(n) || n <= 0 || n > bytes.length - written) throw new Error('stdout made no valid progress');
        written += n;
      }
      at += count; remaining -= count;
    }
    return 0n;
  } catch (cause) {
    const problem = new Error('host output failure: ' + (cause.message || String(cause)));
    problem.code = 'R0007'; problem.line = Number(line);
    throw problem;
  }
}
