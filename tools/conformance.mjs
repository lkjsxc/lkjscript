#!/usr/bin/env node
// The process envelopes below belong to the test runner, never to the language.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { run as runProcess } from './process.mjs';
import { runRoutes } from './batch.mjs';
import { oracleCache, packResult, unpackResult } from './oracle.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = path.resolve(root, process.env.LKJSCRIPT_BIN || process.argv[2] || 'target/debug/lkjscript');
const engines = (process.env.LKJ_ENGINES || process.env.LKJ_ENGINE || 'native').split(',');
const fixtures = path.join(root, 'tests/conformance');
const cases = JSON.parse(fs.readFileSync(path.join(fixtures, 'cases.json'), 'utf8'));
const kinds = new Set(['run', 'output', 'reject', 'trap']);
const ids = new Set();
for (const c of cases) {
    if (!kinds.has(c.kind)) throw new Error('unknown conformance kind: ' + c.kind);
    if (ids.has(c.id)) throw new Error('duplicate conformance id: ' + c.id);
    ids.add(c.id);
    if (c.rustCodes !== undefined && (c.rustCode !== undefined || !Array.isArray(c.rustCodes)
        || c.rustCodes.length === 0 || c.rustCodes.some(code => typeof code !== 'string' || !/^E[0-9]{4}$/.test(code)))) {
        throw new Error('Rust diagnostic alternatives must be a nonempty code list: ' + c.id);
    }
    if (c.kind === 'output' && (!c.rust || typeof c.hex !== 'string' || !/^(?:[0-9a-fA-F]{2})*$/.test(c.hex)
        || (c.ending !== 'normal' && !/^R[0-9]{4}$/.test(c.ending)))) {
        throw new Error('output cases need a Rust oracle, hexadecimal bytes and an explicit ending: ' + c.id);
    }
    if (c.kind === 'trap' && (c.rust || !/^R[0-9]{4}$/.test(c.code))) {
        throw new Error('trap cases need a runtime code and no Rust oracle; use output for Rust trap comparisons: ' + c.id);
    }
}
const out = path.join(root, 'target/conformance', engines.join('-'));
fs.mkdirSync(out, { recursive: true });
const timeout = Number(process.env.LKJ_TEST_TIMEOUT_MS || 30000);
function run(command, args, options = {}) {
    return runProcess(command, args, { cwd: root, encoding: 'utf8', timeout, maxBuffer: 4 * 1024 * 1024, ...options });
}
function diagnostic(result) {
    return [result.stderr, result.stdout, result.error?.message].map(value => value?.toString().trim()).find(Boolean) || '';
}
function succeeded(result) { return !result.error && result.status === 0; }
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
const referenceResults = await oracleCache({suite:'conformance-positive',harness,flags:'2024 overflow-checks=yes opt-level=1'},async()=>{
 const compiled=await run('rustc',['--edition=2024','-C','overflow-checks=yes','-C','opt-level=1','-A','warnings',oracleSource,'-o',oracle]);
 if(!succeeded(compiled))throw new Error('Rust oracle did not build: '+diagnostic(compiled));
 const results={};
 for(const c of rustCases)results[c.id]=packResult(await run(oracle,[c.id],{encoding:null}));
 return results;
});
// A new rejection fixture must not invalidate every unchanged Rust rejection.
// Cache each source with its complete host wrapper and the actual compiler flags.
const negativeFlags=['--edition=2024','--crate-type=lib','--emit=metadata','-A','warnings'];
for(const c of cases.filter(c=>c.rust&&c.kind!=='run'&&c.kind!=='output')){
 let negativeSource=path.join(fixtures,c.file);
 let source=fs.readFileSync(negativeSource,'utf8');
 if(c.kind==='reject'&&c.world){
  negativeSource=path.join(out,c.id+'.rs');
  source='mod __lkj_host {\n'+fs.readFileSync(path.join(root,'bridge/output.rs'),'utf8')+'\n}\nmod case {\n'+source+'\nuse crate::__lkj_host::{World,Stdout};\n}\n';
  fs.writeFileSync(negativeSource,source);
 }
 referenceResults[c.id]=await oracleCache({suite:'conformance-rejection',file:c.file,source,flags:negativeFlags},async()=>packResult(
  await run('rustc',[...negativeFlags,negativeSource,'-o',path.join(out,c.id+'.rmeta')],{encoding:null})
 ));
}
const actualRoutes = await runRoutes(binary,cases.map(c=>({path:path.join(fixtures,c.file),entry:c.entry,mode:c.kind==='reject'?'check':'run'})),{engines,output:out,encoding:null});
for (const engine of engines) {
const actualResults = actualRoutes[engine];
const failures = [], pending = [];
const fail = message => failures.push(message);
let passed = 0, rustCompared = 0;

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

    if (c.kind === 'trap') {
        const code = result.stderr.toString().match(/: (R[0-9]+)\b/)?.[1];
        const expected = Buffer.from(c.hex || '', 'hex');
        if (result.error || result.signal || result.status === 0 || code !== c.code
            || !result.stdout.equals(expected)) {
            fail(c.id + ': expected ' + c.code + ' and output ' + expected.toString('hex')
                + ', got ' + diagnostic(result));
            continue;
        }
    } else if (c.kind === 'output') {
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
                fail(c.id + ': rustc/' + engine + ' disagreement: ' + diagnostic(reference) + ' result=' + reference.stdout?.trim());
                continue;
            }
            rustCompared++;
        }
    } else if (c.rust) {
        const reference = unpackResult(referenceResults[c.id]);
        const codes = c.rustCodes ?? (c.rustCode ? [c.rustCode] : []);
        if (succeeded(reference) !== Boolean(c.rustAccept) || (codes.length && !codes.some(code => reference.stderr.includes('['+code+']')))) {
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
if (failures.length !== 0) process.exitCode = 1;
}
