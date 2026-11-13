#!/usr/bin/env node
// Compare the production K boundary with B0. --world invokes its native World
// driver using relative inputs and captures the completed ELF from stdout.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { run } from './process.mjs';
const args=process.argv.slice(2), world=args.includes('--world');
const positional=args.filter(arg=>arg!=='--world');
if(positional.length>2||positional.some(arg=>arg.startsWith('--')))throw Error('usage: node tools/kc.mjs [B0] [KC] [--world]');
const binary=path.resolve(positional[0]||'target/release/lkjscript');
const kc=path.resolve(positional[1]||'target/release/kc');
const out=path.resolve(process.env.LKJ_KC_OUT||'target/kc-observation');
fs.mkdirSync(out,{recursive:true});
const casesPath=path.resolve(process.env.LKJ_KC_CASES||'tests/conformance/cases.json');
const cases=JSON.parse(fs.readFileSync(casesPath,'utf8'));
const observations=cases.map(c=>({id:c.id,file:c.file}));
const nativeOptions=world?{cwd:out,memoryBytes:process.env.LKJ_KC_MEMORY_BYTES||'8589934592'}:{};
if(process.env.LKJ_KC_TIMEOUT_MS!==undefined)nativeOptions.timeout=Number(process.env.LKJ_KC_TIMEOUT_MS);
const artifactBuffer=Number(process.env.LKJ_KC_MAX_OUTPUT_BYTES||268435456);
if(!Number.isSafeInteger(artifactBuffer)||artifactBuffer<1)throw Error('LKJ_KC_MAX_OUTPUT_BYTES must be positive bytes');
const runtime=hostResult(await run(binary,['runtime-k'],{encoding:'utf8'}),'runtime preparation');
if(runtime.status!==0)throw Error(runtime.stderr);
const runtimePath=path.join(out,'runtime.k');fs.writeFileSync(runtimePath,runtime.stdout);
const summary={stage:world?'native K compiler (World I/O)':'cargo-built K compiler',verdicts:0,rejections:0,artifacts:0,pending:0,failures:[],
 compilers:{b0:fileRecord(binary),kc:fileRecord(kc)},manifest:fileRecord(casesPath),runtime:fileRecord(runtimePath),world};
const started=performance.now();let next=0;
function diagnostic(result){return (result.stderr||'').match(/\bE_[A-Z_]+\b/)?.[0]||String(result.status);}
function fileRecord(file){const bytes=fs.readFileSync(file);return {path:file,bytes:bytes.length,sha256:crypto.createHash('sha256').update(bytes).digest('hex')};}
function hostResult(result,context){
 if(result.error||result.signal||result.status===null)throw Error(context+': host failure '+String(result.error||result.signal||'no exit status'));
 return result;
}
function verdict(result){return {accepted:result.status===0,status:result.status,diagnostic:result.status===0?null:diagnostic(result)};}
function inputPath(file){return world?path.basename(file):file;}
async function checkK(file){return hostResult(await run(kc,[inputPath(file),'--check'],{...nativeOptions,encoding:'utf8'}),path.basename(file)+' K check');}
async function emitK(file,options,actual){
 fs.rmSync(actual,{force:true});
 const args=[inputPath(file),'--runtime',inputPath(runtimePath),...options];
 const result=hostResult(await run(kc,world?args:[...args,'-o',actual],{...nativeOptions,encoding:world?null:'utf8',maxBuffer:artifactBuffer}),path.basename(file)+' K emission');
 if(world&&result.status===0&&!result.error)fs.writeFileSync(actual,result.stdout);
 return result;
}
async function observe(c,observation){
 const source=path.resolve('tests/conformance',c.file), kfile=path.join(out,c.id+'.k');
 observation.source=fileRecord(source);
 if(c.file.endsWith('.k')){
  fs.copyFileSync(source,kfile);
  observation.input=fileRecord(kfile);
  const a=hostResult(await run(binary,['check',source],{encoding:'utf8'}),c.id+' B0 check');
  const b=await checkK(kfile);
  observation.verdicts={b0:verdict(a),kc:verdict(b)};
  summary.verdicts++;
  if((a.status===0)!==(b.status===0)||(a.status!==0&&diagnostic(a)!==diagnostic(b)))throw Error(c.id+': K verdict differs '+a.stderr+' / '+b.stderr);
  if(a.status!==0){summary.rejections++;observation.outcome='rejected';return;}
 }else{
  const result=hostResult(await run(binary,['k',source],{encoding:'utf8',maxBuffer:32*1024*1024}),c.id+' source compilation');
  observation.source_verdict=verdict(result);
  if(result.status!==0){if(c.kind==='reject'){observation.outcome='source rejected';return;}if(c.pending){summary.pending++;observation.outcome='pending';return;}throw Error(c.id+': source did not produce K: '+result.stderr);}
  fs.writeFileSync(kfile,result.stdout);
  observation.input=fileRecord(kfile);
  const accepted=await checkK(kfile);summary.verdicts++;
  observation.verdicts={b0:verdict(result),kc:verdict(accepted)};
  if(accepted.status!==0)throw Error(c.id+': generated K was rejected: '+accepted.stderr);
 }
 const options=c.entry?['--entry',c.entry]:[];
 const reference=path.join(out,c.id+'-b0.elf'),actual=path.join(out,c.id+'-kc0.elf');
 fs.rmSync(reference,{force:true});
 const a=hostResult(await run(binary,['build',source,...options,'-o',reference],{encoding:'utf8'}),c.id+' B0 emission');
 const b=await emitK(kfile,options,actual);
 if(a.status!==0||b.status!==0||a.error||b.error)throw Error(c.id+': artifact build failed '+a.stderr+' / '+b.stderr+' '+String(a.error||b.error||''));
 observation.artifacts={b0:fileRecord(reference),kc:fileRecord(actual)};
 observation.identical=fs.readFileSync(reference).equals(fs.readFileSync(actual));
 if(!observation.identical)throw Error(c.id+': native bytes differ');
 observation.outcome='accepted';summary.artifacts++;
}
const jobs=Number(process.env.LKJ_KC_JOBS||4);if(!Number.isSafeInteger(jobs)||jobs<1)throw Error('LKJ_KC_JOBS must be positive');
await Promise.all(Array.from({length:jobs},async()=>{for(;;){const index=next++;if(index>=cases.length)return;try{await observe(cases[index],observations[index]);}catch(error){observations[index].outcome='failure';observations[index].failure=String(error);summary.failures.push(String(error));}}}));
summary.elapsedSeconds=(performance.now()-started)/1000;
summary.observations=path.join(out,'observations.json');
fs.writeFileSync(summary.observations,JSON.stringify(observations,null,2)+'\n');
fs.writeFileSync(path.join(out,'summary.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary));if(summary.failures.length)process.exitCode=1;
