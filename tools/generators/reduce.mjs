#!/usr/bin/env node
// Reduce one generated source while preserving its observed failure and Rust
// validity assumption. Process time/output envelopes belong to this tool only.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
import { run } from '../process.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const hash = text => createHash('sha256').update(text).digest('hex');
const OWN = new Set(['E0382', 'E0499', 'E0502', 'E0503', 'E0505', 'E0506',
  'E0507', 'E0509', 'E0515', 'E0596', 'E0597', 'E0716', 'E0713', 'E0381', 'E0384', 'E0594']);
const equalCategories = new Set(['value-equal', 'trap-equal', 'both-run-value-equal',
  'both-run-trap-equal', 'both-reject-ownership', 'equal-output-exit', 'equal-output-trap']);

function tokens(source) {
  const result = [];
  const add = (kind, start, end) => result.push({kind, text: source.slice(start, end), start, end});
  let at = 0;
  while (at < source.length) {
    const start = at;
    if (/\s/.test(source[at])) { at++; continue; }
    if (source.startsWith('//', at)) { at = source.indexOf('\n', at); if (at < 0) at = source.length; continue; }
    if (source.startsWith('/*', at)) {
      at += 2; let depth = 1;
      while (at < source.length && depth) {
        if (source.startsWith('/*', at)) { depth++; at += 2; }
        else if (source.startsWith('*/', at)) { depth--; at += 2; }
        else at++;
      }
      continue;
    }
    const raw = source.slice(at).match(/^(?:br|cr|r)(#*)"/);
    if (raw) {
      const end = source.indexOf('"' + raw[1], at + raw[0].length);
      at = end < 0 ? source.length : end + 1 + raw[1].length;
      add('literal', start, at); continue;
    }
    let quote = source[at];
    if ((quote === 'b' || quote === 'c') && (source[at + 1] === '"' || source[at + 1] === "'")) quote = source[++at];
    if (quote === '"' || quote === "'") {
      const lifetime = quote === "'" && source.slice(at).match(/^'[A-Za-z_][A-Za-z0-9_]*/);
      if (lifetime && source[at + lifetime[0].length] !== "'") {
        at += lifetime[0].length; add('lifetime', start, at); continue;
      }
      at++;
      while (at < source.length) {
        if (source[at] === '\\') at += 2;
        else if (source[at++] === quote) break;
      }
      add('literal', start, Math.min(at, source.length)); continue;
    }
    if (/[A-Za-z_0-9]/.test(source[at])) {
      while (at < source.length && /[A-Za-z_0-9]/.test(source[at])) at++;
      add('word', start, at); continue;
    }
    const operator = source.slice(at, at + 2);
    at += ['::', '->', '=>', '&&', '||', '==', '!=', '<=', '>=', '..'].includes(operator) ? 2 : 1;
    add('symbol', start, at);
  }
  return result;
}

// A small delimiter reader, not a second Rust parser. Candidate spans end only
// at balanced statement/item boundaries; rustc decides candidate validity.
export function statementSpans(source) {
  const list = tokens(source), pairs = new Map(), groups = [], stack = [];
  const close = {'(': ')', '[': ']', '{': '}'};
  for (let i = 0; i < list.length; i++) {
    const token = list[i];
    if (token.kind !== 'symbol') continue;
    if (close[token.text]) {
      const previous = list[i - 1], before = list[i - 2];
      const macro = stack[stack.length - 1]?.macro
        || (previous?.text === '!' && before?.kind === 'word')
        || (previous?.kind === 'word' && before?.text === '!' && list[i - 3]?.text === 'macro_rules')
        || previous?.text === '#';
      stack.push({start: i, end: undefined, macro});
    } else if ([')', ']', '}'].includes(token.text)) {
      const current = stack[stack.length - 1];
      if (!current || close[list[current.start].text] !== token.text) continue;
      stack.pop(); current.end = i; pairs.set(current.start, i); groups.push(current);
    }
  }
  const spans = [];
  const continuations = new Set(['else', '{', '.', '?', '(', '[', '+', '-', '*', '/',
    '%', '&', '|', '^', '&&', '||', '==', '!=', '<', '>', '<=', '>=', '=']);
  function collect(first, last, top) {
    let start = first;
    function emit(end) {
      if (start < end) spans.push({start: list[start].start, end: list[end - 1].end});
      start = end;
    }
    function leading() {
      let index = start;
      while (list[index]?.text === '#' && list[index + 1]?.text === '[' && pairs.has(index + 1)) index = pairs.get(index + 1) + 1;
      if (list[index]?.kind === 'lifetime' && list[index + 1]?.text === ':') index += 2;
      return list[index]?.text;
    }
    for (let at = first; at < last; at++) {
      const token = list[at];
      if (token.kind !== 'symbol') continue;
      if (token.text === ';') { emit(at + 1); continue; }
      const end = pairs.get(at);
      if (end === undefined) continue;
      const lead = leading();
      const standalone = ['{', 'if', 'while', 'loop', 'for', 'match', 'unsafe', 'async'].includes(lead);
      const item = ['fn', 'pub', 'struct', 'enum', 'impl', 'trait', 'mod'].includes(lead);
      if (token.text === '{' && (top || standalone || item) && !continuations.has(list[end + 1]?.text)) {
        const withSemicolon = list[end + 1]?.text === ';' ? end + 2 : end + 1;
        emit(withSemicolon); at = withSemicolon - 1;
      } else at = end;
    }
  }
  collect(0, list.length, true);
  for (const group of groups) {
    if (list[group.start].text === '{' && !group.macro) collect(group.start + 1, group.end, false);
  }
  const unique = new Map(spans.map(span => [span.start + ':' + span.end, span]));
  return [...unique.values()].sort((a, b) => (b.end - b.start) - (a.end - a.start) || a.start - b.start);
}

export function preservesFailure(baseline, candidate) {
  return candidate.category === baseline.category
    && candidate.rustAccepted === baseline.rustAccepted
    && candidate.rustAccepted !== null
    && candidate.rustCode === baseline.rustCode
    && candidate.lkCode === baseline.lkCode;
}

export async function reduceSource(source, evaluate, options = {}) {
  const baseline = options.baseline || await evaluate(source);
  const category = options.category || 'auto';
  if (category !== 'auto' && category !== baseline.category) {
    throw new Error('expected category ' + category + ', observed ' + baseline.category);
  }
  if (baseline.rustAccepted === null || /oracle|generator/.test(baseline.category)
      || (category === 'auto' && equalCategories.has(baseline.category))) {
    throw new Error('baseline is not a reducible compiler disagreement: ' + baseline.category);
  }
  const cache = new Map([[hash(source), baseline]]);
  let attempts = 0, accepted = 0, current = source, observation = baseline;
  for (;;) {
    let changed = false;
    for (const span of statementSpans(current)) {
      const candidate = current.slice(0, span.start) + current.slice(span.end);
      const key = hash(candidate);
      let result = cache.get(key);
      if (!result) { result = await evaluate(candidate); cache.set(key, result); attempts++; }
      if (preservesFailure(baseline, result)) {
        current = candidate; observation = result; accepted++; changed = true;
        await options.onAccept?.({source: current, observation, attempts, accepted});
        break;
      }
    }
    if (!changed) return {source: current, baseline, observation, attempts, accepted};
  }
}

const normalTrap = code => Number(code.slice(1)) >= 16 ? 'R0004' : code;
const rustTrap = text => /with overflow/.test(text) ? 'R0001'
  : /divide by zero|divisor of zero/.test(text) ? 'R0002'
  : /index out of bounds/.test(text) ? 'R0003' : /assertion/.test(text) ? 'R0005' : 'R0004';
function completed(result) { return !result.error && !result.signal && result.status === 0; }
function execution(result, rust) {
  const stdout = result.stdout, stderr = result.stderr.toString();
  if (result.error || result.signal) return {kind: result.error?.code === 'ETIMEDOUT' ? 'timeout' : 'crash', stdout};
  if (result.status === 0) return {kind: 'value', stdout};
  if (rust) {
    const shim = stderr.match(/LKJ_RUST_TRAP (R[0-9]+)/);
    if (shim) return {kind: 'trap', code: normalTrap(shim[1]), stdout};
    const panic = stderr.match(/panicked at [^\n]*?:(\d+):\d+:\r?\n([^\n]*)/);
    if (result.status === 101 && panic) return {kind: 'trap', code: rustTrap(panic[2]), line: Number(panic[1]), stdout};
    return {kind: 'crash', stdout};
  }
  const diagnostic = stderr.match(/:(\d+):(?:\d+:)? (E_[A-Z_]+|M_[A-Z_]+|R[0-9]+)\b/);
  if (!diagnostic) return {kind: 'crash', stdout};
  const code = diagnostic[2];
  return {kind: code[0] === 'R' ? 'trap' : code[0] === 'M' ? 'machine-error' : 'reject',
    code: code[0] === 'R' ? normalTrap(code) : code, line: Number(diagnostic[1]), stdout};
}
export function classify(reference, actual, profile = 'verdict') {
  if (reference.kind === 'timeout' || reference.kind === 'crash') return 'oracle-abnormal';
  if (reference.kind === 'reject' && !OWN.has(reference.code)) return 'generator-or-oracle-problem';
  if (actual.kind === 'timeout' || actual.kind === 'crash') return actual.kind;
  if (actual.kind === 'machine-error') return 'machine-error:' + actual.code;
  if (reference.kind === 'reject') {
    if (actual.kind !== 'reject') return 'ACCEPTS-RUST-REJECTED';
    return ['E_MOVE', 'E_BORROW'].includes(actual.code) ? 'both-reject-ownership' : 'rejected-for-another-reason:' + actual.code;
  }
  if (actual.kind === 'reject') return ['E_PARSE', 'E_UNSUPPORTED'].includes(actual.code) ? 'unsupported' : 'rejects-valid:' + actual.code;
  if (profile === 'output') {
    if (reference.kind !== actual.kind || reference.code !== actual.code) return 'different-ending';
    if (!reference.stdout.equals(actual.stdout)) return 'different-output';
    return reference.kind === 'value' ? 'equal-output-exit' : 'equal-output-trap';
  }
  if (reference.kind === 'value' && actual.kind === 'value') {
    if (reference.stdout.toString().trim() !== actual.stdout.toString().trim()) return 'VALUE-DIFFERS';
    return profile === 'value' ? 'value-equal' : 'both-run-value-equal';
  }
  if (reference.kind === 'trap' && actual.kind === 'trap') {
    if (reference.code !== actual.code) return 'TRAP-KIND-DIFFERS';
    if (profile === 'value' && reference.line !== actual.line) return 'trap-same-kind-other-line';
    return profile === 'value' ? 'trap-equal' : 'both-run-trap-equal';
  }
  return 'VALUE-VS-TRAP';
}
function compact(outcome) {
  return {kind: outcome.kind, code: outcome.code, line: outcome.line,
    stdoutBytes: outcome.stdout?.length || 0, stdoutSha256: hash(outcome.stdout || Buffer.alloc(0))};
}
function writeAtomic(file, content) {
  fs.writeFileSync(file + '.tmp-' + process.pid, content);
  fs.renameSync(file + '.tmp-' + process.pid, file);
}

export function createEvaluator(binary, options = {}) {
  const engine = options.engine || 'node', entry = options.entry || 'probe', profile = options.profile || 'verdict';
  if (!['native', 'k'].includes(engine) || !['value', 'verdict', 'output'].includes(profile)) throw new Error('invalid engine or profile');
  if (!/^[A-Za-z_][A-Za-z0-9_]*(::[A-Za-z_][A-Za-z0-9_]*)*$/.test(entry)) throw new Error('entry must be a Rust path');
  fs.mkdirSync(path.join(root, 'target'), {recursive: true});
  const directory = fs.mkdtempSync(path.join(root, 'target/reduce-'));
  const frozen = path.join(directory, 'lkjscript'), sourceFile = path.join(directory, 'case.rs');
  const rustSource = path.join(directory, 'oracle.rs'), rustCase = path.join(directory, 'oracle_case.rs'), oracle = path.join(directory, 'oracle');
  fs.copyFileSync(binary, frozen); fs.chmodSync(frozen, 0o700);
  const runtimeFiles = ['bridge/output.rs', 'tools/process.mjs']
    .filter(file => fs.existsSync(path.join(root, file)));
  const runtimeHashes = Object.fromEntries(runtimeFiles.map(file => [file, hash(fs.readFileSync(path.join(root, file)))]));
  const shim = path.join(directory, 'output.rs');
  if (profile === 'output') fs.copyFileSync(path.join(root, 'bridge/output.rs'), shim);
  const flags = ['--edition=2024', '--error-format=json', '-C', 'overflow-checks=yes', '-C', 'opt-level=0', '-A', 'warnings'];
  if (profile === 'value') flags.push('-A', 'arithmetic_overflow', '-A', 'unconditional_panic', '-A', 'overflowing_literals');
  const processOptions = {cwd: root, encoding: null, timeout: options.timeout || 10000};
  function stableRuntime() {
    for (const [file, digest] of Object.entries(runtimeHashes)) {
      if (hash(fs.readFileSync(path.join(root, file))) !== digest) throw new Error('runtime changed during reduction: ' + file);
    }
  }
  const evaluate = async source => {
    stableRuntime();
    const adapter = '__lkjscript_reduce_' + hash(source).slice(0, 16);
    fs.writeFileSync(sourceFile, source);
    const importHost = profile === 'output' ? '\nuse crate::__lkj_host::{World,Stdout};\n' : '\n';
    const body = profile === 'output' ? 'crate::__lkj_host::enter(' + entry + ');' : 'println!("{}", ' + entry + '());';
    fs.writeFileSync(rustCase, source + importHost + 'pub fn ' + adapter + '() { ' + body + ' }\n');
    fs.writeFileSync(rustSource, '#![allow(warnings)]\n'
      + (profile === 'output' ? '#[path="output.rs"] mod __lkj_host;\n' : '')
      + '#[path="oracle_case.rs"] mod case;\nfn main() { case::' + adapter + '(); }\n');
    const built = await run('rustc', [...flags, rustSource, '-o', oracle], processOptions);
    let reference, rustAccepted;
    if (built.error || built.signal) {
      rustAccepted = null; reference = execution(built, true);
    } else if (!completed(built)) {
      rustAccepted = false;
      const diagnostics = built.stderr.toString().split('\n').flatMap(line => {
        try { const diagnostic = JSON.parse(line); return diagnostic.level === 'error' ? [diagnostic] : []; }
        catch { return []; }
      });
      reference = {kind: 'reject', code: diagnostics.find(d => d.code?.code)?.code.code || 'NO_RUST_CODE', stdout: Buffer.alloc(0)};
    } else {
      rustAccepted = true; reference = execution(await run(oracle, [], processOptions), true);
    }
    const actual = execution(await run(frozen, ['run', sourceFile, '--entry', entry, '--engine', engine], processOptions), false);
    stableRuntime();
    return {category: classify(reference, actual, profile), rustAccepted,
      rustCode: reference.kind === 'reject' ? reference.code : undefined,
      lkCode: actual.code, reference: compact(reference), actual: compact(actual)};
  };
  return {evaluate, close: () => fs.rmSync(directory, {recursive: true, force: true}),
    binarySha256: hash(fs.readFileSync(frozen)), runtimeHashes, rustFlags: flags};
}

const usage = 'usage: node tools/generators/reduce.mjs <binary> <source.rs> <output.rs>'
  + ' [--engine native|k] [--entry probe] [--category auto|CLASS]'
  + ' [--profile verdict|value|output] [--timeout milliseconds]\n';
async function main() {
  const args = process.argv.slice(2);
  if (args.includes('--help')) { process.stdout.write(usage); return; }
  const positional = [], options = {engine: 'native', category: 'auto', profile: 'verdict'};
  for (let i = 0; i < args.length; i++) {
    if (!args[i].startsWith('--')) { positional.push(args[i]); continue; }
    const name = args[i].slice(2);
    if (!['engine', 'entry', 'category', 'profile', 'timeout'].includes(name) || args[i + 1] === undefined) throw new Error(usage);
    options[name] = args[++i];
  }
  if (positional.length !== 3) throw new Error(usage);
  if (options.timeout !== undefined && (!Number.isSafeInteger(Number(options.timeout)) || Number(options.timeout) <= 0)) throw new Error('timeout must be a positive number of milliseconds');
  if (options.timeout !== undefined) options.timeout = Number(options.timeout);
  options.entry ||= options.profile === 'output' ? 'main' : 'probe';
  const [binary, input, output] = positional.map(file => path.resolve(file));
  if (input === output) throw new Error('use a separate output file to preserve the original failing source');
  fs.mkdirSync(path.dirname(output), {recursive: true});
  const source = fs.readFileSync(input, 'utf8'), evaluator = createEvaluator(binary, options), started = performance.now();
  const identities = await Promise.all([
    run('git', ['rev-parse', 'HEAD'], {cwd: root}),
    run('git', ['status', '--porcelain', '--untracked-files=no'], {cwd: root}),
    run('rustc', ['--version'], {cwd: root}),
  ]);
  const record = {tool: 'statement-reducer', engine: options.engine, profile: options.profile, entry: options.entry,
    reducerSha256: hash(fs.readFileSync(fileURLToPath(import.meta.url))), binarySha256: evaluator.binarySha256,
    repositoryHead: identities[0].stdout.trim(), repositoryDirty: identities[1].stdout.trim() !== '',
    rustc: identities[2].stdout.trim(), node: process.version, rustFlags: evaluator.rustFlags,
    runtimeHashes: evaluator.runtimeHashes, input, inputSha256: hash(source), inputBytes: Buffer.byteLength(source), output};
  try {
    const result = await reduceSource(source, evaluator.evaluate, {...options,
      onAccept: async progress => { writeAtomic(output, progress.source); options.onAccept?.(progress); }});
    writeAtomic(output, result.source);
    Object.assign(record, {status: 'complete', category: result.baseline.category, rustAccepted: result.baseline.rustAccepted,
      attempts: result.attempts, deletions: result.accepted, outputBytes: Buffer.byteLength(result.source), outputSha256: hash(result.source),
      baseline: result.baseline, reduced: result.observation});
  } catch (error) {
    Object.assign(record, {status: 'aborted', error: error.message}); process.exitCode = 1;
  } finally {
    evaluator.close(); record.elapsedMs = performance.now() - started;
    fs.appendFileSync(output + '.jsonl', JSON.stringify(record) + '\n');
    process.stdout.write(JSON.stringify(record) + '\n');
  }
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(error => { process.stderr.write(error.message + '\n'); process.exitCode = 1; });
}
