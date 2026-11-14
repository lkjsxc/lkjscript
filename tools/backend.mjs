#!/usr/bin/env node
// Compare the production IR boundary with B0. --world invokes its native World
// driver using relative inputs and captures the completed ELF from stdout.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { run } from './process.mjs';
const args=process.argv.slice(2), world=args.includes('--world');
const positional=args.filter(arg=>arg!=='--world');
if(positional.length>2||positional.some(arg=>arg.startsWith('--')))throw Error('usage: node tools/backend.mjs [B0] [BACKEND] [--world]');
const binary=path.resolve(positional[0]||'target/release/lkjscript');
const backend=path.resolve(positional[1]||'target/release/lkjscript-backend');
const out=path.resolve(process.env.LKJSCRIPT_BACKEND_OUT||'target/backend-observation');
fs.mkdirSync(out,{recursive:true});
const casesPath=path.resolve(process.env.LKJSCRIPT_BACKEND_CASES||'tests/conformance/cases.json');
const cases=JSON.parse(fs.readFileSync(casesPath,'utf8'));
const observations=cases.map(c=>({id:c.id,file:c.file}));
const nativeOptions=world?{cwd:out,memoryBytes:process.env.LKJSCRIPT_BACKEND_MEMORY_BYTES||'8589934592'}:{};
if(process.env.LKJSCRIPT_BACKEND_TIMEOUT_MS!==undefined)nativeOptions.timeout=Number(process.env.LKJSCRIPT_BACKEND_TIMEOUT_MS);
const artifactBuffer=Number(process.env.LKJSCRIPT_BACKEND_MAX_OUTPUT_BYTES||268435456);
if(!Number.isSafeInteger(artifactBuffer)||artifactBuffer<1)throw Error('LKJSCRIPT_BACKEND_MAX_OUTPUT_BYTES must be positive bytes');
const runtime=hostResult(await run(binary,['runtime-ir'],{encoding:'utf8'}),'runtime preparation');
if(runtime.status!==0)throw Error(runtime.stderr);
const runtimePath=path.join(out,'runtime.lkjir');fs.writeFileSync(runtimePath,runtime.stdout);
const summary={stage:world?'native backend (World I/O)':'cargo-built backend',verdicts:0,rejections:0,artifacts:0,pending:0,failures:[],
 compilers:{b0:fileRecord(binary),backend:fileRecord(backend)},manifest:fileRecord(casesPath),runtime:fileRecord(runtimePath),world};
const started=performance.now();let next=0;
function diagnostic(result){return (result.stderr||'').match(/\bE_[A-Z_]+\b/)?.[0]||String(result.status);}
function fileRecord(file){const bytes=fs.readFileSync(file);return {path:file,bytes:bytes.length,sha256:crypto.createHash('sha256').update(bytes).digest('hex')};}
function hostResult(result,context){
 if(result.error||result.signal||result.status===null)throw Error(context+': host failure '+String(result.error||result.signal||'no exit status'));
 return result;
}
function verdict(result){return {accepted:result.status===0,status:result.status,diagnostic:result.status===0?null:diagnostic(result)};}
function inputPath(file){return world?path.basename(file):file;}
async function checkIr(file){return hostResult(await run(backend,[inputPath(file),'--check'],{...nativeOptions,encoding:'utf8'}),path.basename(file)+' IR check');}
async function emitIr(file,options,actual){
 fs.rmSync(actual,{force:true});
 const args=[inputPath(file),'--runtime',inputPath(runtimePath),...options];
 const result=hostResult(await run(backend,world?args:[...args,'-o',actual],{...nativeOptions,encoding:world?null:'utf8',maxBuffer:artifactBuffer}),path.basename(file)+' IR emission');
 if(world&&result.status===0&&!result.error)fs.writeFileSync(actual,result.stdout);
 return result;
}
async function observe(c,observation){
 const source=path.resolve('tests/conformance',c.file), irFile=path.join(out,c.id+'.lkjir');
 observation.source=fileRecord(source);
 if(c.file.endsWith('.lkjir')){
  fs.copyFileSync(source,irFile);
  observation.input=fileRecord(irFile);
  const a=hostResult(await run(binary,['check',source],{encoding:'utf8'}),c.id+' B0 check');
  const b=await checkIr(irFile);
  observation.verdicts={b0:verdict(a),backend:verdict(b)};
  summary.verdicts++;
  if((a.status===0)!==(b.status===0)||(a.status!==0&&diagnostic(a)!==diagnostic(b)))throw Error(c.id+': IR verdict differs '+a.stderr+' / '+b.stderr);
  if(a.status!==0){summary.rejections++;observation.outcome='rejected';return;}
 }else{
  const result=hostResult(await run(binary,['ir',source],{encoding:'utf8',maxBuffer:32*1024*1024}),c.id+' source compilation');
  observation.source_verdict=verdict(result);
  if(result.status!==0){if(c.kind==='reject'){observation.outcome='source rejected';return;}if(c.pending){summary.pending++;observation.outcome='pending';return;}throw Error(c.id+': source did not produce IR: '+result.stderr);}
  fs.writeFileSync(irFile,result.stdout);
  observation.input=fileRecord(irFile);
  const accepted=await checkIr(irFile);summary.verdicts++;
  observation.verdicts={b0:verdict(result),backend:verdict(accepted)};
  if(accepted.status!==0)throw Error(c.id+': generated IR was rejected: '+accepted.stderr);
 }
 const options=c.entry?['--entry',c.entry]:[];
 const reference=path.join(out,c.id+'-b0.elf'),actual=path.join(out,c.id+'-backend0.elf');
 fs.rmSync(reference,{force:true});
 const a=hostResult(await run(binary,['build',source,...options,'-o',reference],{encoding:'utf8'}),c.id+' B0 emission');
 const b=await emitIr(irFile,options,actual);
 if(a.status!==0||b.status!==0||a.error||b.error)throw Error(c.id+': artifact build failed '+a.stderr+' / '+b.stderr+' '+String(a.error||b.error||''));
 observation.artifacts={b0:fileRecord(reference),backend:fileRecord(actual)};
 observation.identical=fs.readFileSync(reference).equals(fs.readFileSync(actual));
 if(!observation.identical)throw Error(c.id+': native bytes differ');
 observation.outcome='accepted';summary.artifacts++;
}
const jobs=Number(process.env.LKJSCRIPT_BACKEND_JOBS||4);if(!Number.isSafeInteger(jobs)||jobs<1)throw Error('LKJSCRIPT_BACKEND_JOBS must be positive');
await Promise.all(Array.from({length:jobs},async()=>{for(;;){const index=next++;if(index>=cases.length)return;try{await observe(cases[index],observations[index]);}catch(error){observations[index].outcome='failure';observations[index].failure=String(error);summary.failures.push(String(error));}}}));
summary.elapsedSeconds=(performance.now()-started)/1000;
summary.observations=path.join(out,'observations.json');
fs.writeFileSync(summary.observations,JSON.stringify(observations,null,2)+'\n');
fs.writeFileSync(path.join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary));if(summary.failures.length)process.exitCode=1;
