// Test-process envelopes are operational controls, never language acceptance rules.
// Kill the complete process group on timeout: killing only a compiler can strand
// its Node child, which happened in the original outside-review experiments.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
const active = new Set();
function stop(child) {
  if (!child.pid) return;
  try { process.kill(process.platform === 'win32' ? child.pid : -child.pid, 'SIGKILL'); }
  catch (error) { if (error.code !== 'ESRCH') throw error; }
}
for (const [signal, status] of [['SIGINT', 130], ['SIGTERM', 143]]) {
  process.on(signal, () => { for (const child of active) stop(child); process.exit(status); });
}
export function run(command, args, options = {}) {
  const { timeout = Number(process.env.LKJ_TEST_TIMEOUT_MS || 30000),
    maxBuffer = 32 * 1024 * 1024, encoding = 'utf8', sampleMemory = false, ...spawnOptions } = options;
  return new Promise(resolve => {
    const stdout = [], stderr = [];
    let size = 0, error;
    const child = spawn(command, args, { ...spawnOptions, detached: process.platform !== 'win32',
      stdio: ['ignore', 'pipe', 'pipe'] });
    active.add(child);
    let peakTreeRss = 0;
    function sample() {
      if (!child.pid) return;
      let bytes = 0; const pending = [child.pid];
      while (pending.length) {
        const pid = pending.pop();
        try {
          const status = fs.readFileSync(`/proc/${pid}/status`, 'utf8');
          bytes += Number(status.match(/^VmRSS:\s+(\d+)/m)?.[1] || 0) * 1024;
          const children = fs.readFileSync(`/proc/${pid}/task/${pid}/children`, 'utf8').trim();
          if (children) pending.push(...children.split(/\s+/).map(Number));
        } catch (error) { if (error.code !== 'ENOENT' && error.code !== 'ESRCH') throw error; }
      }
      peakTreeRss = Math.max(peakTreeRss, bytes);
    }
    const memoryTimer = sampleMemory ? setInterval(sample, 10) : undefined;
    const timer = setTimeout(() => {
      error = Object.assign(new Error(`process exceeded test envelope of ${timeout} ms`), {code: 'ETIMEDOUT'});
      stop(child);
    }, timeout);
    for (const [stream, chunks] of [[child.stdout, stdout], [child.stderr, stderr]]) {
      stream.on('data', chunk => {
        size += chunk.length;
        if (size > maxBuffer) {
          error = Object.assign(new Error('process exceeded test output envelope'), {code: 'ENOBUFS'});
          stop(child);
        } else chunks.push(chunk);
      });
    }
    child.on('error', problem => { error = problem; });
    child.on('close', (status, signal) => {
      clearTimeout(timer); if (memoryTimer) clearInterval(memoryTimer); active.delete(child);
      const convert = chunks => encoding ? Buffer.concat(chunks).toString(encoding) : Buffer.concat(chunks);
      resolve({status, signal, error, sampled_peak_tree_rss_bytes: sampleMemory ? peakTreeRss : undefined, stdout: convert(stdout), stderr: convert(stderr)});
    });
  });
}
