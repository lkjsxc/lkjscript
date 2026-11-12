// Adapt virtual stack reservation before any worker executes guest code. A
// failure after construction is never retried: the guest may have had effects.
import { Worker } from 'node:worker_threads';
export function startWorker(url, options, requestedBytes, create = (url, options) => new Worker(url, options)) {
  let bytes = Math.floor(requestedBytes);
  for (;;) {
    try {
      const worker = create(url, {...options, resourceLimits: {...options.resourceLimits, stackSizeMb: bytes / (1024 * 1024)}});
      return {worker, stackBytes: bytes};
    } catch (error) {
      const resource = ['EAGAIN','ENOMEM'].includes(error.code)
        || (error.code === 'ERR_WORKER_INIT_FAILED' && /\b(EAGAIN|ENOMEM)\b/.test(error.message));
      if (!resource || bytes <= 1) throw error;
      bytes = Math.floor(bytes / 2);
    }
  }
}
