#!/usr/bin/env node
// Measure the source producer over a declared bundle, K input and native backend.
// Stages are explicit so artifact size and controls can be inspected before runs.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { run } from './process.mjs';
import { packFiles } from './source-bundle.mjs';

const usage = [
  'usage: node tools/frontend-selfhost.mjs prepare SNAPSHOT NATIVE_KC RUNTIME_K OUT',
  '       node tools/frontend-selfhost.mjs emit OUT',
  '       node tools/frontend-selfhost.mjs run OUT',
  'SNAPSHOT must contain one accepted source target, prepared with frontend.mjs snapshot --source.',
  'prepare validates and packs inputs without executing compilers; emit builds frontend1;',
  'run checks its own K, builds frontend2 and checks the K/ELF fixed point.',
  'LKJ_FRONTEND_SELFHOST_MEMORY_BYTES defaults to 8589934592; unlimited inherits Linux controls.',
  'LKJ_FRONTEND_SELFHOST_STACK_BYTES is optional and independent; default inherits the stack limit.',
  'LKJ_FRONTEND_SELFHOST_TIMEOUT_MS defaults to 600000 per command; 0 disables the deadline.',
  'LKJ_FRONTEND_SELFHOST_MAX_OUTPUT_BYTES defaults to 268435456.',
].join('\n');
const argv = process.argv.slice(2);
if (argv.includes('--help')) { console.log(usage); process.exit(0); }
const stage = argv.shift();
if (!['prepare', 'emit', 'run'].includes(stage) || argv.length !== (stage === 'prepare' ? 4 : 1)) throw Error(usage);
const out = path.resolve(argv[stage === 'prepare' ? 3 : 0]);
const summaryPath = path.join(out, 'summary.json');
const harness = fileURLToPath(import.meta.url);
const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
function record(file) {
  const bytes = fs.readFileSync(file);
  return { path: path.resolve(file), bytes: bytes.length, sha256: digest(bytes) };
}
function unchanged(expected) {
  const actual = record(expected.path);
  if (actual.sha256 !== expected.sha256 || actual.bytes !== expected.bytes) throw Error('declared input changed: ' + expected.path);
}
function numberSetting(name, fallback, minimum) {
  const value = Number(process.env[name] ?? fallback);
  if (!Number.isSafeInteger(value) || value < minimum) throw Error(name + ' must be an integer >= ' + minimum);
  return value;
}
function byteSetting(name, fallback) {
  const value = process.env[name] ?? fallback;
  if (value !== undefined && value !== 'unlimited' && !/^[1-9][0-9]*$/.test(value)) throw Error(name + ' must be positive bytes or unlimited');
  return value;
}
const controls = {
  memory_bytes: byteSetting('LKJ_FRONTEND_SELFHOST_MEMORY_BYTES', '8589934592'),
  stack_bytes: byteSetting('LKJ_FRONTEND_SELFHOST_STACK_BYTES', undefined) || 'inherited',
  timeout_ms: numberSetting('LKJ_FRONTEND_SELFHOST_TIMEOUT_MS', 600000, 0),
  max_output_bytes: numberSetting('LKJ_FRONTEND_SELFHOST_MAX_OUTPUT_BYTES', 268435456, 1),
};
let summary;
function save() { fs.writeFileSync(summaryPath, JSON.stringify(summary, null, 2) + '\n'); }
function sourceUnchanged() {
  for (const file of summary.declared_source_files) unchanged(file);
  for (const file of summary.absent_source_files) if (fs.existsSync(file)) throw Error('previously absent module appeared: ' + file);
}
function copyInput(source, name) {
  const target = path.join(out, name);
  fs.copyFileSync(source, target, fs.constants.COPYFILE_EXCL);
  return record(target);
}
function timing(file) {
  if (!fs.existsSync(file)) return null;
  const line = fs.readFileSync(file, 'utf8').trim().split('\n').findLast(value => /^\d+(\.\d+)?\t\d+\t\d+$/.test(value));
  if (!line) return null;
  const [seconds, rss, exit] = line.split('\t').map(Number);
  return { elapsed_seconds: seconds, max_rss_bytes: rss * 1024, exit_status: exit };
}
function failureKind(result) {
  if (result.error?.code === 'ETIMEDOUT') return 'timeout-envelope';
  if (result.error?.code === 'ENOBUFS') return 'output-envelope';
  if (result.error) return 'host-error';
  if (result.signal || /^Command terminated by signal \d+/m.test(result.stderr.toString('utf8'))) return 'signal';
  if (result.status === null) return 'missing-exit-status';
  if (result.status !== 0) {
    if (/\bR\d{4}\b/.test(result.stderr.toString('utf8'))) return 'runtime-trap';
    if (/\bE_[A-Z_]+\b/.test(result.stderr.toString('utf8'))) return 'compiler-rejection';
    return 'command-failure';
  }
  return null;
}
async function measure(name, executable, args, { cwd = out, stdout = name + '.stdout', artifact } = {}) {
  const timeFile = path.join(out, name + '.time.tsv');
  const stdoutFile = path.join(out, stdout), stderrFile = path.join(out, name + '.stderr');
  for (const file of [timeFile, stdoutFile, stderrFile, ...(artifact ? [path.join(out, artifact)] : [])]) {
    if (fs.existsSync(file)) throw Error('refusing to overwrite observation: ' + file);
  }
  const stack = controls.stack_bytes === 'inherited' ? [] : ['prlimit', '--stack=' + controls.stack_bytes, '--'];
  console.log('frontend-selfhost: ' + name);
  const start = performance.now();
  const result = await run('/usr/bin/time', ['-f', '%e\t%M\t%x', '-o', timeFile, '--', ...stack, executable, ...args], {
    cwd, encoding: null, memoryBytes: controls.memory_bytes, timeout: controls.timeout_ms, maxBuffer: controls.max_output_bytes,
  });
  fs.writeFileSync(stdoutFile, result.stdout);
  fs.writeFileSync(stderrFile, result.stderr);
  const observation = {
    name, executable, args, cwd, status: result.status, signal: result.signal,
    error: result.error ? { code: result.error.code || null, message: result.error.message } : null,
    failure_kind: failureKind(result), wall_ms: performance.now() - start, timing: timing(timeFile),
    stdout: record(stdoutFile), stderr: record(stderrFile),
  };
  if (fs.existsSync(timeFile)) observation.timing_file = record(timeFile);
  if (artifact && fs.existsSync(path.join(out, artifact))) observation.artifact = record(path.join(out, artifact));
  summary.commands.push(observation); save();
  if (observation.failure_kind) throw Error(name + ': ' + observation.failure_kind + ' (see ' + stderrFile + ')');
  if (!observation.timing) throw Error(name + ': no completed /usr/bin/time measurement');
  return observation;
}
function elf(name) {
  const file = path.join(out, name), bytes = fs.readFileSync(file);
  if (bytes.length < 4 || !bytes.subarray(0, 4).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]))) throw Error(name + ': output is not ELF');
  fs.chmodSync(file, 0o755);
  return record(file);
}
function compare(name, left, right) {
  const a = fs.readFileSync(left), b = fs.readFileSync(right), identical = a.equals(b);
  const observation = { name, left: record(left), right: record(right), identical };
  if (!identical) {
    let at = 0;
    while (at < Math.min(a.length, b.length) && a[at] === b[at]) at++;
    observation.first_difference = { byte_offset: at, expected_bytes: a.length, actual_bytes: b.length };
  }
  summary.comparisons.push(observation); save();
  if (!identical) throw Error(name + ': bytes differ at ' + observation.first_difference.byte_offset);
}

