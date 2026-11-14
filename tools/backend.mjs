#!/usr/bin/env node
// Compare the production IR boundary with B0. --world invokes its native World
// driver using relative inputs and captures the completed ELF from stdout.
import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';
const args=process.argv.slice(2), world=args.includes('--world');
const positional=args.filter(arg=>arg!=='--world');
if(positional.length>2||positional.some(arg=>arg.startsWith('--')))throw Error('usage: node tools/backend.mjs [B0] [BACKEND] [--world]');
const binary=path.resolve(positional[0]||'target/release/lkjscript');
const backend=path.resolve(positional[1]||'target/release/lkjscript-backend');
const out=path.resolve(process.env.LKJSCRIPT_BACKEND_OUT||'target/backend');
fs.mkdirSync(out,{recursive:true});
const casesPath=path.resolve(process.env.LKJSCRIPT_BACKEND_CASES||'tests/conformance/cases.json');
const cases=JSON.parse(fs.readFileSync(casesPath,'utf8'));
const nativeOptions=world?{cwd:out}:{};
const runtime=hostResult(await run(binary,['runtime-ir'],{encoding:'utf8'}),'runtime preparation');
if(runtime.status!==0)throw Error(runtime.stderr);
const runtimePath=path.join(out,'runtime.lkjir');fs.writeFileSync(runtimePath,runtime.stdout);
const summary={stage:world?'native backend (World I/O)':'cargo-built backend',verdicts:0,rejections:0,artifacts:0,pending:0,failures:0};
const started=performance.now();let next=0;
function diagnostic(result){return (result.stderr||'').match(/\bE_[A-Z_]+\b/)?.[0]||String(result.status);}
function hostResult(result,context){
 if(result.error||result.signal||result.status===null)throw Error(context+': host failure '+String(result.error||result.signal||'no exit status'));
 return result;
}
function inputPath(file){return world?path.basename(file):file;}
async function checkIr(file){return hostResult(await run(backend,[inputPath(file),'--check'],{...nativeOptions,encoding:'utf8'}),path.basename(file)+' IR check');}
async function emitIr(file,options,actual){
 fs.rmSync(actual,{force:true});
 const args=[inputPath(file),'--runtime',inputPath(runtimePath),...options];
 const result=hostResult(await run(backend,world?args:[...args,'-o',actual],{...nativeOptions,encoding:world?null:'utf8'}),path.basename(file)+' IR emission');
 if(world&&result.status===0&&!result.error)fs.writeFileSync(actual,result.stdout);
 return result;
}
async function checkCase(c){
 const source=path.resolve('tests/conformance',c.file), irFile=path.join(out,c.id+'.lkjir');
 if(c.file.endsWith('.lkjir')){
  fs.copyFileSync(source,irFile);
  const a=hostResult(await run(binary,['check',source],{encoding:'utf8'}),c.id+' B0 check');
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
 const reference=path.join(out,c.id+'-b0.elf'),actual=path.join(out,c.id+'-backend0.elf');
 fs.rmSync(reference,{force:true});
 const a=hostResult(await run(binary,['build',source,...options,'-o',reference],{encoding:'utf8'}),c.id+' B0 emission');
 const b=await emitIr(irFile,options,actual);
 if(a.status!==0||b.status!==0||a.error||b.error)throw Error(c.id+': artifact build failed '+a.stderr+' / '+b.stderr+' '+String(a.error||b.error||''));
 if(!fs.readFileSync(reference).equals(fs.readFileSync(actual)))throw Error(c.id+': native bytes differ');
 summary.artifacts++;
}
const jobs=Number(process.env.LKJSCRIPT_BACKEND_JOBS||4);if(!Number.isSafeInteger(jobs)||jobs<1)throw Error('LKJSCRIPT_BACKEND_JOBS must be positive');
await Promise.all(Array.from({length:jobs},async()=>{for(;;){const index=next++;if(index>=cases.length)return;try{await checkCase(cases[index]);}catch(error){console.error(String(error));summary.failures++;}}}));
summary.elapsedSeconds=(performance.now()-started)/1000;
console.log(JSON.stringify(summary));if(summary.failures)process.exitCode=1;
