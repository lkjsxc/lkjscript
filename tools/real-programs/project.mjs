#!/usr/bin/env node
// Host-only projection: Git plumbing and Linux inode/hardlink/symlink operations.
// No source translation, payload copying, chmod, or symlink traversal.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {spawn,spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {fileURLToPath,pathToFileURL} from 'node:url';
const MANIFEST='projection.json', C=fs.constants;
const fail=m=>{throw new Error('real-programs-project: '+m);};
const hash=b=>createHash('sha256').update(b).digest('hex');
const decode=(b,description)=>{try{return new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(b);}catch{fail('non-UTF-8 '+description);}};
const stat=p=>{try{return fs.lstatSync(p,{bigint:true});}catch(e){if(e.code==='ENOENT')return null;throw e;}};
const same=(a,b)=>a.dev===b.dev&&a.ino===b.ino;
const rawSort=(a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b));
const scalarSort=(a,b)=>{a=Array.from(a);b=Array.from(b);for(let i=0;i<Math.min(a.length,b.length);i++){const d=a[i].codePointAt(0)-b[i].codePointAt(0);if(d)return d;}return a.length-b.length;};
function json(v,level=0){const pad=n=>' '.repeat(n*3),quote=x=>JSON.stringify(x).replace(/[\u007f-\uffff]/g,c=>'\\u'+c.charCodeAt(0).toString(16).padStart(4,'0'));if(v===null||typeof v!=='object')return typeof v==='string'?quote(v):JSON.stringify(v);const array=Array.isArray(v),keys=array?v.map((_,i)=>i):Object.keys(v).sort(scalarSort);if(!keys.length)return array?'[]':'{}';return(array?'[':'{')+'\n'+keys.map(k=>pad(level+1)+(array?'':quote(k)+' : ')+json(v[k],level+1)).join(',\n')+'\n'+pad(level)+(array?']':'}');}

const regularClaims={"claim":"Path-only projection: tracked bytes are unchanged; .rs files gain .lkj aliases. No Rust compatibility, semantic equivalence, or checker acceptance is claimed.","storage":"All tracked originals and suffix aliases are hardlinks to SOURCE; no payload copies. Hardlinks share writable contents and are not a write-protection boundary.","provenance":"Records Git identity and byte integrity; does not authenticate a publisher or verify signatures.","concurrency":"SOURCE and DEST namespaces must remain quiescent during creation, verification and cleanup. This tool is not a boundary against concurrent host filesystem mutation.","clean_state":"Index equals HEAD; every tracked file equals its raw Git blob and executable mode. Untracked files are ignored. Symlinks and gitlinks are refused."};
const linkClaims={"claim":"Path-only projection: tracked regular-file bytes and symbolic-link target bytes are unchanged; .rs paths gain .lkj aliases. No Rust compatibility, semantic equivalence, or checker/loader acceptance is claimed.","storage":"Tracked regular files, symbolic-link objects and suffix aliases are hardlinks to SOURCE, including symlink inodes on Linux. No target payload copies or chmod. Hardlinks share writable contents and are not a write-protection boundary.","clean_state":"Index equals HEAD; regular files equal their raw Git blobs and executable modes, and symlink targets equal their raw Git blobs. Untracked files are ignored. Gitlinks and nonregular nonsymlink nodes are refused.","symlink_policy":"Only relative links resolving entirely within the original tracked-node/directory graph are admitted. Absolute, escaping, dangling/untracked, chain-cyclic and directory-cyclic targets are refused. Inventory uses lstat/readlink and never traverses a symbolic link."};

