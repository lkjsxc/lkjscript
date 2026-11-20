#!/usr/bin/env node
// Observe unchanged upstream roots. This host harness is not an admission certificate.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';

const here=path.dirname(fileURLToPath(import.meta.url));
const fail=message=>{throw new Error('real-programs-probe: '+message)};
export const sha=raw=>createHash('sha256').update(raw).digest('hex');
export function json(value, pretty=false) {
  const sorted=v=>Array.isArray(v)?v.map(sorted):v!==null&&typeof v==='object'
    ?Object.fromEntries(Object.keys(v).sort().map(k=>[k,sorted(v[k])])):v;
  return JSON.stringify(sorted(value),null,pretty?3:undefined).replace(/[\u0080-\uffff]/g,c=>'\\u'+c.charCodeAt(0).toString(16).padStart(4,'0'));
}
const bytes=file=>fs.readFileSync(file);
function text(raw,description) {
  try{return new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(raw)}
  catch{fail('non-UTF-8 '+description)}
}
function unicode(value,description) {
  if(Buffer.from(value,'utf8').toString('utf8')!==value)fail('invalid Unicode '+description);
  return value;
}
function exists(file){try{fs.lstatSync(file);return true}catch(e){if(e.code==='ENOENT')return false;throw e}}
function rawArguments(){
  // Node's decoded argv replaces malformed bytes; validate the original Linux argv.
  const fields=bytes('/proc/self/cmdline').subarray(0,-1).toString('latin1').split('\0').map(s=>Buffer.from(s,'latin1'));
  const count=process.argv.length-2;return count?fields.slice(-count):[];
}
function options(){
  const out={entry:[],time:Buffer.from('/usr/bin/time')},allowed=new Set(['compiler','library','source','projection','output','package','version','entry','time']);
  const args=rawArguments();
  for(let i=0;i<args.length;i++){
    const split=args[i].indexOf(61),flag=(split<0?args[i]:args[i].subarray(0,split)).toString('ascii').replace(/^--?/,'');
    if(flag==='help'&&split<0){out.help=true;continue}
    if(!/^--?/.test(args[i].toString('ascii'))||!allowed.has(flag))fail('use --help');
    const value=split<0?args[++i]:args[i].subarray(split+1);
    if(value===undefined)fail('use --help');
    if(flag==='entry')out.entry.push(value);else out[flag]=value;
  }
  return out;
}
function libraryIdentity(root){
  const files=[],rootBytes=Buffer.from(root);
  function visit(relative){
    const relativeText=text(relative,'library path');
    const file=relative.length?Buffer.concat([rootBytes,Buffer.from('/'),relative]):rootBytes;
    const stat=fs.lstatSync(file);
    if(stat.isSymbolicLink())fail('symlink in library input: '+file);
    if(stat.isDirectory()){
      for(const name of fs.readdirSync(file,{encoding:'buffer'}).sort(Buffer.compare))
        visit(relative.length?Buffer.concat([relative,Buffer.from('/'),name]):name);
    }else if(stat.isFile()){
      const raw=bytes(file);files.push({path:relativeText,bytes:raw.length,sha256:sha(raw)});
    }else fail('nonregular library input: '+file);
  }
  visit(Buffer.alloc(0));return {files,manifest_sha256:sha(json(files))};
}
function stream(file){
  const raw=bytes(file),result={bytes:raw.length,sha256:sha(raw)};
  try{result.text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(raw)}
  catch{result.base64=raw.toString('base64')}
  return result;
}
function loadAverage(){const match=bytes('/proc/loadavg').toString().match(/^([0-9.]+) /);if(!match)fail('unexpected /proc/loadavg');return match[1]}
function observed(temporary,index,command,time){
  const out=path.join(temporary,index+'.stdout'),err=path.join(temporary,index+'.stderr'),rss=path.join(temporary,index+'.rss');
  const before=loadAverage(),start=process.hrtime.bigint();
  const outFd=fs.openSync(out,'w'),errFd=fs.openSync(err,'w');
  let child;
  try{child=spawnSync(time,['-f','%M','-o',rss,...command],{stdio:['ignore',outFd,errFd]})}
  finally{fs.closeSync(outFd);fs.closeSync(errFd)}
  if(child.error)fail('exec '+time+': '+child.error.message);
  const elapsed=Number(process.hrtime.bigint()-start)/1e9,after=loadAverage();
  const match=bytes(rss).toString().match(/(?:^|\n)([0-9]+)\n$/);if(!match)fail('missing GNU-time peak RSS');
  return {command,exit_status:child.status??0,launcher_signal:child.signal?os.constants.signals[child.signal]:0,
    wall_seconds:Number(elapsed.toFixed(6)),peak_rss_kib:Number(match[1]),load_before:before,load_after:after,
    stdout:stream(out),stderr:stream(err),time_report:stream(rss)};
}
function verify(projector,source,projection){
  const child=spawnSync(process.execPath,[projector,'--verify',source,projection],{encoding:'buffer',maxBuffer:Infinity});
  if(child.stderr?.length)process.stderr.write(child.stderr);
  if(child.error||child.status!==0||child.signal)fail('projection verification failed: '+(child.error?.message??child.status));
  return JSON.parse(text(child.stdout,'verifier JSON'));
}
export function main(){
  const args=options();
  if(args.help){
    process.stdout.write('Usage: node probe.mjs --compiler COMPILER --library LIBRARY --source SOURCE\n       --projection PROJECTION --package NAME --version EXACT --entry src/lib.rs\n       [--entry another/root.rs ...] --output NEW_RECEIPT.json [--time GNU_TIME]\nSOURCE and PROJECTION must already pass project.mjs --verify. Every diagnostic byte,\nexact command, input identity, load, elapsed wall time and peak process RSS is retained.\nThese source-root observations do not establish Cargo-equivalent admission.\n');return;
  }
  if(!args.entry.length||['compiler','library','source','projection','output','package','version'].some(k=>!args[k]?.length))
    fail('all named arguments and at least one --entry are required');
  const entries=args.entry.map(v=>text(v,'entry path'));
  const packageName=text(args.package,'package name'),version=text(args.version,'package version'),output=text(args.output,'output path');
  for(const key of ['compiler','library','source','projection','time']){
    text(args[key],'input path');
    try{args[key]=text(fs.realpathSync(args[key],{encoding:'buffer'}),'resolved input path')}
    catch(e){if(e.message.startsWith('real-programs-probe:'))throw e;fail('input path does not exist')}
  }
  const {compiler,library,source,projection,time}=args;
  for(const [file,label]of [[compiler,'compiler'],[time,'GNU-time-compatible launcher']]){
    try{if(!fs.statSync(file).isFile())throw new Error();fs.accessSync(file,fs.constants.X_OK)}catch{fail(label+' is not executable')}
  }
  if(exists(output))fail('output already exists');
  const projector=path.join(here,'project.mjs');
  if(!fs.existsSync(projector))fail('project.mjs must be adjacent to probe.mjs');
  const before=verify(projector,source,projection),manifestRaw=bytes(path.join(projection,'projection.json'));
  if(sha(manifestRaw)!==before.manifest_sha256)fail('projection manifest differs from the verified bytes');
  const manifest=JSON.parse(text(manifestRaw,'projection manifest')),files=new Map(manifest.files.map(f=>[f.path,f])),aliases=new Map(manifest.aliases.map(a=>[a.from,a.to])),seen=new Set();
  for(const entry of entries){if(!files.has(entry)||!aliases.has(entry)||seen.has(entry))fail('entry must be a unique tracked Rust path: '+entry);seen.add(entry)}
  const producer=sha(bytes(compiler)),libraryBefore=libraryIdentity(library);
  const temporary=fs.mkdtempSync(path.join(path.dirname(output),'root-probe-'));
  try{
    const observations=[];
    for(const entry of entries)for(const route of ['original_suffix','suffix_projection']){
      const root=route==='original_suffix'?source:projection,selected=route==='original_suffix'?entry:unicode(aliases.get(entry),'manifest alias path');
      observations.push({route,upstream_entry:entry,entry_sha256:files.get(entry).sha256,entry_bytes:files.get(entry).bytes,
        ...observed(temporary,observations.length,[compiler,'check',selected,'--source-dir',root,'--library-dir',library],time)});
    }
    const after=verify(projector,source,projection);
    if(before.manifest_sha256!==after.manifest_sha256)fail('projection manifest changed during observations');
    if(sha(bytes(compiler))!==producer)fail('compiler bytes changed during observations');
    if(libraryBefore.manifest_sha256!==libraryIdentity(library).manifest_sha256)fail('library bytes changed during observations');
    const receipt={schema:1,kind:'unmodified_real_program_root_observations',timestamp_utc:new Date().toISOString().replace(/\.\d{3}Z$/,'Z'),
      package:{name:packageName,version,...manifest.source},compiler:{path:compiler,sha256:producer},time_launcher:{path:time,sha256:sha(bytes(time))},
      library:{root:library,...libraryBefore},projection_manifest_sha256:sha(manifestRaw),verification_before:before,verification_after:after,
      interpretation:"Every source byte is unchanged. Suffix projection preserves tracked module/asset context. The compiler has no Cargo package/feature/test configuration here. A refusal is a root observation, not an exhaustive blocker count; acceptance would not establish Cargo-equivalent configuration or executed upstream tests. Exit status and launcher_signal belong to the time launcher; the complete separate time_report retains its abnormal-exit/signal diagnostic as well as peak RSS.",
      observations};
    const pending=path.join(temporary,'.root-probe-receipt');
    fs.writeFileSync(pending,json(receipt,true)+'\n',{mode:0o600,flag:'wx'});
    try{fs.linkSync(pending,output)}catch(e){fail('publish new receipt: '+e.message)}
    fs.unlinkSync(pending);
    process.stdout.write(json({receipt:output,observations:observations.length,compiler_sha256:producer,source_unchanged:true})+'\n');
  }finally{fs.rmSync(temporary,{recursive:true,force:true})}
}
if(process.argv[1]&&fs.realpathSync(process.argv[1])===fileURLToPath(import.meta.url)){
  try{main()}catch(e){console.error(e.message.startsWith('real-programs-probe:')?e.message:'real-programs-probe: '+e.message);process.exitCode=255}
}
