#!/usr/bin/env node
// Outside-review programs are independent examples, not a language definition.
import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';
const binary = path.resolve(process.argv[2] || 'target/debug/lkjscript');
const suites = process.argv.slice(3);
if (!suites.length) suites.push('idioms','ownership','rejected');
const out = path.resolve('target/corpus'); fs.mkdirSync(out,{recursive:true});
let fatal = false;
for (const suite of suites) {
  const directory = path.resolve('tests/corpus',suite);
  const cases = fs.readdirSync(directory).filter(f=>f.endsWith('.rs')).sort();
  const tally = {}, details = [];
  for (const file of cases) {
    const source = path.join(directory,file);
    const rustSource = path.join(out,'oracle.rs'), oracle = path.join(out,'oracle');
    fs.writeFileSync(rustSource,fs.readFileSync(source,'utf8')+'\nfn main() { println!("{}", probe()); }\n');
    const compiled = await run('rustc',['--edition=2024','-A','warnings','-C','overflow-checks=yes',rustSource,'-o',oracle]);
    const checked = await run(binary,['check',source]);
    let category, reference, actual;
    if (compiled.error || compiled.signal || checked.error || checked.signal) {
      category = [compiled,checked].some(r=>r.error?.code==='ETIMEDOUT') ? 'timeout' : 'crash';
    } else if (suite === 'rejected') {
      category = compiled.status === 0 ? 'oracle-accepted' : checked.status === 0 ? 'accepted-rust-rejected' : 'both-rejected';
    } else if (compiled.status !== 0) category = 'oracle-rejected';
    else if (checked.status !== 0) category = 'valid-but-rejected:'+(checked.stderr.match(/\bE_[A-Z_]+\b/)?.[0] || 'unknown');
    else {
      reference = await run(oracle,[]); actual = await run(binary,['run',source,'--entry','probe']);
      if (reference.error || actual.error || reference.signal || actual.signal) category = [reference,actual].some(r=>r.error?.code==='ETIMEDOUT') ? 'timeout' : 'crash';
      else if (reference.status === 0 && actual.status === 0 && reference.stdout === actual.stdout) category = 'equal-output-and-exit';
      else category = 'output-or-exit-differs';
    }
    tally[category]=(tally[category]||0)+1;
    if (category !== 'equal-output-and-exit' && category !== 'both-rejected') details.push({file,category,diagnostic:checked.stderr.trim(),rust:reference?.stdout,lkj:actual?.stdout,execution:actual?.stderr});
    if (!category.startsWith('valid-but-rejected') && category !== 'equal-output-and-exit' && category !== 'both-rejected') fatal=true;
  }
  const record={suite,total:cases.length,tally,details};
  fs.writeFileSync(path.join(out,`${suite}.json`),JSON.stringify(record,null,2)+'\n');
  console.log(JSON.stringify({suite,total:cases.length,tally}));
}
if (fatal) process.exitCode=1;
