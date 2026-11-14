#!/usr/bin/env node
// Preserve and compare the source producer's exact IR over identical declared
// inputs. Resource envelopes belong to this experiment, not source acceptance.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { run } from './process.mjs';
import { packFiles } from './source-bundle.mjs';

const usage = 'usage: node tools/frontend.mjs snapshot [--source ROOT ...] B0 OUT | compare [--world] SNAPSHOT CANDIDATE OUT';
const argv = process.argv.slice(2);
if (argv.includes('--help')) { console.log(usage); process.exit(0); }
const mode = argv.shift(), args = [], sources = [];
let world = false;
for (let index = 0; index < argv.length; index++) {
  const value = argv[index];
  if (value === '--world' && mode === 'compare' && !world) world = true;
  else if (value === '--source' && mode === 'snapshot' && index + 1 < argv.length) sources.push(argv[++index]);
  else if (value.startsWith('--')) throw Error(usage);
  else args.push(value);
}
if (!['snapshot', 'compare'].includes(mode) ||
    (mode === 'snapshot' && args.length !== 2) ||
    (mode === 'compare' && args.length !== 3)) throw Error(usage);
const sourceRoot = path.resolve(process.env.LKJSCRIPT_FRONTEND_SOURCE_ROOT || '.');
const snapshotRoot = mode === 'compare' ? path.resolve(args[0]) : undefined;
const binary = path.resolve(args[mode === 'snapshot' ? 0 : 1]);
const out = path.resolve(args[mode === 'snapshot' ? 1 : 2]);
const jobs = Number(process.env.LKJSCRIPT_FRONTEND_JOBS || 4);
const timeout = Number(process.env.LKJSCRIPT_FRONTEND_TIMEOUT_MS || 60000);
const maxBuffer = Number(process.env.LKJSCRIPT_FRONTEND_MAX_OUTPUT_BYTES || 134217728);
const memoryBytes = process.env.LKJSCRIPT_FRONTEND_MEMORY_BYTES || '8589934592';
const stackBytes = process.env.LKJSCRIPT_FRONTEND_STACK_BYTES;
if (stackBytes !== undefined && stackBytes !== 'unlimited' && !/^[1-9][0-9]*$/.test(stackBytes)) throw Error('LKJSCRIPT_FRONTEND_STACK_BYTES must be positive bytes or unlimited');
if (!Number.isSafeInteger(jobs) || jobs < 1) throw Error('LKJSCRIPT_FRONTEND_JOBS must be positive');
if (!Number.isSafeInteger(timeout) || timeout < 1) throw Error('LKJSCRIPT_FRONTEND_TIMEOUT_MS must be positive');
if (!Number.isSafeInteger(maxBuffer) || maxBuffer < 1) throw Error('LKJSCRIPT_FRONTEND_MAX_OUTPUT_BYTES must be positive');
if (fs.existsSync(path.join(out, 'summary.json'))) throw Error('refusing to overwrite an existing observation');
fs.mkdirSync(path.join(out, 'records'), { recursive: true });
if (world) fs.mkdirSync(path.join(out, 'bundles'), { recursive: true });

