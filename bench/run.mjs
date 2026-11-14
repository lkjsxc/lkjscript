import fs from 'node:fs';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const options={compiler:'target/release/lkjscript',previous:'',out:'target/bench'};
for(let i=2;i<process.argv.length;i+=2){
 const name=process.argv[i].slice(2);
 if(!Object.hasOwn(options,name)||!process.argv[i+1])throw new Error('usage: node bench/run.mjs [--compiler path] [--previous path] [--out path]');
 options[name]=process.argv[i+1];
}
const output=path.resolve(root,options.out);
fs.mkdirSync(output,{recursive:true});
const previousFile=path.join(output,'latest.json');
const previous=fs.existsSync(previousFile)?JSON.parse(fs.readFileSync(previousFile,'utf8')):null;

function run(command,args){
 return new Promise((resolve,reject)=>{
  const started=process.hrtime.bigint();
  const child=spawn(command,args,{cwd:root,stdio:['ignore','pipe','pipe']});
  const stdout=[],stderr=[];
  child.stdout.on('data',part=>stdout.push(part));
  child.stderr.on('data',part=>stderr.push(part));
  child.on('error',reject);
  child.on('close',(status,signal)=>{
   const result={status,signal,stdout:Buffer.concat(stdout),stderr:Buffer.concat(stderr),seconds:Number(process.hrtime.bigint()-started)/1e9};
   if(status!==0||signal)reject(new Error(command+' '+args.join(' ')+' failed: '+signal+' '+status+'\n'+result.stderr.toString()+result.stdout.toString()));
   else resolve(result);
  });
 });
}
function same(reference,result,description){
 if(!result.stdout.equals(reference)||result.stderr.length!==0){
  throw new Error(description+' differs: stdout='+result.stdout.toString('hex')+' stderr='+result.stderr.toString('hex'));
 }
}
const programs=[
 {name:'aggregate',arguments:['20000000','7'],small:['13','7']},
 {name:'scalar',arguments:['200000000','7'],small:['13','7']},
 {name:'scalar_add',arguments:['200000000','7'],small:['13','7']},
 {name:'buffers',arguments:['200000','1000','7'],small:['127','19','7']},
 {name:'vector',arguments:['100000000','5000000','7'],small:['127','19','7']},
];
const routes=[
 {name:'native',compiler:path.resolve(root,options.compiler)},
 ...(options.previous&&fs.existsSync(path.resolve(root,options.previous))?[{name:'previous',compiler:path.resolve(root,options.previous)}]:[]),
 {name:'rustc0',opt:'0'},
 {name:'rustcO',opt:'3'},
];
const measurements=[];
for(const program of programs){
 const source=path.join(root,'bench',program.name+'.rs');
 const wrapper=path.join(output,program.name+'-witness.rs');
 const text=fs.readFileSync(source,'utf8').replace('mod common;', '#[path='+JSON.stringify(path.join(root,'bench/common.rs'))+'] mod common;');
 fs.writeFileSync(wrapper,'#[path='+JSON.stringify(path.join(root,'bridge/output.rs'))+'] mod __lkjscript_host;\nmod case {\n'+text+'\nuse crate::__lkjscript_host::World;\n}\nfn main(){__lkjscript_host::enter(case::main);}\n');
 const images=[];
 for(const route of routes){
  const image=path.join(output,program.name+'-'+route.name+'.elf');
  if(route.compiler)await run(route.compiler,['build',source,'-o',image]);
  else await run('rustc',['--edition=2024','--crate-name','benchmark','-C','overflow-checks=yes','-C','panic=abort','-C','opt-level='+route.opt,'-A','warnings',wrapper,'-o',image]);
  images.push({...route,image,bytes:fs.statSync(image).size});
 }
 let smallReference;
 for(const image of images){
  const result=await run(image.image,program.small);
  if(smallReference===undefined)smallReference=result.stdout;
  same(smallReference,result,program.name+'/'+image.name+' small');
 }
 let reference;
 for(const image of images){
  const rssFile=path.join(output,program.name+'-'+image.name+'.rss');
  const result=await run('/usr/bin/time',['-f','%M %U %S','-o',rssFile,image.image,...program.arguments]);
  if(reference===undefined)reference=result.stdout;
  same(reference,result,program.name+'/'+image.name);
  const [peakKiB,userSeconds,systemSeconds]=fs.readFileSync(rssFile,'utf8').trim().split(/\s+/).map(Number);
  fs.unlinkSync(rssFile);
  measurements.push({program:program.name,route:image.name,arguments:program.arguments,seconds:result.seconds,userSeconds,systemSeconds,peakKiB,bytes:image.bytes,result:result.stdout.toString().trim()});
 }
}
const lookup=(program,route)=>measurements.find(row=>row.program===program&&row.route===route);
const time=row=>row?row.seconds.toFixed(3):'—';
console.log('| Program | Native s | Previous s | rustc0 s | rustcO s | Native / rustc0 | Previous / native | Last run native s |');
console.log('| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |');
for(const program of programs){
 const current=lookup(program.name,'native'),old=lookup(program.name,'previous'),rust=lookup(program.name,'rustc0');
 const last=previous?.measurements?.find(row=>row.program===program.name&&row.route==='native');
 console.log('| '+[program.name,time(current),time(old),time(rust),time(lookup(program.name,'rustcO')),(current.seconds/rust.seconds).toFixed(2)+'x',old?(old.seconds/current.seconds).toFixed(2)+'x':'—',time(last)].join(' | ')+' |');
}
console.log('\n| Program / route | User s | Kernel s | Peak RSS KiB | ELF bytes | Result |');
console.log('| --- | ---: | ---: | ---: | ---: | ---: |');
for(const row of measurements)console.log('| '+[row.program+'/'+row.route,row.userSeconds.toFixed(2),row.systemSeconds.toFixed(2),row.peakKiB,row.bytes,row.result].join(' | ')+' |');
const report={measuredAt:new Date().toISOString(),compiler:options.compiler,previousCompiler:options.previous,measurements};
fs.writeFileSync(previousFile,JSON.stringify(report,null,2)+'\n');