if (stage === 'prepare') {
  if (fs.existsSync(summaryPath)) throw Error('refusing to overwrite an existing observation');
  const snapshot = path.resolve(argv[0]);
  const baseline = JSON.parse(fs.readFileSync(path.join(snapshot, 'summary.json'), 'utf8'));
  const observations = JSON.parse(fs.readFileSync(path.join(snapshot, baseline.observations || 'observations.json'), 'utf8'));
  if (baseline.mode !== 'snapshot' || baseline.failures.length || observations.length !== 1 || observations[0].result?.status !== 0) throw Error('prepare requires one accepted source snapshot');
  const input = observations[0], sourceRoot = baseline.source_root;
  unchanged(baseline.compiler);
  const declared = [
    ...input.inputs.filter(file => !file.missing).map(file => ({ ...file, logical_path: file.path })),
    ...baseline.library,
  ];
  const declaredFiles = declared.map(file => ({ ...file, path: path.join(sourceRoot, file.path) }));
  for (const file of declaredFiles) unchanged(file);
  const absent = input.inputs.filter(file => file.missing).map(file => path.join(sourceRoot, file.path));
  for (const file of absent) if (fs.existsSync(file)) throw Error('previously absent module appeared: ' + file);
  const packed = await packFiles({ root: input.source, library: '$std/lib.rs', files: declared.map(file => ({ path: file.logical_path, file: path.join(sourceRoot, file.path) })) });
  for (let index = 0; index < declared.length; index++) if (packed.manifest.files[index].sha256 !== declared[index].sha256) throw Error('input changed while packing ' + declared[index].path);
  const expectedK = { path: path.join(snapshot, input.result.stdout), bytes: input.result.stdout_bytes, sha256: input.result.stdout_sha256 };
  unchanged(expectedK);
  fs.mkdirSync(out, { recursive: true });
  summary = {
    stage: 'source producer self-host fixed point', status: 'prepared', started_at: new Date().toISOString(),
    source_root: sourceRoot, source: input.source,
    snapshot: { directory: snapshot, summary: record(path.join(snapshot, 'summary.json')), observations: record(path.join(snapshot, baseline.observations || 'observations.json')) },
    compilers: { b0: baseline.compiler, kc: record(argv[1]) },
    runtime_origin: record(argv[2]), declared_source_files: declaredFiles, absent_source_files: absent,
    inputs: {}, artifacts: {}, commands: [], comparisons: [], runs: [], failures: [],
  };
  summary.inputs.program = copyInput(expectedK.path, 'frontend.k');
  summary.inputs.runtime = copyInput(argv[2], 'runtime.k');
  fs.writeFileSync(path.join(out, 'frontend.lkjsb'), packed.bytes, { flag: 'wx' });
  fs.writeFileSync(path.join(out, 'bundle.json'), JSON.stringify(packed.manifest, null, 2) + '\n', { flag: 'wx' });
  summary.inputs.bundle = record(path.join(out, 'frontend.lkjsb'));
  summary.inputs.bundle_manifest = record(path.join(out, 'bundle.json'));
  summary.runs.push({ stage, controls, harness: record(harness), encoder: record(path.join(path.dirname(harness), 'source-bundle.mjs')), finished_at: new Date().toISOString(), status: 'prepared' });
  save();
} else {
  summary = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));
  if (summary.stage !== 'source producer self-host fixed point' || summary.runs.some(run => run.stage === stage)) throw Error('stage already attempted or observation type differs; use a new output directory for a new attempt');
  if (summary.status !== (stage === 'emit' ? 'prepared' : 'emitted')) throw Error('previous stage is not complete');
  const attempt = { stage, controls, harness: record(harness), started_at: new Date().toISOString(), status: 'running' };
  summary.runs.push(attempt); save();
  const began = performance.now();
  try {
    for (const input of Object.values(summary.inputs)) unchanged(input);
    unchanged(summary.compilers.kc);
    const b0 = summary.compilers.b0.path, kc = summary.compilers.kc.path;
    if (stage === 'emit') {
      unchanged(summary.compilers.b0); sourceUnchanged();
      await measure('b0_source', b0, ['k', summary.source], { cwd: summary.source_root, stdout: 'b0-source.k' });
      compare('B0 source reproduces the declared K', summary.inputs.program.path, path.join(out, 'b0-source.k'));
      await measure('b0_runtime', b0, ['runtime-k'], { stdout: 'b0-runtime.k' });
      compare('B0 runtime matches declared native runtime', summary.inputs.runtime.path, path.join(out, 'b0-runtime.k'));
      await measure('b0_emit', b0, ['build', 'frontend.k', '--entry', 'main', '-o', 'frontend-b0.elf'], { artifact: 'frontend-b0.elf' });
      summary.artifacts.b0 = elf('frontend-b0.elf');
      await measure('kc_emit_frontend1', kc, ['frontend.k', '--runtime', 'runtime.k', '--entry', 'main'], { stdout: 'frontend1.elf' });
      summary.artifacts.frontend1 = elf('frontend1.elf');
      compare('B0 versus native K compiler frontend ELF', summary.artifacts.b0.path, summary.artifacts.frontend1.path);
      summary.status = 'emitted';
    } else {
      unchanged(summary.artifacts.frontend1);
      await measure('frontend1_source', summary.artifacts.frontend1.path, ['frontend.lkjsb'], { stdout: 'frontend1.k' });
      compare('frontend1 versus B0 own-source K', summary.inputs.program.path, path.join(out, 'frontend1.k'));
      await measure('kc_emit_frontend2', kc, ['frontend1.k', '--runtime', 'runtime.k', '--entry', 'main'], { stdout: 'frontend2.elf' });
      summary.artifacts.frontend2 = elf('frontend2.elf');
      compare('frontend1 versus frontend2 ELF', summary.artifacts.frontend1.path, summary.artifacts.frontend2.path);
      await measure('frontend2_source', summary.artifacts.frontend2.path, ['frontend.lkjsb'], { stdout: 'frontend2.k' });
      compare('frontend2 versus B0 own-source K', summary.inputs.program.path, path.join(out, 'frontend2.k'));
      summary.status = 'fixed point';
    }
    attempt.status = summary.status;
  } catch (error) {
    summary.failures.push({ stage, problem: String(error) });
    summary.status = 'failed'; attempt.status = 'failed'; process.exitCode = 1;
  } finally {
    attempt.elapsed_ms = performance.now() - began;
    attempt.finished_at = new Date().toISOString(); save();
  }
}
console.log(JSON.stringify({ status: summary.status, summary: summaryPath, inputs: summary.inputs, artifacts: summary.artifacts, failures: summary.failures }));
