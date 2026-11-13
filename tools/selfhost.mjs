#!/usr/bin/env node
// Observe the production K compiler's fixed point on one declared K/runtime pair.
// Resource envelopes are operator choices and never language acceptance limits.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { run } from './process.mjs';

const repository=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const argv=process.argv.slice(2);
if(argv.includes('--help')){
 console.log('usage: node tools/selfhost.mjs [B0] [CARGO_KC] [--prepare-only]\n'+
  'LKJ_SELFHOST_OUT selects the observation directory (default target/selfhost-observation).\n'+
  'LKJ_SELFHOST_MEMORY_BYTES defaults to 8589934592; unlimited inherits Linux controls.\n'+
  'LKJ_SELFHOST_TIMEOUT_MS defaults to 600000 per command; 0 disables the deadline.\n'+
  'LKJ_SELFHOST_MAX_OUTPUT_BYTES defaults to 268435456 for captured compiler output.');
 process.exit(0);
}
const prepareOnly=argv.includes('--prepare-only');
const positional=argv.filter(value=>value!=='--prepare-only');
if(positional.length>2||positional.some(value=>value.startsWith('--')))throw Error('unexpected argument; see --help');
const binary=path.resolve(positional[0]||path.join(repository,'target/release/lkjscript'));
const cargoKc=path.resolve(positional[1]||path.join(repository,'target/release/kc'));
const out=path.resolve(process.env.LKJ_SELFHOST_OUT||path.join(repository,'target/selfhost-observation'));
function numberSetting(name,fallback,minimum){
 const value=Number(process.env[name]??fallback);
 if(!Number.isSafeInteger(value)||value<minimum)throw Error(name+' must be an integer >= '+minimum);
 return value;
}
const controls={
 memory_bytes:process.env.LKJ_SELFHOST_MEMORY_BYTES||'8589934592',
 timeout_ms:numberSetting('LKJ_SELFHOST_TIMEOUT_MS',600000,0),
 max_output_bytes:numberSetting('LKJ_SELFHOST_MAX_OUTPUT_BYTES',268435456,1),
};
fs.mkdirSync(out,{recursive:true});
const digest=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function fileRecord(filename){const bytes=fs.readFileSync(filename);return {path:filename,bytes:bytes.length,sha256:digest(bytes)};}
function gitSnapshot(){
 const options={cwd:repository,encoding:'utf8',maxBuffer:controls.max_output_bytes};
 const commit=execFileSync('git',['rev-parse','HEAD'],options).trim();
 const status=execFileSync('git',['status','--porcelain=v1'],options);
 const diff=execFileSync('git',['diff','--binary','HEAD'],{...options,encoding:null});
 return {commit,dirty:status.length!==0,status,diff_sha256:digest(diff)};
}
const start=performance.now();
const summary={
 stage:'production K compiler fixed point',status:'running',started_at:new Date().toISOString(),
 controls,git_start:gitSnapshot(),harness:fileRecord(fileURLToPath(import.meta.url)),compilers:{b0:fileRecord(binary),cargo_kc:fileRecord(cargoKc)},
 inputs:{},commands:[],comparisons:[],artifacts:{},failures:[],
};
const summaryPath=path.join(out,'summary.json');
function save(){summary.elapsed_ms=performance.now()-start;fs.writeFileSync(summaryPath,JSON.stringify(summary,null,2)+'\n');}
function parseTiming(filename){
 if(!fs.existsSync(filename))return null;
 const line=fs.readFileSync(filename,'utf8').trim().split('\n').findLast(value=>/^\d+(\.\d+)?\t\d+\t\d+$/.test(value));
 if(!line)return null;
 const [seconds,rss,exit]=line.split('\t').map(Number);
 return {elapsed_seconds:seconds,max_rss_bytes:rss*1024,exit_status:exit};
}
async function measure(name,executable,args,{cwd=repository,stdout,artifact}={}){
 const timePath=path.join(out,name+'.time.tsv');
 const stderrPath=path.join(out,name+'.stderr');
 for(const filename of [timePath,stdout,artifact])if(filename)fs.rmSync(filename,{force:true});
 console.log('selfhost: '+name);
 const began=performance.now();
 const result=await run('/usr/bin/time',['-f','%e\t%M\t%x','-o',timePath,'--',executable,...args],{
  cwd,encoding:null,memoryBytes:controls.memory_bytes,timeout:controls.timeout_ms,maxBuffer:controls.max_output_bytes,
 });
 fs.writeFileSync(stderrPath,result.stderr);
 if(stdout)fs.writeFileSync(stdout,result.stdout);
 const observation={name,executable,args,cwd,status:result.status,signal:result.signal,
  error:result.error?String(result.error):null,wall_ms:performance.now()-began,
  timing:parseTiming(timePath),stderr:stderrPath,stdout_bytes:result.stdout.length};
 if(stdout)observation.stdout=fileRecord(stdout);
 if(artifact&&fs.existsSync(artifact))observation.artifact=fileRecord(artifact);
 summary.commands.push(observation);save();
 if(result.status!==0||result.error)throw Error(name+' failed: '+String(result.error||result.stderr.toString('utf8')||result.signal||result.status));
 if(!observation.timing)throw Error(name+' has no completed /usr/bin/time observation');
 return result;
}
function elf(filename){
 const bytes=fs.readFileSync(filename);
 if(bytes.length<4||!bytes.subarray(0,4).equals(Buffer.from([0x7f,0x45,0x4c,0x46])))throw Error(filename+' is not ELF output');
 fs.chmodSync(filename,0o755);
 return fileRecord(filename);
}
function compare(name,left,right){
 const identical=fs.readFileSync(left).equals(fs.readFileSync(right));
 summary.comparisons.push({name,left,right,identical});save();
 if(!identical)throw Error(name+': ELF bytes differ');
}
try{
 const input=path.join(out,'kc.k'),runtime=path.join(out,'runtime.k');
 await measure('prepare_k',binary,['k','src/kc_main.rs'],{stdout:input});
 await measure('prepare_runtime',binary,['runtime-k'],{stdout:runtime});
 summary.inputs={program:fileRecord(input),runtime:fileRecord(runtime)};save();
 if(prepareOnly){
  summary.status='prepared';
 }else{
  const b0=path.join(out,'b0.elf'),kc1=path.join(out,'kc1.elf'),kc2=path.join(out,'kc2.elf'),kc3=path.join(out,'kc3.elf');
  await measure('b0_emit',binary,['build',input,'--entry','main','-o',b0],{artifact:b0});
  summary.artifacts.b0=elf(b0);
  await measure('cargo_emit',cargoKc,[input,'--runtime',runtime,'--entry','main','-o',kc1],{artifact:kc1});
  summary.artifacts.kc1=elf(kc1);
  compare('B0 versus Cargo K compiler',b0,kc1);
  const args=['kc.k','--runtime','runtime.k','--entry','main'];
  await measure('kc1_emit_kc2',kc1,args,{cwd:out,stdout:kc2});
  summary.artifacts.kc2=elf(kc2);
  compare('kc1 versus kc2',kc1,kc2);
  await measure('kc2_emit_kc3',kc2,args,{cwd:out,stdout:kc3});
  summary.artifacts.kc3=elf(kc3);
  compare('kc2 versus kc3',kc2,kc3);
  summary.status='fixed point';
 }
}catch(error){
 summary.failures.push(String(error));summary.status='failed';process.exitCode=1;
}finally{
 summary.git_end=gitSnapshot();summary.finished_at=new Date().toISOString();save();
 console.log(JSON.stringify({status:summary.status,summary:summaryPath,comparisons:summary.comparisons,failures:summary.failures}));
}
