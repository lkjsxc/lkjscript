import { readFile } from 'node:fs/promises';

const [path, entry = 'main', resultKind = 'i64', ...args] = process.argv.slice(2);
const messages = {
  1: 'integer overflow', 2: 'division by zero', 3: 'index out of bounds',
  4: 'explicit trap', 5: 'assertion failed', 6: 'allocation failed',
};
try {
  if (!path) throw new Error('usage: node bridge/run.mjs module.wasm [entry] [result-kind] [integer-arguments...]');
  const bytes = await readFile(path);
  const module = await WebAssembly.compile(bytes);
  const metadata = WebAssembly.Module.customSections(module, 'lkjscript.traps');
  const userMessages = metadata.length ? JSON.parse(Buffer.from(metadata[0]).toString('utf8')) : [];
  const instance = await WebAssembly.instantiate(module, {
    lkjscript: {
      trap(code, line) {
        const problem = new Error(messages[Number(code)] ?? userMessages[Number(code) - 16] ?? 'trap');
        problem.code = 'R' + code.toString().padStart(4, '0');
        problem.line = line;
        throw problem;
      },
    },
  });
  const fn = instance.exports['fn:' + entry];
  if (typeof fn !== 'function') throw new Error('entry function not found: ' + entry);
  const result = fn(...args.map(BigInt));
  if (resultKind === 'f64') {
    const data = new DataView(new ArrayBuffer(8));
    data.setBigUint64(0, BigInt.asUintN(64, result), true);
    process.stdout.write(String(data.getFloat64(0, true)) + '\n');
  } else if (resultKind === 'bool') {
    process.stdout.write((result === 0n ? 'false' : 'true') + '\n');
  } else if (resultKind !== 'unit' && resultKind !== '--unit') {
    process.stdout.write((resultKind === 'u64' || resultKind === 'usize' ? BigInt.asUintN(64, result) : result).toString() + '\n');
  }
} catch (problem) {
  const message = problem instanceof Error ? problem.message : String(problem);
  process.stderr.write(path + ':' + (problem.line ?? 0) + ': ' + (problem.code ?? 'E_ENGINE') + ' ' + message + '\n');
  process.exitCode = 1;
}

