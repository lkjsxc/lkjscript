// Test-process envelopes are operational controls, never language acceptance rules.
// Kill the complete process group on timeout: killing only a compiler can strand
// its Node child, which happened in the original outside-review experiments.
// Batch callers disable the total deadline and require stdout progress instead;
// their protocol emits only completed cases or prepared modules, never program output.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
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
  const { timeout = Number(process.env.LKJSCRIPT_TEST_TIMEOUT_MS || 30000),
    maxBuffer = 32 * 1024 * 1024, encoding = 'utf8', sampleMemory = false, idleTimeout = 0, input, captureControl = false,
    memoryBytes = process.env.LKJSCRIPT_TEST_MEMORY_BYTES || (path.basename(command) === 'rustc' ? '8589934592' : '2147483648'), ...spawnOptions } = options;
  if (![timeout, idleTimeout].every(value => Number.isFinite(value) && value >= 0)) throw new Error('invalid process timeout');
  return new Promise(resolve => {
    const stdout = [], stderr = [], control = [];
    let size = 0, error;
    // Programs under test, including Rust references, inherit an explicit Linux
    // address-space envelope. Capacity probes opt into a larger value or
    // LKJSCRIPT_TEST_MEMORY_BYTES=unlimited. Node only orchestrates tests and reserves
    // a large V8 address cage; generated programs never execute inside it.
    const budget = String(memoryBytes);
    if (budget !== 'unlimited' && (!/^[1-9][0-9]*$/.test(budget) || BigInt(budget) > 18446744073709551615n)) {
      throw new Error('LKJSCRIPT_TEST_MEMORY_BYTES must be positive bytes or unlimited');
    }
    const node = ['node', 'nodejs'].includes(path.basename(command));
    const executable = process.platform === 'linux' ? 'prlimit' : command;
    const arguments_ = process.platform === 'linux'
      ? ['--core=0', ...(node || budget === 'unlimited' ? [] : ['--as=' + budget]), '--', command, ...args] : args;
    const child = spawn(executable, arguments_, { ...spawnOptions, detached: process.platform !== 'win32',
      stdio: [input === undefined ? 'ignore' : 'pipe', 'pipe', 'pipe', ...(captureControl ? ['pipe'] : [])] });
    if (child.stdin) {
      child.stdin.on('error', problem => { if (problem.code !== 'EPIPE') error = problem; });
      child.stdin.end(input);
    }
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
    function expire(message) {
      error = Object.assign(new Error(message), {code: 'ETIMEDOUT'});
      stop(child);
    }
    const timer = timeout ? setTimeout(() => expire(`process exceeded test envelope of ${timeout} ms`), timeout) : undefined;
    let idleTimer;
    function progress() {
      if (!idleTimeout) return;
      clearTimeout(idleTimer);
      idleTimer = setTimeout(() => expire(`process produced no result for ${idleTimeout} ms`), idleTimeout);
    }
    progress();
    for (const [stream, chunks] of [[child.stdout, stdout], [child.stderr, stderr], ...(captureControl ? [[child.stdio[3], control]] : [])]) {
      stream.on('data', chunk => {
        if (stream === child.stdout) progress();
        size += chunk.length;
        if (size > maxBuffer) {
          error = Object.assign(new Error('process exceeded test output envelope'), {code: 'ENOBUFS'});
          stop(child);
        } else chunks.push(chunk);
      });
    }
    child.on('error', problem => { error = problem; });
    child.on('close', (status, signal) => {
      clearTimeout(timer); clearTimeout(idleTimer); if (memoryTimer) clearInterval(memoryTimer); active.delete(child);
      const convert = chunks => encoding ? Buffer.concat(chunks).toString(encoding) : Buffer.concat(chunks);
      resolve({status, signal, error, sampled_peak_tree_rss_bytes: sampleMemory ? peakTreeRss : undefined, stdout: convert(stdout), stderr: convert(stderr), control: Buffer.concat(control)});
    });
  });
}
