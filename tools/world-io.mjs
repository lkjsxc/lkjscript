#!/usr/bin/env node
// Execute source data with explicit bytes, directories and Linux budgets.
import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';
import { runRoutes } from './batch.mjs';
import { worldOutcome } from './oracle.mjs';
const root=path.resolve('.'), binary=path.resolve(process.argv[2]||'target/release/lkjscript');
const kc=path.resolve(process.argv[3]||path.join(path.dirname(binary),'kc'));
const output=path.join(root,'target/world-io');fs.mkdirSync(output,{recursive:true});
const started=performance.now(), summary={observations:0,failures:[]};
const host=fs.readFileSync('bridge/output.rs','utf8');
const launch=path.join(output,'launch');
function detail(r){return `${r.status}/${r.signal}: ${r.stderr?.toString()} ${r.error||''}`;}
async function build(command,args){const r=await run(command,args);if(r.status!==0||r.error)throw Error(detail(r));}
await build('rustc',['--edition=2024','tests/fixtures/world/launch.rs','-o',launch]);
const runtime=await run(binary,['runtime-k']);if(runtime.status!==0||runtime.error)throw Error(detail(runtime));
const runtimePath=path.join(output,'runtime.k');fs.writeFileSync(runtimePath,runtime.stdout);
async function compile(name){
 const source=path.join(root,'tests/fixtures/world',name+'.rs');
 const rust=path.join(output,name+'-rust.rs'), oracle=path.join(output,name+'-rust'), native=path.join(output,name+'.elf');
 fs.writeFileSync(rust,'#![allow(warnings)]\nmod host {\n'+host+'\n}\nmod case {\nuse crate::host::{World,Stdout,Stdin,Stderr,Directory,File};\n'+fs.readFileSync(source,'utf8')+'\n}\nfn main(){host::enter(case::main);}\n');
 await build('rustc',['--edition=2024','--cfg','lkj_control','-C','overflow-checks=yes','-C','opt-level=1',rust,'-o',oracle]);
 await build(binary,['build',source,'-o',native]);
 const k=await run(binary,['k',source]);if(k.status!==0||k.error)throw Error(detail(k));
 const kpath=path.join(output,name+'.k'), kelf=path.join(output,name+'-kc.elf');fs.writeFileSync(kpath,k.stdout);
 await build(kc,[kpath,'--runtime',runtimePath,'-o',kelf]);summary.observations++;
 if(!fs.readFileSync(native).equals(fs.readFileSync(kelf)))summary.failures.push(name+': K compiler bytes differ');
 return {source,routes:[['standalone',native,[]],['native',binary,['run',source,'--engine','native','--']],['k',binary,['run',source,'--engine','k','--']],['rust',oracle,[]]]};
}
async function execute(route,args,input,close,nofile){
 const [name,command,prefix]=route;
 const encoded=[...prefix,...args].map(x=>Buffer.isBuffer(x)?x.toString('hex'):Buffer.from(x).toString('hex'));
 const args_=[...(close===undefined?[]:[typeof close==='string'?close:'--close='+close]),command,...encoded];
 return run(nofile===undefined?launch:'prlimit',nofile===undefined?args_:['--nofile='+nofile,'--',launch,...args_],{encoding:null,input,captureControl:name==='rust',memoryBytes:'1073741824',timeout:30000});
}
function check(result,status,stdout,stderr,label){
 summary.observations++;
 const reference=label.endsWith('/rust')?worldOutcome(result):undefined;
 if(result.error||result.signal||result.status!==status||!result.stdout.equals(stdout)||!result.stderr.equals(stderr)||(reference&&(reference.kind!=='normal'||reference.status!==status))){
   summary.failures.push(label+': '+detail(result)+` stdout=${result.stdout.toString('hex')} stderr=${result.stderr.toString('hex')}`);
 }
}
const main=await compile('main');
const payload=Buffer.from([0xff,0x80,0x61,0x0a,0xc0]);
for(const [sample,input] of [['empty',Buffer.alloc(0)],['small',Buffer.from([0,255,10,128,1])],['multiple-reads',Buffer.from(Array.from({length:17003},(_,i)=>i%256))]]){
 for(const route of main.routes){
  const dir=path.join(output,'main-'+sample+'-'+route[0]);fs.mkdirSync(dir,{recursive:true});
  const expected=Buffer.concat([payload,input]);
  const result=await execute(route,[payload,dir,'output.bin'],input);
  check(result,7,expected,input,sample+'/'+route[0]);
  const file=path.join(dir,'output.bin');summary.observations++;
  if(!fs.existsSync(file)||!fs.readFileSync(file).equals(expected))summary.failures.push('file round trip '+sample+'/'+route[0]);
 }
}
const paths=await compile('paths');
for(const route of paths.routes){
 const dir=path.join(output,'data-paths-'+route[0]);fs.mkdirSync(dir,{recursive:true});
 fs.writeFileSync(path.join(dir,'inside'),'inside');fs.writeFileSync(path.join(output,'outside'),'outside');
 for(const [name,target] of [['inward','inside'],['outward','../outside']]){const file=path.join(dir,name);fs.rmSync(file,{force:true});fs.symlinkSync(target,file);}
 check(await execute(route,[dir],Buffer.alloc(0)),0,Buffer.from('paths ok\n'),Buffer.alloc(0),'paths/'+route[0]);
}
const closed=await compile('closed_stdin');
for(const route of closed.routes){const dir=path.join(output,'closed-'+route[0]);fs.mkdirSync(dir,{recursive:true});check(await execute(route,[dir],undefined,0),0,Buffer.from('closed ok\n'),Buffer.alloc(0),'closed stdin/'+route[0]);}
const cleanup=await compile('handle_cleanup');
for(const route of cleanup.routes){const dir=path.join(output,'data-cleanup-'+route[0]);fs.mkdirSync(dir,{recursive:true});check(await execute(route,[dir],Buffer.alloc(0),undefined,64),0,Buffer.from('cleanup ok\n'),Buffer.alloc(0),'handle cleanup/'+route[0]);}
const failure=await compile('output_failure');
for(const mode of ['close','broken-pipe'])for(const fd of [1,2])for(const route of failure.routes){
 const dir=path.join(output,'data-output-'+mode+'-'+fd+'-'+route[0]);fs.mkdirSync(dir,{recursive:true});
 const r=await execute(route,[dir,String(fd)],undefined,'--'+mode+'='+fd);
 summary.observations++;
 if(r.error||r.signal||r.status!==(route[0]==='rust'?101:1)||r.stdout.length!==0||(route[0]==='rust'?(worldOutcome(r).kind!=='trap'||worldOutcome(r).code!=='R0007'||r.stderr.length!==0):(fd===1?!r.stderr.toString().includes('R0007'):r.stderr.length!==0)))summary.failures.push('output failure/'+mode+'/'+fd+'/'+route[0]+': '+detail(r));
 summary.observations++;
 const file=path.join(dir,'owned-file');if(!fs.existsSync(file)||!fs.readFileSync(file).equals(Buffer.from('owned\n')))summary.failures.push('standard descriptor retargeted/'+mode+'/'+fd+'/'+route[0]);
}
const arguments_=await compile('arguments');
for(const route of arguments_.routes)check(await execute(route,[payload],Buffer.alloc(0)),0,Buffer.from('arguments ok\n'),Buffer.alloc(0),'arguments/'+route[0]);
const invalid=await compile('invalid_argument');
for(const route of invalid.routes){
 const result=await execute(route,[],Buffer.alloc(0));summary.observations++;
 if(result.error||result.signal||result.status==0||(route[0]==='rust'?(worldOutcome(result).kind!=='trap'||worldOutcome(result).code!=='R0003'):!result.stderr.toString().includes('R0003')))summary.failures.push('invalid argument/'+route[0]+': '+detail(result));
}
const marker=await compile('status_marker');
for(const route of marker.routes)check(await execute(route,[],Buffer.alloc(0)),101,Buffer.alloc(0),Buffer.from('LKJ_RUST_TRAP R0005\n'),'status marker/'+route[0]);
// Real program stderr is separate from diagnostics in the batch protocol.
const status=await compile('status');
const routes=await runRoutes(binary,[{path:status.source}],{engines:['native','k'],output,encoding:null});
for(const [engine,results] of Object.entries(routes)){
 const r=results[0];summary.observations++;
 if(r.status!==23||r.checkStatus!==0||r.stderr.length!==0||!r.stdout.equals(Buffer.from([0,255]))||!r.programStderr.equals(Buffer.from([255,0,128])))summary.failures.push('batch stderr/status '+engine+': '+detail(r));
}
summary.elapsedSeconds=(performance.now()-started)/1000;
fs.writeFileSync(path.join(output,'summary.json'),JSON.stringify(summary,null,2)+'\n');
console.log(JSON.stringify(summary));if(summary.failures.length)process.exitCode=1;
