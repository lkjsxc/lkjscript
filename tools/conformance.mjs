#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { run as runProcess } from './process.mjs';
import { runRoutes } from './batch.mjs';
import { oracleCache, packResult, unpackResult, worldOutcome } from './oracle.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const binary = path.resolve(root, process.env.LKJSCRIPT_BIN || process.argv[2] || 'target/debug/lkjscript');
const engines = (process.env.LKJSCRIPT_ENGINES || process.env.LKJSCRIPT_ENGINE || 'native').split(',');
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
    if (c.status !== undefined && (!Number.isInteger(c.status) || c.status < 0 || c.status > 255)) throw new Error('invalid expected exit status: ' + c.id);
    if (c.stderrHex !== undefined && (typeof c.stderrHex !== 'string' || !/^(?:[0-9a-fA-F]{2})*$/.test(c.stderrHex))) throw new Error('invalid expected stderr bytes: ' + c.id);
    if (c.kind === 'trap' && (c.rust || !/^R[0-9]{4}$/.test(c.code))) {
        throw new Error('trap cases need a runtime code and no Rust oracle; use output for Rust trap comparisons: ' + c.id);
    }
}
const out = path.join(root, 'target/conformance', engines.join('-'));
fs.mkdirSync(out, { recursive: true });
const timeout = Number(process.env.LKJSCRIPT_TEST_TIMEOUT_MS || 30000);
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
harness += 'mod __lkjscript_host {\n' + fs.readFileSync(path.join(root, 'bridge/output.rs'), 'utf8') + '\n}\n';
for (const c of rustCases) {
    harness += 'mod case_' + c.id + ' {\n' + (c.kind === 'output' ? 'use crate::__lkjscript_host::{World, Stdout, Stdin, Stderr, Directory, File};\n' : '') + fs.readFileSync(path.join(fixtures, c.file), 'utf8') + '\n}\n';
}
harness += 'fn main() { let name = std::env::var("LKJSCRIPT_ORACLE_CASE").unwrap(); match name.as_str() {\n';
for (const c of rustCases) {
    harness += JSON.stringify(c.id) + (c.kind === 'output' ? ' => __lkjscript_host::enter(case_' + c.id + '::main),\n' : ' => __lkjscript_host::observe(|| { println!("{}", case_' + c.id + '::probe()); 0 }),\n');
}
harness += '_ => std::process::exit(2), } }\n';
const oracleSource = path.join(out, 'oracle.rs');
const oracle = path.join(out, 'oracle');
fs.writeFileSync(oracleSource, harness);
const referenceResults = await oracleCache({suite:'conformance-positive',harness,flags:'2024 overflow-checks=yes opt-level=1 cfg=lkjscript_control'},async()=>{
 const compiled=await run('rustc',['--edition=2024','--cfg','lkjscript_control','-C','overflow-checks=yes','-C','opt-level=1','-A','warnings',oracleSource,'-o',oracle]);
 if(!succeeded(compiled))throw new Error('Rust oracle did not build: '+diagnostic(compiled));
 const results={};
 for(const c of rustCases)results[c.id]=packResult(await run(oracle,[],{encoding:null,captureControl:true,env:{...process.env,LKJSCRIPT_ORACLE_CASE:c.id}}));
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
  source='mod __lkjscript_host {\n'+fs.readFileSync(path.join(root,'bridge/output.rs'),'utf8')+'\n}\nmod case {\n'+source+'\nuse crate::__lkjscript_host::{World,Stdout,Stdin,Stderr,Directory,File};\n}\n';
  fs.writeFileSync(negativeSource,source);
 }
 referenceResults[c.id]=await oracleCache({suite:'conformance-rejection',file:c.file,source,flags:negativeFlags},async()=>packResult(
  await run('rustc',[...negativeFlags,negativeSource,'-o',path.join(out,c.id+'.rmeta')],{encoding:null})
 ));
}
const jobs = Number(process.env.LKJSCRIPT_CONFORMANCE_JOBS || 1);
const batchStarted = performance.now();
const actualRoutes = await runRoutes(binary,cases.map(c=>({path:path.join(fixtures,c.file),entry:c.entry,mode:c.kind==='reject'?'check':'run'})),{engines,output:out,encoding:null,jobs});
const batchSeconds = (performance.now() - batchStarted) / 1000;
for (const engine of engines) {
    fs.writeFileSync(path.join(out, 'results-' + engine + '.jsonl'), actualRoutes[engine].map(result => JSON.stringify({
        index: result.index, id: cases[result.index].id, engine, checkStatus: result.checkStatus,
        status: result.status, stdoutHex: result.stdout.toString('hex'),
        stderrHex: result.programStderr.toString('hex'), diagnostic: result.stderr.toString(),
        signal: result.signal, error: result.error ? {message: result.error.message, code: result.error.code} : null,
    })).join('\n') + (cases.length ? '\n' : ''));
}
fs.writeFileSync(path.join(out, 'batch-summary.json'), JSON.stringify({jobs,cases:cases.length,engines,batchSeconds},null,2)+'\n');
process.stdout.write('conformance batch: ' + cases.length + ' cases, ' + jobs + ' workers, ' + batchSeconds.toFixed(3) + ' s\n');
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
        const emittedCode = result.stderr.toString().match(/: (R[0-9]+)\b/)?.[1];
        const code = c.code === 'R0004' && emittedCode && Number(emittedCode.slice(1)) >= 16 ? 'R0004' : emittedCode;
        const expected = Buffer.from(c.hex || '', 'hex');
        if (result.error || result.signal || result.status === 0 || code !== c.code
            || !result.stdout.equals(expected)
            || (c.line !== undefined && !result.stderr.toString().includes(`:${c.line}: ${emittedCode} `))) {
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
            if (rust) {
                const observed = worldOutcome(result);
                if (observed.kind === 'trap') return normalKind(observed.code);
                return observed.kind === 'normal' && observed.status === (c.status ?? 0) ? 'normal' : 'crash';
            }
            if (result.status === (c.status ?? 0) && !result.signal && result.stderr.length === 0) return 'normal';
            const code = result.stderr.toString().match(/: (R[0-9]+)\b/)?.[1];
            return code ? normalKind(code) : 'crash';
        };
        const expected = Buffer.from(c.hex, 'hex');
        const expectedStderr = Buffer.from(c.stderrHex || '', 'hex');
        if (!actual.stdout.equals(expected) || !reference.stdout.equals(expected)
            || !actual.programStderr.equals(expectedStderr) || !reference.stderr.equals(expectedStderr)
            || ending(actual, false) !== c.ending || ending(reference, true) !== c.ending) {
            fail(c.id + ': output/termination differs: lkjscript=' + ending(actual, false)
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