function digest(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}
function fileRecord(root, relative) {
  const bytes = fs.readFileSync(path.join(root, relative));
  return { path: relative, bytes: bytes.length, sha256: digest(bytes) };
}
function binaryRecord(file) {
  const bytes = fs.readFileSync(file);
  return { path: file, bytes: bytes.length, sha256: digest(bytes) };
}
function libraryInputs(root) {
  const provider = fs.readFileSync(path.join(root, 'bridge/library.rs'), 'utf8');
  const names = [...provider.matchAll(/include_str!\("\.\.\/(library\/[^"]+)"\)/g)].map(match => match[1]);
  if (!names.length) throw Error('source library provisioning has no recorded inputs');
  return names.map(name => ({ ...fileRecord(root, name), logical_path: '$std/' + path.basename(name) }));
}
function inputChanges(expected, root) {
  const changes = [];
  for (const before of expected) {
    try {
      if (before.missing) {
        if (fs.existsSync(path.join(root, before.path))) changes.push({ path: before.path, error: 'previously absent module input appeared' });
        continue;
      }
      const after = fileRecord(root, before.path);
      if (before.sha256 !== after.sha256) changes.push({ path: before.path, expected: before.sha256, actual: after.sha256 });
    } catch (error) {
      changes.push({ path: before.path, error: String(error) });
    }
  }
  return changes;
}
function sourceInputs(root, source) {
  // The present corpus has single-file cases. Compiler roots additionally use
  // outlined modules. This inventory follows existing on-disk module candidates;
  // the compiler remains authoritative for syntax, cfg, ambiguity and errors.
  const seen = new Set(), missing = new Set(), pending = [{ file: source, isRoot: true }];
  while (pending.length) {
    const { file, isRoot } = pending.pop();
    if (seen.has(file)) continue;
    seen.add(file);
    const text = fs.readFileSync(path.join(root, file), 'utf8');
    const stem = path.basename(file, '.rs');
    const directory = isRoot || stem === 'mod' ? path.dirname(file) : path.join(path.dirname(file), stem);
    for (const match of text.matchAll(/^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z_0-9]*)\s*;/gm)) {
      for (const candidate of [path.join(directory, match[1] + '.rs'), path.join(directory, match[1], 'mod.rs')]) {
        if (fs.existsSync(path.join(root, candidate))) pending.push({ file: candidate, isRoot: false });
        else missing.add(candidate);
      }
    }
  }
  return [
    ...[...seen].map(name => fileRecord(root, name)),
    ...[...missing].filter(name => !seen.has(name)).map(name => ({ path: name, missing: true })),
  ].sort((a, b) => a.path.localeCompare(b.path));
}
function resultRecord(result) {
  const diagnostic = result.stderr.toString('utf8').match(/\bE_[A-Z_]+\b/)?.[0] || null;
  return {
    status: result.status,
    signal: result.signal,
    error: result.error ? { code: result.error.code || null, message: result.error.message } : null,
    diagnostic,
    stdout_bytes: result.stdout.length,
    stdout_sha256: digest(result.stdout),
    stderr_bytes: result.stderr.length,
    stderr_sha256: digest(result.stderr),
  };
}
function hostFailure(result, record) {
  if (result.error) return result.error.message;
  if (result.signal) return 'compiler terminated by signal ' + result.signal;
  if (result.status === null) return 'compiler returned no exit status';
  if (['E_HOST', 'E_IO', 'E_USAGE', ...(world ? ['E_BUNDLE'] : [])].includes(record.diagnostic)) return result.stderr.toString('utf8');
  if (result.status !== 0 && !record.diagnostic) return 'compiler failed without a diagnostic (status ' + result.status + ')';
  if (result.status !== 0 && result.stdout.length) return 'rejected source emitted partial IR output';
  if (result.status === 0 && record.diagnostic) return 'successful compiler status included a diagnostic';
  return null;
}
function saveStreams(index, result) {
  const prefix = 'records/' + String(index).padStart(5, '0');
  const stdout = prefix + (result.status === 0 ? '.lkjir' : '.stdout');
  const stderr = prefix + '.stderr';
  fs.writeFileSync(path.join(out, stdout), result.stdout);
  fs.writeFileSync(path.join(out, stderr), result.stderr);
  return { stdout, stderr };
}
function firstDifference(a, b) {
  let at = 0;
  while (at < Math.min(a.length, b.length) && a[at] === b[at]) at++;
  return { byte_offset: at, expected_bytes: a.length, actual_bytes: b.length };
}

