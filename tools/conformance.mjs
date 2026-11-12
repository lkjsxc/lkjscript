#!/usr/bin/env node
// The process envelopes below belong to the test runner, never to the language.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { run as runProcess } from './process.mjs';
import { runCases } from './batch.mjs';
import { oracleCache, packResult, unpackResult } from './oracle.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = path.resolve(root, process.env.LKJSCRIPT_BIN || process.argv[2] || 'target/debug/lkjscript');
const engine = process.env.LKJ_ENGINE || 'node';
const fixtures = path.join(root, 'tests/conformance');
const cases = JSON.parse(fs.readFileSync(path.join(fixtures, 'cases.json'), 'utf8'));
const out = path.join(root, 'target/conformance', engine);
fs.mkdirSync(out, { recursive: true });
const timeout = Number(process.env.LKJ_TEST_TIMEOUT_MS || 30000);
function run(command, args, options = {}) {
    return runProcess(command, args, { cwd: root, encoding: 'utf8', timeout, maxBuffer: 4 * 1024 * 1024, ...options });
}
function diagnostic(result) {
    return (result.stderr || result.stdout || result.error?.message || '').toString().trim();
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
const rustCases = cases.filter(c => c.rust && (c.kind === 'run' || c.kind === 'output'));
let harness = '#![allow(dead_code, unused_imports, unused_variables)]\n';
harness += 'mod __lkj_host {\n' + fs.readFileSync(path.join(root, 'bridge/output.rs'), 'utf8') + '\n}\n';
for (const c of rustCases) {
    harness += 'mod case_' + c.id + ' {\n' + (c.kind === 'output' ? 'use crate::__lkj_host::{World, Stdout};\n' : '') + fs.readFileSync(path.join(fixtures, c.file), 'utf8') + '\n}\n';
}
harness += 'fn main() { let name = std::env::args().nth(1).unwrap(); match name.as_str() {\n';
for (const c of rustCases) {
    harness += JSON.stringify(c.id) + (c.kind === 'output' ? ' => __lkj_host::enter(case_' + c.id + '::main),\n' : ' => println!("{}", case_' + c.id + '::probe()),\n');
}
harness += '_ => std::process::exit(2), } }\n';
const oracleSource = path.join(out, 'oracle.rs');
const oracle = path.join(out, 'oracle');
fs.writeFileSync(oracleSource, harness);
const referenceResults = await oracleCache({suite:'conformance',harness,flags:'2024 overflow-checks=yes opt-level=1',negative:cases.filter(c=>c.rust&&c.kind!=='run'&&c.kind!=='output').map(c=>[c.id,fs.readFileSync(path.join(fixtures,c.file),'utf8')])},async()=>{
 const compiled=await run('rustc',['--edition=2024','-C','overflow-checks=yes','-C','opt-level=1','-A','warnings',oracleSource,'-o',oracle]);
 if(!succeeded(compiled))throw new Error('Rust oracle did not build: '+diagnostic(compiled));
 const results={};
 for(const c of cases.filter(c=>c.rust)) {
  const result=c.kind==='run'||c.kind==='output'?await run(oracle,[c.id],{encoding:null}):await run('rustc',['--edition=2024','--crate-type=lib','--emit=metadata','-A','warnings',path.join(fixtures,c.file),'-o',path.join(out,c.id+'.rmeta')],{encoding:null});
  results[c.id]=packResult(result);
 }
 return results;
});
const actualResults = await runCases(binary,cases.map(c=>({path:path.join(fixtures,c.file),entry:c.entry,mode:c.kind==='run'||c.kind==='output'?'run':'check'})),{engine,output:out,encoding:null});

for (const c of cases) {
    const source = path.join(fixtures, c.file);
    const result = actualResults[cases.indexOf(c)];
    const checked = {...result,status:result.checkStatus};
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

    if (c.kind === 'output') {
        const actual = result;
        const reference = unpackResult(referenceResults[c.id],null);
        const normalKind = code => Number(code.slice(1)) >= 16 ? 'R0004' : code;
        const ending = (result, rust) => {
            if (result.error) return result.error.code === 'ETIMEDOUT' ? 'timeout' : 'crash';
            if (result.status === 0 && !result.signal) return 'normal';
            const diagnostic = result.stderr.toString();
            const code = diagnostic.match(rust ? /LKJ_RUST_TRAP (R[0-9]+)/ : /: (R[0-9]+)\b/)?.[1];
            return code ? normalKind(code) : 'crash';
        };
        const expected = Buffer.from(c.hex, 'hex');
        if (!actual.stdout.equals(expected) || !reference.stdout.equals(expected)
            || ending(actual, false) !== c.ending || ending(reference, true) !== c.ending) {
            fail(c.id + ': output/termination differs: lkj=' + ending(actual, false)
                + ' rust=' + ending(reference, true) + ' expected=' + c.ending + ' ' + diagnostic(actual));
            continue;
        }
        rustCompared++;
    } else if (c.kind === 'run') {
        const executed = {...result,stdout:result.stdout.toString(),stderr:result.stderr.toString()};
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
            const reference = unpackResult(referenceResults[c.id]);
            if (!succeeded(reference) || reference.stdout.trim() !== actual) {
                fail(c.id + ': rustc/Node disagreement: ' + diagnostic(reference) + ' result=' + reference.stdout?.trim());
                continue;
            }
            rustCompared++;
        }
    } else if (c.rust) {
        const reference = unpackResult(referenceResults[c.id]);
        if (succeeded(reference) !== Boolean(c.rustAccept)) {
            fail(c.id + ': unexpected Rust oracle verdict: ' + diagnostic(reference));
            continue;
        }
        rustCompared++;
    }
    passed++;
}
const seconds = ((performance.now() - started) / 1000).toFixed(2);
process.stdout.write('conformance (' + engine + '): ' + passed + ' passed, ' + pending.length + ' pending, ' + failures.length + ' failed; ' + rustCompared + ' rustc comparisons; ' + seconds + ' s\n');
for (const message of pending) process.stdout.write('PENDING ' + message + '\n');
for (const message of failures) process.stderr.write('FAIL ' + message + '\n');
if (failures.length !== 0) process.exit(1);
