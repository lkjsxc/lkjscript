#!/usr/bin/env node
// Observe the independent cargo-built K boundary. This is not self-compilation.
import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';
const binary=path.resolve(process.argv[2]||'target/release/lkjscript');
const kc=path.resolve(process.argv[3]||'target/release/kc');
const out=path.resolve(process.env.LKJ_KC_OUT||'target/kc-observation');
fs.mkdirSync(out,{recursive:true});
const cases=JSON.parse(fs.readFileSync('tests/conformance/cases.json','utf8'));
const runtime=await run(binary,['runtime-k'],{encoding:'utf8'});
if(runtime.status!==0)throw Error(runtime.stderr);
const runtimePath=path.join(out,'runtime.k');fs.writeFileSync(runtimePath,runtime.stdout);
const summary={stage:'cargo-built K compiler',verdicts:0,rejections:0,artifacts:0,pending:0,failures:[]};
const started=performance.now();let next=0;
function diagnostic(result){return (result.stderr||'').match(/\bE_[A-Z_]+\b/)?.[0]||String(result.status);}
async function observe(c){
 const source=path.resolve('tests/conformance',c.file), kfile=path.join(out,c.id+'.k');
 if(c.file.endsWith('.k')){
  const a=await run(binary,['check',source],{encoding:'utf8'}), b=await run(kc,[source,'--check'],{encoding:'utf8'});
  summary.verdicts++;
  if((a.status===0)!==(b.status===0)||(a.status!==0&&diagnostic(a)!==diagnostic(b)))throw Error(c.id+': K verdict differs '+a.stderr+' / '+b.stderr);
  if(a.status!==0){summary.rejections++;return;}
  fs.copyFileSync(source,kfile);
 }else{
  const result=await run(binary,['k',source],{encoding:'utf8',maxBuffer:32*1024*1024});
  if(result.status!==0){if(c.kind==='reject')return;if(c.pending){summary.pending++;return;}throw Error(c.id+': source did not produce K: '+result.stderr);}
  fs.writeFileSync(kfile,result.stdout);
  const accepted=await run(kc,[kfile,'--check'],{encoding:'utf8'});summary.verdicts++;
  if(accepted.status!==0)throw Error(c.id+': generated K was rejected: '+accepted.stderr);
 }
 const options=c.entry?['--entry',c.entry]:[];
 const reference=path.join(out,c.id+'-b0.elf'),actual=path.join(out,c.id+'-kc0.elf');
 const a=await run(binary,['build',source,...options,'-o',reference],{encoding:'utf8'});
 const b=await run(kc,[kfile,'--runtime',runtimePath,...options,'-o',actual],{encoding:'utf8'});
 if(a.status!==0||b.status!==0)throw Error(c.id+': artifact build failed '+a.stderr+' / '+b.stderr);
 if(!fs.readFileSync(reference).equals(fs.readFileSync(actual)))throw Error(c.id+': native bytes differ');
 summary.artifacts++;
}
const jobs=Number(process.env.LKJ_KC_JOBS||4);if(!Number.isSafeInteger(jobs)||jobs<1)throw Error('LKJ_KC_JOBS must be positive');
await Promise.all(Array.from({length:jobs},async()=>{for(;;){const index=next++;if(index>=cases.length)return;try{await observe(cases[index]);}catch(error){summary.failures.push(String(error));}}}));
summary.elapsedSeconds=(performance.now()-started)/1000;
fs.writeFileSync(path.join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary));if(summary.failures.length)process.exitCode=1;
