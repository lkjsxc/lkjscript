import { readFile } from 'node:fs/promises';
import { writeSync } from 'node:fs';
import { describe, resultText, executeModule } from './engine.mjs';
import { isMainThread, parentPort, workerData } from 'node:worker_threads';
import { startWorker } from './worker.mjs';
import { dirname, join } from 'node:path';

const readText = path => readFile(path, 'utf8');

async function cgroupExtent(name, extent) {
  const groups = await readText('/proc/self/cgroup');
  const group = groups.split('\n').find(line => line.startsWith('0::'))?.slice(3);
  if (group === undefined) return extent;
  const root = '/sys/fs/cgroup';
  let directory = join(root, group);
  for (;;) {
    try {
      const text = (await readText(join(directory, name))).trim();
      if (/^[0-9]+$/.test(text)) extent = Math.min(extent, Number(text));
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
    }
    if (directory === root) return extent;
    const parent = dirname(directory);
    if (parent === directory) return extent;
    directory = parent;
  }
}

async function stackReservation() {
  const memory = await readText('/proc/meminfo');
  const kib = name => {
    const match = memory.match(new RegExp('^' + name + ':\\s+(\\d+)', 'm'));
    if (!match) throw new Error('cannot read ' + name + ' memory extent');
    return Number(match[1]) * 1024;
  };
  let bytes = await cgroupExtent('memory.max', kib('MemTotal'))
    + await cgroupExtent('memory.swap.max', kib('SwapTotal'));
  const limits = await readText('/proc/self/limits');
  const limit = limits.match(/^Max address space\s+(\d+)/m);
  if (limit) {
    const status = await readText('/proc/self/status');
    const mapped = status.match(/^VmSize:\s+(\d+)/m);
    if (!mapped) throw new Error('cannot read mapped address-space extent');
    bytes = Math.min(bytes, Math.max(0, Number(limit[1]) - Number(mapped[1]) * 1024) / 2);
  }
  if (!Number.isSafeInteger(Math.floor(bytes)) || bytes <= 0) {
    throw new Error('no representable address space available for worker stack');
  }
  // This reserves address space. Pages commit only as calls touch them; each
  // simultaneous worker has its own reservation and shares physical memory.
  return Math.floor(bytes);
}

function report(path, problem) {
  writeSync(2, path + ':' + problem.line + ': ' + problem.code + ' ' + problem.message + '\n');
  process.exitCode = 1;
}

function printResult(result, kind) { writeSync(1, resultText(result, kind)); }

if (isMainThread) {
  const [path, entry = 'main', resultKind = 'i64', ...args] = process.argv.slice(2);
  let metrics;
  try {
    if (!path) throw new Error('usage: node bridge/run.mjs module.wasm [entry] [result-kind] [integer-arguments...]');
    const stack = await stackReservation();
    const started = performance.now();
    let startup;
    const {worker, stackBytes} = startWorker(new URL(import.meta.url), {
      workerData: { path, entry, args },
    }, stack);
    const message = await new Promise((resolve, reject) => {
      let result;
      let failure;
      worker.on('message', message => {
        if (message.ready) startup = performance.now() - started;
        else result = message;
      });
      worker.on('error', error => { failure = error; });
      worker.on('exit', code => {
        // A posted result is not a pass until the worker finishes normally.
        if (failure) reject(failure);
        else if (!result) reject(new Error('worker exited without a result (status ' + code + ')'));
        else if (code !== 0 && result.ok) reject(new Error('worker failed after returning a result (status ' + code + ')'));
        else resolve(result);
      });
    });
    metrics = {
      route: 'node', ...message.metrics, worker_startup_ms: startup,
      stack_reservation_bytes: stackBytes, requested_stack_bytes: stack, peak_rss_bytes: process.resourceUsage().maxRSS * 1024,
    };
    if (message.ok) printResult(message.result, resultKind);
    else report(path, message.problem);
  } catch (problem) {
    report(path, describe(problem));
  } finally {
    if (process.env.LKJSCRIPT_TIMINGS === '1' && metrics) {
      writeSync(2, 'LKJ_TIMING ' + JSON.stringify(metrics) + '\n');
    }
  }
} else {
  parentPort.postMessage({ ready: true });
  const result = await executeModule(workerData);
  parentPort.postMessage(result);
  if (!result.ok) process.exitCode = 1;
}
