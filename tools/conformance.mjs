#!/usr/bin/env node
// The process envelopes below belong to the test runner, never to the language.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = path.resolve(root, process.env.LKJSCRIPT_BIN || process.argv[2] || 'target/debug/lkjscript');
const fixtures = path.join(root, 'tests/conformance');
const cases = JSON.parse(fs.readFileSync(path.join(fixtures, 'cases.json'), 'utf8'));
const out = path.join(root, 'target/conformance');
fs.mkdirSync(out, { recursive: true });
const timeout = Number(process.env.LKJ_TEST_TIMEOUT_MS || 30000);
function run(command, args) {
    return spawnSync(command, args, { cwd: root, encoding: 'utf8', timeout, maxBuffer: 4 * 1024 * 1024 });
}
function diagnostic(result) {
    return (result.stderr || result.stdout || result.error?.message || '').trim();
}
function succeeded(result) { return !result.error && result.status === 0; }
function fail(message) { failures.push(message); }
const failures = [];
const pending = [];
let passed = 0;
let rustCompared = 0;
const started = performance.now();

// One Rust build contains every comparable positive case, including planned
// language features. Unsupported lkjscript cases remain explicitly pending.
const rustCases = cases.filter(c => c.rust && c.kind === 'run');
let harness = '#![allow(dead_code, unused_imports, unused_variables)]\n';
for (const c of rustCases) {
    harness += 'mod case_' + c.id + ' {\n' + fs.readFileSync(path.join(fixtures, c.file), 'utf8') + '\n}\n';
}
harness += 'fn main() { let name = std::env::args().nth(1).unwrap(); match name.as_str() {\n';
for (const c of rustCases) {
    harness += JSON.stringify(c.id) + ' => println!("{}", case_' + c.id + '::probe()),\n';
}
harness += '_ => std::process::exit(2), } }\n';
const oracleSource = path.join(out, 'oracle.rs');
const oracle = path.join(out, 'oracle');
fs.writeFileSync(oracleSource, harness);
const compiled = run('rustc', ['--edition=2024', '-C', 'overflow-checks=yes', '-C', 'opt-level=1', '-A', 'warnings', oracleSource, '-o', oracle]);
if (!succeeded(compiled)) {
    process.stderr.write('Rust oracle did not build:\n' + diagnostic(compiled) + '\n');
    process.exit(1);
}

for (const c of cases) {
    const source = path.join(fixtures, c.file);
    const checked = run(binary, ['check', source]);
    if (!succeeded(checked)) {
        const error = diagnostic(checked);
        const actualCode = error.match(/\bE_[A-Z_]+\b/)?.[0] || 'NO_DIAGNOSTIC';
        if (c.pending && (actualCode === 'E_PARSE' || actualCode === 'E_UNSUPPORTED')) {
            pending.push(c.id + ': ' + error.split('\n')[0]);
            continue;
        }
        if (c.kind !== 'reject' || actualCode !== c.code) {
            fail(c.id + ': expected ' + (c.kind === 'reject' ? c.code : 'acceptance') + ', got ' + error);
            continue;
        }
    } else if (c.kind === 'reject') {
        fail(c.id + ': expected ' + c.code + ', but check accepted the program');
        continue;
    }

    if (c.kind === 'run') {
        const args = ['run', source];
        if (c.entry) args.push('--entry', c.entry);
        const executed = run(binary, args);
        if (!succeeded(executed)) {
            fail(c.id + ': execution failed: ' + diagnostic(executed));
            continue;
        }
        const actual = executed.stdout.trim();
        if (actual !== c.value) {
            fail(c.id + ': expected ' + c.value + ', got ' + JSON.stringify(actual));
            continue;
        }
        if (c.rust) {
            const reference = run(oracle, [c.id]);
            if (!succeeded(reference) || reference.stdout.trim() !== actual) {
                fail(c.id + ': rustc/Node disagreement: ' + diagnostic(reference) + ' result=' + reference.stdout?.trim());
                continue;
            }
            rustCompared++;
        }
    } else if (c.rust) {
        const reference = run('rustc', ['--edition=2024', '--crate-type=lib', '--emit=metadata', '-A', 'warnings', source, '-o', path.join(out, c.id + '.rmeta')]);
        if (succeeded(reference) !== Boolean(c.rustAccept)) {
            fail(c.id + ': unexpected Rust oracle verdict: ' + diagnostic(reference));
            continue;
        }
        rustCompared++;
    }
    passed++;
}
const seconds = ((performance.now() - started) / 1000).toFixed(2);
process.stdout.write('conformance: ' + passed + ' passed, ' + pending.length + ' pending, ' + failures.length + ' failed; ' + rustCompared + ' rustc comparisons; ' + seconds + ' s\n');
for (const message of pending) process.stdout.write('PENDING ' + message + '\n');
for (const message of failures) process.stderr.write('FAIL ' + message + '\n');
if (failures.length !== 0) process.exit(1);