let cases, reference, manifest;
if (mode === 'snapshot' && sources.length) {
  cases = sources.map(source => ({ id: 'source:' + source, source, kind: 'compiler', pending: false }));
  manifest = { kind: 'explicit-source-roots', sources };
} else if (mode === 'snapshot') {
  const manifestPath = process.env.LKJSCRIPT_FRONTEND_CASES || 'tests/conformance/cases.json';
  const all = JSON.parse(fs.readFileSync(path.join(sourceRoot, manifestPath), 'utf8'));
  cases = all.filter(c => c.file.endsWith('.rs')).map(c => ({
    id: c.id, source: 'tests/conformance/' + c.file, kind: c.kind, pending: Boolean(c.pending)
  }));
  cases.push({ id: 'compiler-backend-main', source: 'src/backend_main.rs', kind: 'compiler', pending: false });
  manifest = { ...fileRecord(sourceRoot, manifestPath), total_cases: all.length, direct_ir_cases: all.length - cases.length + 1 };
} else {
  reference = JSON.parse(fs.readFileSync(path.join(snapshotRoot, 'summary.json'), 'utf8'));
  if (reference.mode !== 'snapshot' || reference.failures.length) throw Error('baseline snapshot is incomplete or failed');
  cases = JSON.parse(fs.readFileSync(path.join(snapshotRoot, 'observations.json'), 'utf8'));
  manifest = reference.manifest;
}
const library = libraryInputs(sourceRoot);
const libraryChanges = reference ? inputChanges(reference.library, sourceRoot) : [];
if (reference) {
  const oldNames = reference.library.map(input => input.path);
  if (JSON.stringify(oldNames) !== JSON.stringify(library.map(input => input.path))) {
    libraryChanges.push({ path: 'bridge/library.rs', error: 'embedded library input sequence differs' });
  }
  for (const added of library.filter(input => !oldNames.includes(input.path))) {
    libraryChanges.push({ path: added.path, error: 'new embedded source-library input' });
  }
}
const summary = {
  stage: 'source producer exact-IR ' + mode,
  mode, route: world ? 'world-bundle' : 'host-cli', source_root: sourceRoot, compiler: binaryRecord(binary),
  manifest, library, library_changes: libraryChanges,
  controls: { jobs, timeout_ms: timeout, memory_bytes: memoryBytes, stack_bytes: stackBytes || 'inherited', max_output_bytes: maxBuffer },
  cases: cases.length, counts: {}, failures: [],
};
if (reference) {
  summary.baseline = { directory: snapshotRoot, compiler: reference.compiler };
}
const observations = new Array(cases.length), started = performance.now();
let next = 0;
function outcome(observation, name, problem) {
  observation.outcome = name;
  summary.counts[name] = (summary.counts[name] || 0) + 1;
  if (problem) {
    observation.failure = problem;
    summary.failures.push({ id: observation.id, outcome: name, problem });
  }
}
async function observe(index) {
  const input = cases[index];
  const observation = { id: input.id, source: input.source, kind: input.kind, pending: input.pending };
  observations[index] = observation;
  try {
    if (mode === 'snapshot') {
      observation.inputs = sourceInputs(sourceRoot, input.source);
    } else {
      observation.inputs = input.inputs;
      const changes = inputChanges(input.inputs, sourceRoot);
      if (libraryChanges.length || changes.length) {
        observation.input_changes = changes;
        outcome(observation, 'input-changed', libraryChanges.length ? 'embedded library inputs differ' : 'source inputs differ');
        return;
      }
    }
    let commandArgs = ['ir', input.source], cwd = sourceRoot;
    if (world) {
      const declared = [
        ...input.inputs.filter(file => !file.missing).map(file => ({ ...file, logical_path: file.path })),
        ...reference.library,
      ];
      const packed = await packFiles({
        root: input.source, library: '$std/lib.rs',
        files: declared.map(file => ({ path: file.logical_path, file: path.join(sourceRoot, file.path) })),
      });
      for (let index = 0; index < declared.length; index++) {
        if (packed.manifest.files[index].sha256 !== declared[index].sha256) throw Error('input changed while packing ' + declared[index].path);
      }
      const name = 'bundles/' + String(index).padStart(5, '0');
      const bundle = name + '.lkjsb', bundleManifest = name + '.json';
      fs.writeFileSync(path.join(out, bundle), packed.bytes);
      fs.writeFileSync(path.join(out, bundleManifest), JSON.stringify(packed.manifest, null, 2) + '\n');
      observation.bundle = { path: bundle, ...packed.manifest.bundle, manifest: bundleManifest };
      commandArgs = [bundle];
      cwd = out;
    }
    const command = stackBytes === undefined ? binary : 'prlimit';
    const commandArguments = stackBytes === undefined ? commandArgs : ['--stack=' + stackBytes, '--', binary, ...commandArgs];
    observation.invocation = { executable: command, args: commandArguments, cwd };
    const result = await run(command, commandArguments, {
      cwd, encoding: null, timeout, memoryBytes, maxBuffer
    });
    const observed = resultRecord(result);
    observation.result = { ...observed, ...saveStreams(index, result) };
    const failedHost = hostFailure(result, observed);
    if (failedHost) {
      outcome(observation, 'host-failure', failedHost);
      return;
    }
    if (mode === 'snapshot') {
      if (result.status === 0) outcome(observation, 'source-accepted');
      else if (input.pending) outcome(observation, 'pending');
      else if (input.kind === 'reject') outcome(observation, 'source-rejected');
      else outcome(observation, 'unexpected-rejection', result.stderr.toString('utf8'));
      return;
    }
    const expected = input.result;
    if ((result.status === 0) !== (expected.status === 0)) {
      outcome(observation, 'verdict-changed', 'expected status ' + expected.status + ', actual ' + result.status);
    } else if (result.status === 0) {
      const expectedBytes = fs.readFileSync(path.join(snapshotRoot, expected.stdout));
      if (expectedBytes.length !== expected.stdout_bytes || digest(expectedBytes) !== expected.stdout_sha256) throw Error('baseline IR stream no longer matches its recorded bytes/hash');
      if (expectedBytes.equals(result.stdout)) outcome(observation, 'identical-ir');
      else {
        observation.difference = firstDifference(expectedBytes, result.stdout);
        outcome(observation, 'ir-changed', 'IR differs at byte ' + observation.difference.byte_offset);
      }
    } else if (observed.diagnostic !== expected.diagnostic || observed.status !== expected.status) {
      outcome(observation, 'diagnostic-changed', 'expected ' + expected.diagnostic + ', actual ' + observed.diagnostic);
    } else {
      outcome(observation, input.outcome === 'pending' ? 'pending-equal' : 'rejection-equal');
    }
  } catch (error) {
    outcome(observation, 'observation-failure', String(error));
  }
}
const workers = await Promise.allSettled(Array.from({ length: jobs }, async () => {
  for (;;) {
    const index = next++;
    if (index >= cases.length) return;
    await observe(index);
  }
}));
for (const [index, worker] of workers.entries()) {
  if (worker.status === 'rejected') summary.failures.push({ worker: index, outcome: 'worker-failure', problem: String(worker.reason) });
}
for (let index = 0; index < cases.length; index++) {
  if (!observations[index]?.outcome) {
    observations[index] = { id: cases[index].id, source: cases[index].source };
    outcome(observations[index], 'observation-failure', 'worker returned no observation');
  }
}
summary.elapsed_seconds = (performance.now() - started) / 1000;
summary.observations = 'observations.json';
fs.writeFileSync(path.join(out, summary.observations), JSON.stringify(observations, null, 2) + '\n');
fs.writeFileSync(path.join(out, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');
console.log(JSON.stringify(summary));
if (summary.failures.length) process.exitCode = 1;