function gitOutput(root,...args){
 const env={...process.env};
 for(const k of Object.keys(env))if(/^(GIT_DIR|GIT_WORK_TREE|GIT_INDEX_FILE|GIT_COMMON_DIR|GIT_OBJECT_DIRECTORY|GIT_ALTERNATE_OBJECT_DIRECTORIES|GIT_CONFIG_PARAMETERS|GIT_CONFIG_COUNT)$/.test(k)||/^GIT_CONFIG_(KEY|VALUE)_\d+$/.test(k))delete env[k];
 Object.assign(env,{GIT_OPTIONAL_LOCKS:'0',GIT_CONFIG_NOSYSTEM:'1',GIT_CONFIG_GLOBAL:'/dev/null'});
 const run=spawnSync('git',['--no-replace-objects','--no-optional-locks','-c','core.fsmonitor=false','-c','core.hooksPath=/dev/null','-C',root,...args],{env,maxBuffer:Infinity});
 if(run.error||run.status!==0){if(run.stderr?.length)process.stderr.write(run.stderr);fail('Git '+args[0]+' failed (status '+run.status+')');}
 return run.stdout;
}
function canonicalDirectory(input,description){
 if(!input.length||input.includes('\0'))fail('empty or NUL-containing '+description);
 const absolute=input.startsWith('/')?input:process.cwd()+'/'+input;
 let walk='/';
 for(const part of absolute.split(/\/+/)){
  if(!part||part==='.')continue;
  if(part==='..'){walk=path.dirname(walk);continue;}
  walk=(walk==='/'?'':walk)+'/'+part;
  const st=stat(walk);
  if(!st)fail('missing directory component in '+description+': '+walk);
  if(st.isSymbolicLink())fail('symlink directory component in '+description+': '+walk);
  if(!st.isDirectory())fail('non-directory component in '+description+': '+walk);
 }
 return fs.realpathSync(walk);
}
function safePath(p){
 if(!p.length)fail('empty tracked path');
 if(/^\/|^[A-Za-z]:|\\|\0/.test(p))fail('absolute or ambiguous tracked path: '+p);
 if(p.split('/').some(x=>!x||x==='.'||x==='..'))fail('escaping or noncanonical tracked path: '+p);
 // Callers decode raw Git/path bytes strictly before constructing a JS string.
 if(decode(Buffer.from(p),'tracked path')!==p)fail('non-UTF-8 tracked path');
 return p;
}
function nodePath(root,relative,allowLink){
 safePath(relative);const parts=relative.split('/');let walk=root;
 for(let i=0;i<parts.length;i++){
  walk+='/'+parts[i];const st=stat(walk);
  if(!st)fail('missing node: '+walk);
  if(i===parts.length-1){if(!st.isFile()&&!(allowLink&&st.isSymbolicLink()))fail('nonregular file: '+walk);}
  else{if(st.isSymbolicLink())fail('symlink parent: '+walk);if(!st.isDirectory())fail('non-directory parent: '+walk);}
 }
 return walk;
}
function readNode(p,format=null,keep=false){
 const before=stat(p);if(!before?.isFile())fail('not a regular file: '+p);
 const fd=fs.openSync(p,C.O_RDONLY|C.O_NOFOLLOW);let result;
 try{
  const opened=fs.fstatSync(fd,{bigint:true});
  if(!opened.isFile()||!same(before,opened))fail('file identity changed while opening: '+p);
  const sha=createHash('sha256'),git=format?createHash(format):null,chunks=[];
  if(git)git.update('blob '+opened.size+'\0');
  let size=0;const buffer=Buffer.alloc(65536);
  while(true){const n=fs.readSync(fd,buffer,0,buffer.length,null);if(!n)break;size+=n;const chunk=buffer.subarray(0,n);sha.update(chunk);if(git)git.update(chunk);if(keep)chunks.push(Buffer.from(chunk));}
  const after=fs.fstatSync(fd,{bigint:true});
  if(BigInt(size)!==opened.size||BigInt(size)!==after.size||opened.mtimeNs!==after.mtimeNs||opened.ctimeNs!==after.ctimeNs)fail('file changed while reading: '+p);
  result={kind:'regular',sha256:sha.digest('hex'),bytes:size,device:opened.dev,inode:opened.ino,mode:Number(opened.mode),links:opened.nlink};
  if(git)result.git_blob=git.digest('hex');if(keep)result.content=Buffer.concat(chunks);
 }finally{fs.closeSync(fd);}
 return result;
}
function readLinkNode(p,format=null){
 const before=stat(p);if(!before?.isSymbolicLink())fail('not a symbolic link: '+p);
 const target=fs.readlinkSync(p,{encoding:'buffer'}),after=stat(p);
 if(!after?.isSymbolicLink()||!same(before,after)||before.mtimeNs!==after.mtimeNs||before.ctimeNs!==after.ctimeNs)fail('symbolic link changed while reading: '+p);
 const record={kind:'symlink',raw_target:target,sha256:hash(target),bytes:target.length,device:before.dev,inode:before.ino,mode:Number(before.mode),links:before.nlink};
 if(format)record.git_blob=createHash(format).update('blob '+target.length+'\0').update(target).digest('hex');
 return record;
}
function validateLinkGraph(nodes){
 const directories=new Set(['']),resolved=new Map(),links=[];
 for(const p of nodes.keys()){const parts=p.split('/');parts.pop();while(parts.length){directories.add(parts.join('/'));parts.pop();}}
 for(const p of [...nodes.keys()].sort(rawSort)){
  if(nodes.get(p).kind!=='symlink')continue;
  links.push(p);const target=decode(nodes.get(p).raw_target,'symlink target');
  if(!target.length||/^\/|^[A-Za-z]:|\\|\0/.test(target))fail('empty, absolute, or ambiguous symlink target: '+p);
 }
 function walk(base,parts,active){
  let stack=[...base],pending=[...parts];
  while(pending.length){
   const part=pending.shift();if(!part||part==='.')continue;
   if(part==='..'){if(!stack.length)fail('symlink target escapes SOURCE');stack.pop();continue;}
   const p=[...stack,part].join('/');
   if(nodes.has(p)){
    const target=nodes.get(p).kind==='symlink'?resolveLink(p,active):{path:p,kind:'file'};
    if(pending.length&&target.kind!=='directory')fail('symlink target traverses a non-directory: '+p);
    if(!pending.length)return target;stack=target.path?target.path.split('/'):[];
   }else if(directories.has(p))stack.push(part);else fail('dangling or untracked symlink target: '+p);
  }
  return {path:stack.join('/'),kind:'directory'};
 }
 function resolveLink(p,active){
  if(active.has(p))fail('cyclic symlink chain: '+p);if(resolved.has(p))return resolved.get(p);
  const base=p.split('/');base.pop();const next=new Set(active);next.add(p);
  const target=walk(base,decode(nodes.get(p).raw_target,'symlink target').split('/'),next);resolved.set(p,target);return target;
 }
 for(const p of links)resolveLink(p,new Set());
 const edges=new Map(),state=new Map();
 const edge=(from,to)=>{if(!edges.has(from))edges.set(from,new Set());edges.get(from).add(to);};
 for(const d of directories)if(d)edge(path.dirname(d)==='.'?'':path.dirname(d),d);
 for(const p of links){const r=resolved.get(p);if(r.kind==='directory')edge(path.dirname(p)==='.'?'':path.dirname(p),r.path);}
 function visit(d){if(state.get(d)===1)fail('cyclic directory symlink graph at: '+d);if(state.get(d)===2)return;state.set(d,1);for(const child of [...(edges.get(d)||[])].sort(rawSort))visit(child);state.set(d,2);}
 visit('');return resolved;
}
function parseEntries(raw,tree=false){
 const records=Object.create(null);
 for(const entry of decode(raw,'Git entries').split('\0')){
  if(!entry)continue;
  const m=entry.match(tree?/^([0-7]{6}) (\w+) ([0-9a-f]+)\t(.+)$/s:/^([0-7]{6}) ([0-9a-f]+) ([0-3])\t(.+)$/s);
  if(!m)fail('malformed Git '+(tree?'tree':'index')+' entry');
  const mode=m[1],oid=tree?m[3]:m[2],p=safePath(m[4]);
  if(Object.hasOwn(records,p)||(!tree&&m[3]!=='0'))fail('unmerged or duplicate tracked path: '+p);
  if(!['100644','100755','120000'].includes(mode)||(tree&&m[2]!=='blob'))fail('gitlink or unsupported tracked mode '+mode+': '+p);
  records[p]={mode,oid};
 }
 return records;
}
function sourceSnapshot(input){
 const root=canonicalDirectory(input,'SOURCE'),gitText=(...a)=>decode(gitOutput(root,...a),'Git output').replace(/\n$/,'');
 if(gitText('rev-parse','--show-toplevel')!==root)fail('SOURCE must be the Git worktree root');
 const head=gitText('rev-parse','--verify','HEAD^{commit}');
 if(!/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(head))fail('invalid Git HEAD identity');
 const format=gitText('rev-parse','--show-object-format');if(!['sha1','sha256'].includes(format))fail('unsupported Git object format: '+format);
 const origins=decode(gitOutput(root,'config','--null','--get-all','remote.origin.url'),'origin URL').split('\0');if(origins.at(-1)==='')origins.pop();
 if(origins.length!==1||!origins[0])fail('SOURCE needs exactly one nonempty origin URL');
 const index=parseEntries(gitOutput(root,'ls-files','--stage','-z')),tree=parseEntries(gitOutput(root,'ls-tree','-r','-z','--full-tree',head),true);
 if(json(index)!==json(tree))fail('dirty Git index: tracked entries differ from HEAD');
 if(!Object.keys(index).length)fail('SOURCE has no tracked files');
 const files=[],nodes=new Map(),records=new Map();
 for(const p of Object.keys(index).sort(rawSort)){
  const isLink=index[p].mode==='120000',node=isLink?readLinkNode(nodePath(root,p,true),format):readNode(nodePath(root,p,false),format);
  if(node.git_blob!==index[p].oid)fail('dirty tracked bytes (including filters/assume-unchanged): '+p);
  const mode=isLink?'120000':node.mode&0o100?'100755':'100644';if(mode!==index[p].mode)fail('dirty tracked executable mode: '+p);
  const record={path:p,sha256:node.sha256,bytes:node.bytes,git_blob:node.git_blob,git_mode:mode};
  if(isLink){record.kind='symlink';record.target=decode(node.raw_target,'symlink target');}
  files.push(record);nodes.set(p,node);records.set(p,record);
 }
 for(const [p,target] of validateLinkGraph(nodes)){records.get(p).resolved_target=target.path;records.get(p).target_kind=target.kind;}
 return {root,source:{root,commit:head,origin:origins[0],object_format:format},files,nodes};
}
function layout(snapshot){
 const targets=new Map(),directories=new Set(),links=[],aliases=[];
 function reserve(p,description){safePath(p);if(targets.has(p))fail('projection path collision: '+p+' ('+targets.get(p)+', '+description+')');targets.set(p,description);const parts=p.split('/');parts.pop();while(parts.length){directories.add(parts.join('/'));parts.pop();}}
 for(const p of [...snapshot.nodes.keys()].sort(rawSort)){
  reserve(p,'tracked original');links.push({from:p,to:p});
  if(p.endsWith('.rs')){const alias=p.slice(0,-3)+'.lkj';reserve(alias,'Rust suffix alias');links.push({from:p,to:alias});aliases.push({from:p,to:alias});}
 }
 reserve(MANIFEST,'projection manifest');
 for(const d of directories)if(targets.has(d))fail('projection file/directory collision: '+d);
 const manifest={schema:1,kind:'git_tracked_suffix_projection',source:snapshot.source,...regularClaims,files:snapshot.files,aliases};
 if([...snapshot.nodes.values()].some(n=>n.kind==='symlink'))Object.assign(manifest,linkClaims);
 const kinds=new Map([[MANIFEST,'regular']]);for(const link of links)kinds.set(link.to,snapshot.nodes.get(link.from).kind);
 return {manifest,json:json(manifest)+'\n',links:links.sort((a,b)=>rawSort(a.to,b.to)),targets,kinds,directories};
}
function destinationPath(input,source){
 if(!input.length||input.includes('\0'))fail('empty or NUL-containing DEST');input=input.replace(/\/+$/,'');
 const name=path.basename(input);if(!name||name==='.'||name==='..')fail('DEST must name a fresh directory');
 const parent=canonicalDirectory(path.dirname(input),'DEST parent'),dest=(parent==='/'?'':parent)+'/'+name;
 if(source==='/'||dest===source||dest.startsWith(source+'/'))fail('DEST must be outside SOURCE');
 return dest;
}
function visitTree(root,callback,relative=''){
 const p=relative?root+'/'+relative:root,st=stat(p);if(!st)fail('missing projection node: '+p);
 if(relative)callback(relative,st);if(!st.isDirectory())return;
 const names=fs.readdirSync(p,{encoding:'buffer'}).map(b=>decode(b,'projection path')).sort(rawSort);
 for(const name of names)visitTree(root,callback,relative?relative+'/'+name:name);
}
export function verifyProjection(sourceInput,destInput){
 const snapshot=sourceSnapshot(sourceInput),plan=layout(snapshot),dest=destinationPath(destInput,snapshot.root);
 canonicalDirectory(dest,'DEST');
 visitTree(dest,(relative,st)=>{
  if(!st.isFile()&&!st.isDirectory()&&!st.isSymbolicLink())fail('nonregular nonsymlink projection node: '+relative);
  if(st.isDirectory()){if(!plan.directories.has(relative))fail('unexpected projection directory: '+relative);}
  else{if(!plan.targets.has(relative))fail('unexpected projection file: '+relative);if((st.isSymbolicLink()?'symlink':'regular')!==plan.kinds.get(relative))fail('projection node type changed: '+relative);}
 });
 const manifest=readNode(nodePath(dest,MANIFEST,false),null,true);
 if(!manifest.content.equals(Buffer.from(plan.json)))fail('manifest differs from current clean SOURCE identity, hashes, or prescribed layout');
 for(const link of plan.links){
  const original=snapshot.nodes.get(link.from),node=original.kind==='symlink'?readLinkNode(nodePath(dest,link.to,true)):readNode(nodePath(dest,link.to,false));
  if(node.device!==original.device||node.inode!==original.inode)fail('not an original-source hardlink: '+link.to);
  if(node.bytes!==original.bytes||node.sha256!==original.sha256)fail('projection bytes changed: '+link.to);
  if(original.kind==='symlink'&&!node.raw_target.equals(original.raw_target))fail('projection symlink target changed: '+link.to);
 }
 const result={operation:'verify',read_only:true,manifest:dest+'/'+MANIFEST,manifest_sha256:manifest.sha256,commit:snapshot.source.commit,tracked_files:snapshot.files.length,aliases:plan.manifest.aliases.length};
 const count=[...snapshot.nodes.values()].filter(n=>n.kind==='symlink').length;if(count)result.tracked_symlinks=count;return result;
}
function removeOwnedTree(root,identity){
 const st=stat(root);if(!st)return;if(!st.isDirectory()||!same(st,identity))fail('refusing cleanup of replaced destination: '+root);
 function remove(p){const node=stat(p);if(!node)return;if(node.isDirectory()){for(const b of fs.readdirSync(p,{encoding:'buffer'}))remove(Buffer.concat([Buffer.from(p+'/'),b]));fs.rmdirSync(p);}else fs.unlinkSync(p);}
 remove(root);
}
function linkPayload(from,to){try{fs.linkSync(from,to);}catch(e){fail('hardlink '+to+': '+e.message+' (copy fallback is forbidden)');}}
// Yield only after a mutation is fully owned (or its descriptor is closed).
// The CLI lets pending signals run at these boundaries; throwing into this
// generator enters the same identity-checked cleanup used for ordinary errors.
function* projectionSteps(sourceInput,destInput,linker=linkPayload){
 const snapshot=sourceSnapshot(sourceInput),plan=layout(snapshot),dest=destinationPath(destInput,snapshot.root);
 if(stat(dest))fail('DEST already exists: '+dest);
 let identity,result;
 try{
  yield;
  fs.mkdirSync(dest,{mode:0o700});identity=stat(dest);if(!identity)fail('cannot record newly created DEST identity');yield;
  for(const d of [...plan.directories].sort(rawSort)){fs.mkdirSync(dest+'/'+d,{mode:0o700});yield;}
  for(const link of plan.links){linker(nodePath(snapshot.root,link.from,true),dest+'/'+link.to);yield;}
  const fd=fs.openSync(dest+'/'+MANIFEST,C.O_WRONLY|C.O_CREAT|C.O_EXCL|C.O_NOFOLLOW,0o600);
  try{fs.writeFileSync(fd,plan.json);}finally{fs.closeSync(fd);}
  yield;
  result=verifyProjection(snapshot.root,dest);yield;
 }catch(e){if(identity)try{removeOwnedTree(dest,identity);}catch(cleanup){e.message+='\ncleanup also failed: '+cleanup.message;}throw e;}
 result.operation='create';delete result.read_only;return result;
}
export function project(sourceInput,destInput,linker=linkPayload){
 const steps=projectionSteps(sourceInput,destInput,linker);let next=steps.next();
 while(!next.done)next=steps.next();return next.value;
}
export async function projectInterruptibly(sourceInput,destInput){
 let interrupted;
 const interrupt=()=>{interrupted??=new Error('interrupted');},terminate=()=>{interrupted??=new Error('terminated');};
 process.on('SIGINT',interrupt);process.on('SIGTERM',terminate);
 try{
  const steps=projectionSteps(sourceInput,destInput);let next=steps.next();
  while(!next.done){
   await new Promise(resolve=>setImmediate(resolve));
   next=interrupted?steps.throw(interrupted):steps.next();
  }
  return next.value;
 }finally{process.removeListener('SIGINT',interrupt);process.removeListener('SIGTERM',terminate);}
}
async function signalSelfTest(){
 const tmp=fs.mkdtempSync(path.join(os.tmpdir(),'lkj-project-signals-')),identity=stat(tmp),source=tmp+'/source';
 let checks=0;const check=(ok,label)=>{checks++;if(!ok)fail('signal self-test: '+label);};
 try{
  fs.mkdirSync(source,{mode:0o700});
  const git=(...args)=>gitOutput(source,...args);
  git('-c','init.defaultBranch=main','init','--quiet');
  for(const[k,v]of [['user.name','Projection signal fixture'],['user.email','fixture@example.invalid'],['commit.gpgsign','false'],['core.autocrlf','false'],['remote.origin.url','https://example.invalid/signals.git']])git('config',k,v);
  // A link sorts first. The remaining files keep creation active long enough
  // for the parent to observe the real CLI's owned symlink inode and signal it.
  for(let i=0;i<512;i++)fs.writeFileSync(source+'/file-'+String(i).padStart(4,'0')+'.rs','// '+i+'\n');
  fs.symlinkSync('file-0000.rs',source+'/000-link.rs');
  git('add','--','.');git('commit','--quiet','-m','signal fixture');
  const before=sourceSnapshot(source);
  for(const signal of ['SIGINT','SIGTERM']){
   const dest=tmp+'/'+signal,child=spawn(process.execPath,[fileURLToPath(import.meta.url),source,dest],{stdio:['ignore','pipe','pipe']});
   const stdout=[],stderr=[];child.stdout.on('data',b=>stdout.push(b));child.stderr.on('data',b=>stderr.push(b));
   let sent=false,timedOut=false;
   const result=await new Promise((resolve,reject)=>{
    const watch=setInterval(()=>{if(!sent&&stat(dest+'/000-link.lkj')?.isSymbolicLink()){sent=child.kill(signal);}},1);
    // Fixture deadline only: it does not limit user projection operations.
    const deadline=setTimeout(()=>{timedOut=true;child.kill('SIGKILL');},10000);
    child.once('error',e=>{clearInterval(watch);clearTimeout(deadline);reject(e);});
    child.once('close',(code,terminatedBy)=>{clearInterval(watch);clearTimeout(deadline);resolve({code,terminatedBy});});
   });
   check(sent&&!timedOut,signal+' delivered after a symlink object was linked');
   check(result.code===255&&result.terminatedBy===null,signal+' preserves failure exit255');
   check(Buffer.concat(stderr).toString().includes(signal==='SIGINT'?'interrupted':'terminated')&&Buffer.concat(stdout).length===0,signal+' reports interruption without a success report');
   check(!stat(dest),signal+' cleans only the owned destination');
   const after=sourceSnapshot(source);
   check(json(after.files)===json(before.files)&&[...before.nodes].every(([p,n])=>after.nodes.get(p).device===n.device&&after.nodes.get(p).inode===n.inode&&after.nodes.get(p).mode===n.mode),signal+' preserves source bytes, mode and inode identity');
  }
 }finally{removeOwnedTree(tmp,identity);}
 return {checks,signals:['SIGINT','SIGTERM'],fixture_cleaned:true};
}
function selfTest(){
 const tmp=fs.mkdtempSync(path.join(os.tmpdir(),'lkj-project-')),identity=stat(tmp),source=tmp+'/source';let checks=0;
 const check=(ok,label)=>{checks++;if(!ok)fail('self-test: '+label);};
 const rejected=(fn,label,reason)=>{let error;try{fn();}catch(e){error=e;}check(!!error,label);if(reason)check(reason.test(error.message),label+': expected refusal reason ('+error.message+')');};
 const write=(p,bytes)=>fs.writeFileSync(p,bytes),drop=p=>{const st=stat(p);if(st)removeOwnedTree(p,st);},git=(...a)=>gitOutput(source,...a);
 const commit=message=>{git('add','--','.');git('commit','--quiet','-m',message);};
 try{
  const legacy={root:'/fixture/source',source:{root:'/fixture/source',commit:'a'.repeat(40),origin:'https://example.invalid/specimen.git',object_format:'sha1'},files:[{path:'src/lib.rs',sha256:hash(''),bytes:0,git_blob:'e69de29bb2d1d6434b8b29ae775ad8c2e48c5391',git_mode:'100644'}],nodes:new Map([['src/lib.rs',{kind:'regular'}]])};
  check(hash(layout(legacy).json)==='1f747dc9f11deb92d2ab2942bb19257601eddd9f4f2555ddd26c478767674ab2','legacy regular-only manifest is byte-identical');
  fs.mkdirSync(source,{mode:0o700});fs.mkdirSync(source+'/src',{mode:0o700});
  git('-c','init.defaultBranch=main','init','--quiet');for(const [k,v] of [['user.name','Projection fixture'],['user.email','fixture@example.invalid'],['commit.gpgsign','false'],['core.autocrlf','false'],['remote.origin.url','https://example.invalid/specimen.git']])git('config',k,v);
  const rust='#[test]\nfn original_test() { assert_eq!(1, 1); }\n',asset=Buffer.from([0,255,97,115,115,101,116,10]),lib=source+'/src/lib.rs';
  write(lib,rust);write(source+'/asset.bin',asset);commit('fixture');
  const before=sourceSnapshot(source),first=project(source,tmp+'/first');
  check(first.tracked_files===2&&first.aliases===1,'complete tracked-file projection');check(verifyProjection(source,tmp+'/first').read_only,'read-only verification');
  const after=sourceSnapshot(source);check(json(before.files)===json(after.files),'original bytes preserved');check(before.nodes.get('src/lib.rs').mode===after.nodes.get('src/lib.rs').mode,'original mode preserved');
  check(first.manifest_sha256===project(source,tmp+'/second').manifest_sha256,'deterministic manifest');
  rejected(()=>project(source,tmp+'/first'),'existing DEST rejected');check(verifyProjection(source,tmp+'/first').aliases===1,'existing DEST not cleaned');
  fs.unlinkSync(tmp+'/first/src/lib.lkj');write(tmp+'/first/src/lib.lkj','changed alias\n');rejected(()=>verifyProjection(source,tmp+'/first'),'changed alias rejected');
  check(!!stat(tmp+'/first/src/lib.lkj'),'failed verify leaves destination');check(sourceSnapshot(source).nodes.get('src/lib.rs').sha256===after.nodes.get('src/lib.rs').sha256,'alias replacement preserves source');
  write(tmp+'/first/src/lib.lkj',rust);rejected(()=>verifyProjection(source,tmp+'/first'),'identical copy rejected');drop(tmp+'/first');drop(tmp+'/second');
  const mode=after.nodes.get('src/lib.rs').mode&0o777;
  fs.chmodSync(lib,mode|0o011);check(project(source,tmp+'/nonowner-added').aliases===1,'non-owner execute addition accepted');check((sourceSnapshot(source).nodes.get('src/lib.rs').mode&0o777)===(mode|0o011),'non-owner bits preserved');drop(tmp+'/nonowner-added');
  fs.chmodSync(lib,mode|0o100);rejected(()=>project(source,tmp+'/owner-added'),'owner execute addition rejected',/dirty tracked executable mode/);check(!stat(tmp+'/owner-added'),'mode refusal before creation');
  fs.chmodSync(lib,mode|0o111);commit('owner-execute fixture');fs.chmodSync(lib,mode|0o011);
  rejected(()=>project(source,tmp+'/owner-removed'),'owner execute removal rejected',/dirty tracked executable mode/);check(!stat(tmp+'/owner-removed'),'mode removal refusal before creation');
  fs.chmodSync(lib,mode|0o100);check(project(source,tmp+'/nonowner-removed').aliases===1,'non-owner execute removal accepted');check((sourceSnapshot(source).nodes.get('src/lib.rs').mode&0o777)===(mode|0o100),'owner-only bits preserved');drop(tmp+'/nonowner-removed');
  fs.chmodSync(lib,mode);commit('restore execute mode');write(lib,rust+'// dirty\n');rejected(()=>project(source,tmp+'/dirty'),'dirty bytes rejected');check(!stat(tmp+'/dirty'),'dirty refusal before creation');write(lib,rust);
  git('update-index','--assume-unchanged','--','src/lib.rs');write(lib,rust+'// hidden dirty\n');rejected(()=>project(source,tmp+'/hidden-dirty'),'assume unchanged checked');write(lib,rust);git('update-index','--no-assume-unchanged','--','src/lib.rs');
  write(source+'/asset.bin','staged asset\n');git('add','--','asset.bin');write(source+'/asset.bin',asset);rejected(()=>project(source,tmp+'/dirty-index'),'dirty index rejected');git('reset','--quiet','HEAD','--','asset.bin');
  write(source+'/src/lib.lkj','collision\n');commit('collision fixture');rejected(()=>project(source,tmp+'/collision'),'suffix collision rejected');check(!stat(tmp+'/collision'),'collision before creation');git('rm','--quiet','--','src/lib.lkj');commit('remove collision');
  fs.symlinkSync('lib.rs',source+'/src/link.rs');commit('symlink fixture');check(project(source,tmp+'/symlink').tracked_symlinks===1,'safe file symlink preserved');
  check(stat(tmp+'/symlink/src/link.lkj').isSymbolicLink()&&same(stat(source+'/src/link.rs'),stat(tmp+'/symlink/src/link.lkj'))&&fs.readlinkSync(tmp+'/symlink/src/link.lkj')==='lib.rs','symlink alias has original inode');drop(tmp+'/symlink');
  fs.mkdirSync(source+'/bench');fs.mkdirSync(source+'/test-pages');const html='<p>fixture</p>\n';write(source+'/test-pages/one.html',html);fs.symlinkSync('../test-pages',source+'/bench/test-pages');fs.symlinkSync('test-pages/one.html',source+'/bench/page');commit('internal directory links');
  check(project(source,tmp+'/internal').tracked_symlinks===3,'internal directory links preserved');
  const bound=JSON.parse(fs.readFileSync(tmp+'/internal/'+MANIFEST)),dir=bound.files.find(f=>f.path==='bench/test-pages');
  check(dir.kind==='symlink'&&dir.target==='../test-pages'&&dir.sha256===hash('../test-pages')&&dir.resolved_target==='test-pages'&&dir.target_kind==='directory','manifest binds link bytes and graph');
  write(source+'/test-pages/one.html',html+'changed');rejected(()=>verifyProjection(source,tmp+'/internal'),'target payload mutation rejected',/dirty tracked bytes/);write(source+'/test-pages/one.html',html);
  for(const target of ['../missing','../test-pages']){fs.unlinkSync(tmp+'/internal/bench/test-pages');fs.symlinkSync(target,tmp+'/internal/bench/test-pages');rejected(()=>verifyProjection(source,tmp+'/internal'),'copied symlink rejected',/not an original-source hardlink/);}
  fs.unlinkSync(tmp+'/internal/bench/test-pages');linkPayload(source+'/bench/test-pages',tmp+'/internal/bench/test-pages');fs.unlinkSync(source+'/bench/test-pages');fs.symlinkSync('../src',source+'/bench/test-pages');
  rejected(()=>verifyProjection(source,tmp+'/internal'),'source target text mutation rejected',/dirty tracked bytes/);fs.unlinkSync(source+'/bench/test-pages');linkPayload(tmp+'/internal/bench/test-pages',source+'/bench/test-pages');
  check(verifyProjection(source,tmp+'/internal').tracked_symlinks===3,'restored original links verify');drop(tmp+'/internal');
  for(const [name,target,reason] of [['absolute','/outside-fixture',/absolute/],['escape','../../outside-fixture',/escapes SOURCE/],['dangling','missing',/dangling or untracked/],['new-alias','../src/lib.lkj',/dangling or untracked/],['new-manifest','../projection.json',/dangling or untracked/],['directory-cycle','..',/cyclic directory symlink/]]){
   fs.symlinkSync(target,source+'/bench/bad');commit(name+' link');rejected(()=>project(source,tmp+'/'+name),name+' rejected',reason);check(!stat(tmp+'/'+name),name+' before creation');git('rm','--quiet','--','bench/bad');commit('remove '+name);
  }
  fs.symlinkSync('b',source+'/bench/a');fs.symlinkSync('a',source+'/bench/b');commit('chain cycle');rejected(()=>project(source,tmp+'/chain-cycle'),'chain cycle rejected',/cyclic symlink chain/);check(!stat(tmp+'/chain-cycle'),'chain refusal before creation');git('rm','--quiet','--','bench/a','bench/b');commit('remove chain cycle');
  const oldHead=sourceSnapshot(source).source.commit;write(lib,rust+'// replacement tree\n');commit('replacement');const newHead=sourceSnapshot(source).source.commit;
  git('replace',oldHead,newHead);git('update-ref','HEAD',oldHead);rejected(()=>project(source,tmp+'/replaced-head'),'replacement refs cannot relabel tree');check(!stat(tmp+'/replaced-head'),'replace refusal before creation');git('replace','--delete',oldHead);git('update-ref','HEAD',newHead);write(lib,rust);commit('remove replacement');
  let linked=false;rejected(()=>project(source,tmp+'/partial',(from,to)=>{linkPayload(from,to);if(stat(from).isSymbolicLink()){linked=true;fail('injected failure after linking symlink');}}),'mid-creation failure propagated');check(linked&&!stat(tmp+'/partial'),'own partial tree cleaned without traversal');check(sourceSnapshot(source).nodes.get('src/lib.rs').sha256===hash(rust),'source survives cleanup');
  rejected(()=>safePath('../escape.rs'),'parent escape rejected');rejected(()=>safePath('/escape.rs'),'absolute path rejected');
  // Unnormalized path checks must precede dot-dot collapse.
  fs.symlinkSync(source,tmp+'/source-link');rejected(()=>canonicalDirectory(tmp+'/source-link/../source','fixture'),'symlink component before dot-dot rejected',/symlink directory component/);
 }finally{removeOwnedTree(tmp,identity);}
 return {passed:true,checks,fixture_cleaned:true};
}
async function main(){
 decode(fs.readFileSync('/proc/self/cmdline'),'command-line arguments');
 let verify=false,self=false,help=false,end=false;const args=[];
 for(const arg of process.argv.slice(2)){if(end){args.push(arg);continue;}if(arg==='--'){end=true;continue;}if(arg==='--verify')verify=true;else if(arg==='--self-test')self=true;else if(arg==='--help')help=true;else if(arg.startsWith('-'))fail('use --help for usage');else args.push(arg);}
 if(help){process.stdout.write('Usage: node project.mjs SOURCE DEST\n       node project.mjs --verify SOURCE DEST\n       node project.mjs --self-test\nClean Git SOURCE and fresh DEST on the same filesystem are required. Tracked files,\nsafe internal symlink objects and .rs-to-.lkj aliases are hardlinked without copies.\nManifest records exact Git identity and byte hashes. --verify is read-only and\nrequires original-source inode identity. Keep both namespaces quiescent: cleanup\nchecks the created root identity but is not hardened against concurrent swaps.\n');return;}
 if(self){if(args.length||verify)fail('--self-test takes no SOURCE, DEST, or --verify');const report=selfTest(),signals=await signalSelfTest();report.checks+=signals.checks;report.signal_controls=signals.signals;process.stdout.write(json({self_test:report})+'\n');return;}
 if(args.length!==2)fail('SOURCE and DEST are required; use --help');
 process.stdout.write(json(verify?verifyProjection(...args):await projectInterruptibly(...args))+'\n');
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href){main().catch(e=>{process.stderr.write(e.message+'\n');process.exitCode=255;});}
