// Optional WebAssembly host. Native owns its stack and heap; this route uses
// Node's ordinary engine resources without a reservation worker or retries.
import { writeSync } from 'node:fs';
import { describe, resultText, executeModule } from './engine.mjs';

const [path, entry = 'main', kind = 'i64', ...args] = process.argv.slice(2);
let metrics;
try {
  if (!path) throw new Error('usage: node bridge/run.mjs module.wasm [entry] [result-kind] [integer-arguments...]');
  const result = await executeModule({path, entry, args});
  metrics = {route: 'node', ...result.metrics, peak_rss_bytes: process.resourceUsage().maxRSS * 1024};
  if (result.ok) writeSync(1, resultText(result.result, kind));
  else {
    writeSync(2, path + ':' + result.problem.line + ': ' + result.problem.code + ' ' + result.problem.message + '\n');
    process.exitCode = 1;
  }
} catch (error) {
  const problem = describe(error);
  writeSync(2, path + ':' + problem.line + ': ' + problem.code + ' ' + problem.message + '\n');
  process.exitCode = 1;
} finally {
  if (process.env.LKJSCRIPT_TIMINGS === '1' && metrics)
    writeSync(2, 'LKJ_TIMING ' + JSON.stringify(metrics) + '\n');
}
