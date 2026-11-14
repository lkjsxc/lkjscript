#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { generate } from './generate.mjs';
import { run } from '../process.mjs';
const args = process.argv.slice(2);
const binary = path.resolve(args.shift() || 'target/release/lkjscript');
function option(name, fallback) { const i = args.indexOf(name); return i < 0 ? fallback : args[i + 1]; }
const sizes = option('--sizes', '8000,16000,32000').split(',').map(Number);
const families = option('--families', 'funcs,ifs,match,boxes,vecs').split(',');
const repeats = Number(option('--repeat', '3'));
const verb = option('--verb', 'check');
const engine = option('--engine', 'native');
const out = path.resolve(option('--out', 'target/scale'));
fs.mkdirSync(out, {recursive: true});
const median = a => { const b = [...a].sort((x,y) => x-y); return b[Math.floor(b.length/2)]; };
const context = {repeats, verb, engine};
let failed = false, anyFailed = false;
for (const family of families) {
  let previous;
  for (const n of sizes) {
    const source = path.join(out, `${family}-${n}.rs`);
    const text = generate(family, n); fs.writeFileSync(source, text);
    const samples = [];
    for (let i = 0; i < repeats; i++) {
      const command = [verb, source, '--stats'];
      if (verb === 'run') command.push('--entry','probe','--engine',engine);
      if (verb === 'build') command.push('-o',path.join(out, `${family}-${n}.elf`));
      const start = performance.now();
      const result = await run(binary, command, {timeout:0, sampleMemory:true});
      const ms = performance.now()-start;
      const stages = result.stderr.split('\n').filter(l=>l.startsWith('LKJSCRIPT_STATS ')).map(l=>JSON.parse(l.slice('LKJSCRIPT_STATS '.length)));
      samples.push({ms,sampled_peak_tree_rss_bytes:result.sampled_peak_tree_rss_bytes,stages});
      if (result.error || result.status !== 0) {
        const diagnostic = result.stderr.split('\n').filter(l=>!l.startsWith('LKJSCRIPT_')).join('\n').trim();
        const record = {...context,family,n,source_bytes:Buffer.byteLength(text),outcome:result.error?.code || 'failed',diagnostic,samples};
        console.log(JSON.stringify(record));
        failed = true; anyFailed = true; break;
      }
    }
    if (failed) break;
    const ms = median(samples.map(x=>x.ms));
    const stageNames = new Set(samples.flatMap(x=>x.stages.flatMap(s=>Object.keys(s).filter(ir=>ir.endsWith('_ms')))));
    const stageMedians = Object.fromEntries([...stageNames].map(name=>[name,median(samples.flatMap(x=>x.stages.filter(s=>name in s).map(s=>s[name])))]));
    const record = {...context,family,n,source_bytes:Buffer.byteLength(text),outcome:'ok',median_ms:Number(ms.toFixed(3)),ratio:previous ? Number((ms/previous).toFixed(3)) : null,sampled_peak_tree_rss_bytes:median(samples.map(x=>x.sampled_peak_tree_rss_bytes)),stages:stageMedians};
    if (verb === 'build') record.emitted_bytes = fs.statSync(path.join(out, `${family}-${n}.elf`)).size;
    previous = ms;
    console.log(JSON.stringify(record));
  }
  failed = false;
}

if (anyFailed) process.exitCode = 1;
