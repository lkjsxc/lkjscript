const fs=require('fs'),crypto=require('crypto');
const [source,prefix]=process.argv.slice(2);
if(!/^[0-9a-f]{40}$/.test(source||'')||!/^source-accept-v[0-9]+-$/.test(prefix||''))throw Error('exact source and owned acceptance prefix required');
const base='.artifacts/selfhost/'+prefix;
const root='.artifacts/lkjscript-dev/check/transient/workspace-tests';
const observed=new Map();let watcher=null;
function scan(){
 try {
  if(fs.existsSync(root)){
   if(!watcher)watcher=fs.watch(root,scan);
   for(const name of fs.readdirSync(root).filter(n=>n.endsWith('-run.stdout'))){
    const path=root+'/'+name,size=fs.statSync(path).size;
    if(observed.get(name)?.bytes===size)continue;
    const fd=fs.openSync(path,'r');let tail;
    try {tail=Buffer.alloc(Math.min(8192,size));fs.readSync(fd,tail,0,tail.length,size-tail.length);}finally{fs.closeSync(fd);}
    const results=(tail.toString().match(/^test result: .*/gm)||[]).filter(l=>/; [01] filtered out;/.test(l));
    if(!results.length)continue;
    const bytes=fs.readFileSync(path),text=bytes.toString(),lines=text.split('\n');
    const record={file:name,terminal_result:results.at(-1),bytes:bytes.length,sha256:crypto.createHash('sha256').update(bytes).digest('hex'),
     selfhost:[...new Set(lines.filter(l=>/^test .*::selfhost_tests::.* \.\.\. (ok|FAILED)$/.test(l)))],
     native:[...new Set(lines.filter(l=>/^test .*::(native_elf_tests|native_filter_tests|native_derive_tests)::.* \.\.\. (ok|FAILED)$/.test(l)||/^test .*::stack_tests::native_elf_.* \.\.\. (ok|FAILED)$/.test(l)))],
     ignored:[...new Set(lines.filter(l=>/^test .* \.\.\. ignored/.test(l)))]};
    observed.set(name,record);
    fs.writeFileSync(base+'harnesses.json',JSON.stringify({source,scope:'Captured terminal harness output; official full receipt owns gate success and executable provenance',harnesses:[...observed.values()]},null,2)+'\n');
    if(record.terminal_result.startsWith('test result: FAILED')){
     const at=text.indexOf('\nfailures:\n');const failure=at<0?text.slice(-60000):text.slice(at);
     fs.writeFileSync(base+name.replace('-run.stdout','-failure.log'),failure.length>60000?failure.slice(0,30000)+'\n[compact failure excerpt]\n'+failure.slice(-29000):failure);
    }
    process.stdout.write(JSON.stringify({file:name,result:record.terminal_result,selfhost:record.selfhost.length,native:record.native.length})+'\n');
   }
  }
  if(fs.existsSync(base+'full-finished.json')){watcher?.close();clearInterval(timer);if(!observed.size)process.exitCode=1;}
 }catch(error){if(error.code!=='ENOENT'){watcher?.close();clearInterval(timer);process.stderr.write(String(error)+'\n');process.exitCode=1;}}
}
const timer=setInterval(scan,1000);scan();
