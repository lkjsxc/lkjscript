#!/usr/bin/env node
// Offline Unicode and byte-path controls, with fixture compiler/time programs.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {json,sha} from './probe.mjs';
const here=path.dirname(fileURLToPath(import.meta.url));
let projector=path.join(here,'project.mjs'),probe=path.join(here,'probe.mjs');
for(let i=2;i<process.argv.length;i++){
  const arg=process.argv[i];if(arg==='--help'){console.log('Usage: node probe-test.mjs [--projector project.mjs] [--probe probe.mjs]\nNo real compiler or network is used.');process.exit(0)}
  if(arg==='--projector')projector=process.argv[++i];else if(arg==='--probe')probe=process.argv[++i];else throw Error('unexpected positional arguments');
}
projector=fs.realpathSync(projector);probe=fs.realpathSync(probe);
const tmp=fs.mkdtempSync(path.join(os.tmpdir(),'lkj-probe-unicode-'));
let checks=0,sequence=0;
const check=(ok,message)=>{checks++;if(!ok)throw Error('review check failed: '+message)},read=p=>fs.readFileSync(p),write=(p,s)=>fs.writeFileSync(p,s);
for(const k of Object.keys(process.env))if(['GIT_DIR','GIT_WORK_TREE','GIT_INDEX_FILE','GIT_COMMON_DIR','GIT_OBJECT_DIRECTORY','GIT_ALTERNATE_OBJECT_DIRECTORIES','GIT_CONFIG_PARAMETERS','GIT_CONFIG_COUNT','PERL_UNICODE'].includes(k)||/^GIT_CONFIG_(KEY|VALUE)_\d+$/.test(k))delete process.env[k];
process.env.GIT_CONFIG_NOSYSTEM='1';process.env.GIT_CONFIG_GLOBAL='/dev/null';
function run(...command){
  let c;
  if(command.some(Buffer.isBuffer)){
    const file=tmp+'/argv-'+sequence++;write(file,Buffer.concat(command.flatMap(s=>[Buffer.isBuffer(s)?s:Buffer.from(s),Buffer.from([0])])));
    c=spawnSync('/bin/bash',['-c','mapfile -d "" -t args < "$1"; exec "${args[@]}"','raw-argv',file],{encoding:'buffer',maxBuffer:Infinity});
  }else c=spawnSync(command[0],command.slice(1),{encoding:'buffer',maxBuffer:Infinity});
  return [c.status??(c.signal?128+os.constants.signals[c.signal]:127),c.stdout??Buffer.alloc(0),c.stderr??Buffer.from(c.error?.message??'')];
}
function git(root,...args){const [s,out,err]=run('git','-c','core.hooksPath=/dev/null','-C',root,...args);if(s)throw Error('git failed: '+err);return out.toString()}
try{
  fs.mkdirSync(tmp+'/tools');fs.copyFileSync(projector,tmp+'/tools/project.mjs');fs.copyFileSync(probe,tmp+'/tools/probe.mjs');
  const compilerCode=['#!'+process.execPath,'const fs=require("node:fs");',
    'const [verb,entry,sf,root,lf,library]=process.argv.slice(2);',
    'if(process.argv.length!==8||verb!=="check"||sf!=="--source-dir"||lf!=="--library-dir"||!fs.statSync(library).isDirectory())throw Error("bad compiler fixture arguments");',
    'const payload=fs.readFileSync(root+"/"+entry);',
    'process.stdout.write(Buffer.concat([Buffer.from("entry\\0"+entry+"\\0root\\0"+root+"\\0payload\\0"),payload,Buffer.from([255])]));',
    'process.stderr.write("diagnostic \\u2603\\n");',''].join('\n');
  const timeCode=['#!'+process.execPath,'const fs=require("node:fs"),cp=require("node:child_process"),os=require("node:os");',
    'const [f,format,o,path,...command]=process.argv.slice(2);',
    'if(f!=="-f"||format!=="%M"||o!=="-o")throw Error("bad time fixture arguments");',
    'const c=cp.spawnSync(command[0],command.slice(1),{stdio:"inherit"});',
    'fs.writeFileSync(path,"4321\\n");process.exit(c.status??(c.signal?128+os.constants.signals[c.signal]:127));',''].join('\n');
  const utf='源',ca=tmp+'/compiler',ta=tmp+'/time',cu=tmp+'/compiler-'+utf,tu=tmp+'/time-'+utf;
  for(const p of [ca,cu])write(p,compilerCode);for(const p of [ta,tu])write(p,timeCode);for(const p of [ca,cu,ta,tu])fs.chmodSync(p,0o755);
  const body=Buffer.from('fn original() {}\r\n// café\r\n');
  function fixture(label,entry,unicode){
    const dir=tmp+'/'+label,source=dir+'/source'+(unicode?'-'+utf:''),projection=dir+'/projection'+(unicode?'-'+utf:''),library=dir+'/library'+(unicode?'-'+utf:''),libRelative=label==='ascii'?'std/plain.lkj':'std/café.lkj';
    fs.mkdirSync(source+'/src',{recursive:true});fs.mkdirSync(library+'/std',{recursive:true});
    git(source,'-c','init.defaultBranch=main','init','--quiet','--object-format=sha1');
    for(const [k,v]of [['user.name','Local review fixture'],['user.email','fixture@example.invalid'],['commit.gpgsign','false'],['core.autocrlf','false'],['remote.origin.url','https://example.invalid/fixture.git']])git(source,'config',k,v);
    write(source+'/'+entry,body);write(library+'/'+libRelative,'fixture library\n');git(source,'add','--','.');git(source,'commit','--quiet','-m','fixture');
    const commit=git(source,'rev-parse','HEAD').trim(),[s,,e]=run(process.execPath,tmp+'/tools/project.mjs',source,projection);
    check(s===0,label+' projection succeeds: '+e);return {dir,source,projection,library,entry,libRelative,commit,compiler:unicode?cu:ca,time:unicode?tu:ta};
  }
  function invoke(c,output,override={}){c={...c,...override};return run(process.execPath,tmp+'/tools/probe.mjs','--compiler',c.compiler,'--time',c.time,'--library',c.library,'--source',c.source,'--projection',c.projection,'--package','fixture','--version','1.0.0','--entry',c.entry,'--output',output)}
  function validate(c,r){
    const digest=sha(read(c.projection+'/projection.json'));
    check(r.observations.length===2&&r.projection_manifest_sha256===digest&&r.verification_before.manifest_sha256===digest&&r.verification_after.manifest_sha256===digest,'both routes bind verified manifest');
    check(r.package.root===c.source&&r.package.commit===c.commit&&r.verification_before.commit===c.commit&&r.verification_after.commit===c.commit,'source root and Git commit binding');
    const raw=read(c.library+'/'+c.libRelative),expected=[{path:c.libRelative,bytes:raw.length,sha256:sha(raw)}];
    check(r.library.root===c.library&&r.library.manifest_sha256===sha(json(expected)),'library root and input manifest');check(json(r.library.files)===json(expected),'library file inventory and Unicode path');
    check(r.compiler.path===c.compiler&&r.compiler.sha256===sha(read(c.compiler)),'compiler path and byte identity');check(r.time_launcher.path===c.time&&r.time_launcher.sha256===sha(read(c.time)),'time launcher path and byte identity');
    for(const [i,o]of r.observations.entries()){
      const selected=i?c.entry.replace(/\.rs$/,'.lkj'):c.entry,root=i?c.projection:c.source;
      check(o.exit_status===0&&!o.launcher_signal,'route '+i+' opens exact OS byte path');check(o.command[2]===selected&&o.command[4]===root,'route '+i+' command text');check(o.upstream_entry===c.entry,'route '+i+' entry text');check(o.entry_sha256===sha(body)&&o.entry_bytes===body.length,'route '+i+' payload identity');
      const expected=Buffer.concat([Buffer.from('entry\0'+selected+'\0root\0'+root+'\0payload\0'),body,Buffer.from([255])]);
      check(Buffer.from(o.stdout.base64,'base64').equals(expected)&&o.stdout.sha256===sha(expected),'route '+i+' diagnostic bytes preserved');check(o.stderr.text==='diagnostic ☃\n','route '+i+' UTF-8 diagnostics');
    }
  }
  function fingerprint(receipt){
    const r=structuredClone(receipt),normal=s=>s.startsWith(tmp)?'/fixture'+s.slice(tmp.length):s;
    delete r.timestamp_utc;r.package.commit='FIXTURE_COMMIT';r.compiler.sha256='FIXTURE_COMPILER_SHA256';r.time_launcher.sha256='FIXTURE_TIME_SHA256';r.projection_manifest_sha256='FIXTURE_PROJECTION_SHA256';
    for(const v of [r.verification_before,r.verification_after]){v.commit='FIXTURE_COMMIT';v.manifest_sha256='FIXTURE_PROJECTION_SHA256';v.manifest=normal(v.manifest)}
    r.package.root=normal(r.package.root);r.compiler.path=normal(r.compiler.path);r.time_launcher.path=normal(r.time_launcher.path);r.library.root=normal(r.library.root);
    for(const o of r.observations){delete o.wall_seconds;delete o.load_before;delete o.load_after;o.command=o.command.map(normal);
      const raw=Buffer.from(Buffer.from(o.stdout.base64,'base64').toString('latin1').split(tmp).join('/fixture'),'latin1');
      Object.assign(o.stdout,{base64:raw.toString('base64'),sha256:sha(raw),bytes:raw.length});}
    return sha(json(r));
  }
  const ascii=fixture('ascii','src/lib.rs',false),[s,,e]=invoke(ascii,ascii.dir+'/receipt.json');check(s===0,'ASCII control runs: '+e);
  const r=JSON.parse(read(ascii.dir+'/receipt.json'));validate(ascii,r);
  check(fingerprint(r)==='0b4f12062b98aace48425ebbbaa2cd67e6b7526feeaf594efd7ac042195cbc22','ASCII receipt schema and stable semantics unchanged');
  for(const c of [fixture('utf-entry','src/café.rs',false),fixture('utf-roots','src/café.rs',true)]){
    const output=c.dir+'/receipt-'+utf+'.json',[s,out,e]=invoke(c,output);check(s===0,'UTF-8 probe runs: '+e);check(JSON.parse(out).receipt===output,'published receipt path is Unicode text');validate(c,JSON.parse(read(output)));
  }
  const invalid=(a,b)=>Buffer.concat([Buffer.from(a),Buffer.from([255]),Buffer.from(b)]);
  for(const [name,override,reason]of [['entry',{entry:invalid('src/bad','.rs')},/non-UTF-8 entry path/],['source',{source:invalid(tmp+'/source-','')},/non-UTF-8 input path/],['projection',{projection:invalid(tmp+'/projection-','')},/non-UTF-8 input path/]]){
    const output=tmp+'/invalid-'+name+'.json',[s,,e]=invoke(ascii,output,override);check(s!==0&&reason.test(e.toString())&&!fs.existsSync(output),'invalid UTF-8 '+name+' refused');
  }
  const bad=invalid(tmp+'/invalid-','.json'),[bs,,be]=invoke(ascii,bad);check(bs!==0&&/non-UTF-8 output path/.test(be.toString())&&!fs.existsSync(bad),'invalid UTF-8 output refused');
  write(invalid(ascii.library+'/bad-','.lkj'),'invalid path fixture\n');const [ls,,le]=invoke(ascii,tmp+'/invalid-library.json');
  check(ls!==0&&/non-UTF-8 library path/.test(le.toString())&&!fs.existsSync(tmp+'/invalid-library.json'),'invalid UTF-8 library member refused');
  console.log(json({passed:true,checks,cases:['ASCII receipt control','UTF-8 entry under ASCII roots','UTF-8 source/projection/library/compiler/time/output paths','invalid UTF-8 entry/source/projection/output/library-member refusals'],diagnostic_bytes_preserved:true,temporary_fixture_cleanup:'finally removes the temporary fixture',real_compiler_invoked:false}));
}catch(e){console.error(e.message);process.exitCode=255}finally{fs.rmSync(tmp,{recursive:true,force:true})}
