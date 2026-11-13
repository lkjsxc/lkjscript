#!/usr/bin/env node
// Outside-review programs are independent examples, not a language definition.
import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';
import { runCases } from './batch.mjs';
import { oracleCache,packResult,unpackResult } from './oracle.mjs';
const binary=path.resolve(process.argv[2]||'target/debug/lkjscript');
const engine=process.env.LKJ_ENGINE || 'native';
const suites=process.argv.slice(3);if(!suites.length)suites.push('idioms','ownership','rejected');
const out=path.resolve('target/corpus');fs.mkdirSync(out,{recursive:true});
let fatal=false;
for(const suite of suites) {
 const directory=path.resolve('tests/corpus',suite),cases=fs.readdirSync(directory).filter(f=>f.endsWith('.rs')).sort();
 const sources=cases.map(file=>fs.readFileSync(path.join(directory,file),'utf8'));
 const flags=['--edition=2024','-A','warnings','-C','overflow-checks=yes'];
 const references=await oracleCache({suite:'corpus/'+suite,sources,flags},async()=>{
  const results=[];
  for(const source of sources){const input=path.join(out,'oracle.rs'),binary=path.join(out,'oracle');fs.writeFileSync(input,source+'\nfn main(){println!("{}",probe());}\n');
   const compiled=await run('rustc',[...flags,input,'-o',binary]);
   results.push({compiled:packResult(compiled),run:compiled.status===0&&suite!=='rejected'?packResult(await run(binary,[])):null});
  }return results;
 });
 const results=await runCases(binary,cases.map(file=>({path:path.join(directory,file),entry:'probe',mode:suite==='rejected'?'check':'run'})),{engine,output:out});
 const tally={},details=[];
 for(let i=0;i<cases.length;i++) {
  const compiled=unpackResult(references[i].compiled),actual=results[i],checked={...actual,status:actual.checkStatus};
  const reference=references[i].run?unpackResult(references[i].run):null;let category;
  if(compiled.error||compiled.signal||checked.error||checked.signal)category=[compiled,checked].some(r=>r.error?.code==='ETIMEDOUT')?'timeout':'crash';
  else if(suite==='rejected')category=compiled.status===0?'oracle-accepted':checked.status===0?'accepted-rust-rejected':'both-rejected';
  else if(compiled.status!==0)category='oracle-rejected';
  else if(checked.status!==0)category='valid-but-rejected:'+(checked.stderr.match(/\bE_[A-Z_]+\b/)?.[0]||'unknown');
  else if(reference.error||actual.error||reference.signal||actual.signal)category=[reference,actual].some(r=>r.error?.code==='ETIMEDOUT')?'timeout':'crash';
  else if(reference.status===0&&actual.status===0&&reference.stdout===actual.stdout)category='equal-output-and-exit';
  else category='output-or-exit-differs';
  tally[category]=(tally[category]||0)+1;
  if(category!=='equal-output-and-exit'&&category!=='both-rejected')details.push({file:cases[i],category,diagnostic:checked.stderr.trim(),rust:reference?.stdout,lkj:actual.stdout,execution:actual.stderr});
  if(!category.startsWith('valid-but-rejected')&&category!=='equal-output-and-exit'&&category!=='both-rejected')fatal=true;
 }
 const record={suite,engine,total:cases.length,tally,details};fs.writeFileSync(path.join(out,suite+'-'+engine+'.json'),JSON.stringify(record,null,2)+'\n');console.log(JSON.stringify({suite,engine,total:cases.length,tally}));
}
if(fatal)process.exitCode=1;
