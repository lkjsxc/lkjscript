#!/usr/bin/env node
// Check serialized IR and compare its emitted bytes with direct source builds.
import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';
const args=process.argv.slice(2);
if(args.length>1||args.some(arg=>arg.startsWith('--')))throw Error('usage: node tools/backend.mjs [COMPILER]');
const binary=path.resolve(args[0]||process.env.LKJSCRIPT_BIN||'target/release/lkjscript');
const out=path.resolve(process.env.LKJSCRIPT_BACKEND_OUT||'target/backend');
fs.mkdirSync(out,{recursive:true});
const casesPath=path.resolve(process.env.LKJSCRIPT_BACKEND_CASES||'tests/conformance/cases.json');
const cases=JSON.parse(fs.readFileSync(casesPath,'utf8'));
const summary={stage:'source/IR boundary',verdicts:0,rejections:0,artifacts:0,pending:0,failures:0};
const started=performance.now();let next=0;
function diagnostic(result){return (result.stderr||'').match(/\bE_[A-Z_]+\b/)?.[0]||String(result.status);}
function hostResult(result,context){
 if(result.error||result.signal||result.status===null)throw Error(context+': host failure '+String(result.error||result.signal||'no exit status'));
 return result;
}
async function checkIr(file){return hostResult(await run(binary,['check',file],{encoding:'utf8'}),path.basename(file)+' IR check');}
async function emitIr(file,options,actual){
 fs.rmSync(actual,{force:true});
 return hostResult(await run(binary,['build',file,...options,'-o',actual],{encoding:'utf8'}),path.basename(file)+' IR emission');
}
async function checkCase(c){
 const source=path.resolve('tests/conformance',c.file), irFile=path.join(out,c.id+'.lkjir');
 if(c.file.endsWith('.lkjir')){
  fs.copyFileSync(source,irFile);
  const a=hostResult(await run(binary,['check',source],{encoding:'utf8'}),c.id+' original IR check');
  const b=await checkIr(irFile);
  summary.verdicts++;
  if((a.status===0)!==(b.status===0)||(a.status!==0&&diagnostic(a)!==diagnostic(b)))throw Error(c.id+': IR verdict differs '+a.stderr+' / '+b.stderr);
  if(a.status!==0){summary.rejections++;return;}
 }else{
  const result=hostResult(await run(binary,['ir',source],{encoding:'utf8'}),c.id+' source compilation');
  if(result.status!==0){if(c.kind==='reject'){return;}if(c.pending){summary.pending++;return;}throw Error(c.id+': source did not produce IR: '+result.stderr);}
  fs.writeFileSync(irFile,result.stdout);
  const accepted=await checkIr(irFile);summary.verdicts++;
  if(accepted.status!==0)throw Error(c.id+': generated IR was rejected: '+accepted.stderr);
 }
 const options=c.entry?['--entry',c.entry]:[];
 const reference=path.join(out,c.id+'-source.elf'),actual=path.join(out,c.id+'-ir.elf');
 fs.rmSync(reference,{force:true});
 const a=hostResult(await run(binary,['build',source,...options,'-o',reference],{encoding:'utf8'}),c.id+' source emission');
 const b=await emitIr(irFile,options,actual);
 if(a.status!==0||b.status!==0)throw Error(c.id+': artifact build failed '+a.stderr+' / '+b.stderr);
 if(!fs.readFileSync(reference).equals(fs.readFileSync(actual)))throw Error(c.id+': source/IR native bytes differ');
 summary.artifacts++;
}
const jobs=Number(process.env.LKJSCRIPT_BACKEND_JOBS||4);if(!Number.isSafeInteger(jobs)||jobs<1)throw Error('LKJSCRIPT_BACKEND_JOBS must be positive');
await Promise.all(Array.from({length:jobs},async()=>{for(;;){const index=next++;if(index>=cases.length)return;try{await checkCase(cases[index]);}catch(error){console.error(String(error));summary.failures++;}}}));
summary.elapsedSeconds=(performance.now()-started)/1000;
console.log(JSON.stringify(summary));if(summary.failures)process.exitCode=1;
