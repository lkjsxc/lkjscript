const fs=require('fs'),crypto=require('crypto');
const root='.artifacts/lkjscript-dev/check/transient/workspace-tests';
const source='108906b51259f00fae4c4fd1c53d16a05b3df1b5';
const start=Date.now();
const timer=setInterval(()=>{
  try {
    if(fs.existsSync(root))for(const name of fs.readdirSync(root).filter(n=>n.endsWith('-run.stdout'))){
      const bytes=fs.readFileSync(root+'/'+name),s=bytes.toString();
      if(!s.includes('::selfhost_tests::'))continue;
      const results=(s.match(/^test result: .*/gm)||[]).filter(line=>/; 0 filtered out;/.test(line));if(!results.length)continue;
      const lines=s.split('\n');const result={scope:'Completed library harness; overall fresh full receipt remains required',source,result:results.at(-1),selfhost:lines.filter(l=>/^test .*::selfhost_tests::.* \.\.\. ok$/.test(l)),native:lines.filter(l=>/^test .*::(native_elf_tests|native_filter_tests|native_derive_tests)::.* \.\.\. ok$/.test(l)||/^test .*::stack_tests::native_elf_.* \.\.\. ok$/.test(l)),ignored:lines.filter(l=>/^test .* \.\.\. ignored/.test(l)),stdout_bytes:bytes.length,stdout_sha256:crypto.createHash('sha256').update(bytes).digest('hex')};
      fs.writeFileSync('.artifacts/selfhost/artifact-official-library.json',JSON.stringify(result,null,2)+'\n');clearInterval(timer);process.stdout.write(JSON.stringify({result:result.result,selfhost:result.selfhost.length,native:result.native.length,ignored:result.ignored.length})+'\n');return;
    }
    if(fs.existsSync('.artifacts/selfhost/artifact-full-receipt.json')||Date.now()-start>3600000){clearInterval(timer);process.stderr.write('Library summary was not captured\n');process.exitCode=1;}
  }catch(e){if(e.code!=='ENOENT'){clearInterval(timer);process.stderr.write(String(e)+'\n');process.exitCode=1;}}
},1000);
