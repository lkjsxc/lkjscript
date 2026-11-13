#!/usr/bin/env node
// Compile once, run a standalone ELF, retain stage markers and wait4 resource
// observations under target/. Sizes are explicit probe inputs, never compiler limits.
import fs from 'node:fs';
import path from 'node:path';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {generateList} from './list_phases.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const args = process.argv.slice(2);
if(args.includes('--help') || args.length < 3) {
  console.log('usage: node tools/scale/owned.mjs <compiler> chain|depth|list|fib|push <size> [--repeat N] [--out target/owned]');
  process.exitCode = args.includes('--help') ? 0 : 2;
} else {
  const option = (name, fallback) => { const at=args.indexOf(name); return at<0 ? fallback : args[at+1]; };
  const binary=path.resolve(args[0]), family=args[1], size=Number(args[2]);
  const repeats=Number(option('--repeat','1')), directory=path.resolve(option('--out','target/owned'));
  if(!Number.isSafeInteger(size)||size<1||!Number.isSafeInteger(repeats)||repeats<1) throw Error('positive integer size/repeat required');
  if(!['chain','depth','list','fib','push'].includes(family)) throw Error('unknown owned-route family '+family);
  fs.mkdirSync(directory,{recursive:true});
  const prefix=path.join(directory,`${family}-${size}`), source=prefix+'.rs', elf=prefix+'.elf';
  const sha = data => createHash('sha256').update(data).digest('hex');
  const markers=family==='list'?['construct','sum','drop','done']:['run','done'];
  const main = expression => `pub fn main(mut world: World) {\n let mut out: Stdout = world.stdout();\n out.write("run\\n".as_bytes());\n ${expression}\n out.write("done\\n".as_bytes());\n}\n`;
  if(family==='chain') {
    const descriptor=fs.openSync(source,'w');
    let part='';
    for(let i=0;i<size;i++) {
      part+=`fn f${i}(x: i64) -> i64 { ${i?`f${i-1}(x) + 1`:'x'} }\n`;
      if(i%1024===1023) { fs.writeSync(descriptor,part);part=''; }
    }
    fs.writeSync(descriptor,part+main(`assert_eq!(f${size-1}(0), ${size-1});`));fs.closeSync(descriptor);
  } else if(family==='depth') {
    fs.writeFileSync(source,'fn depth(n: i64) -> i64 { if n == 0 { 0 } else { depth(n - 1) + 1 } }\n'+main(`assert_eq!(depth(${size}), ${size});`));
  } else if(family==='list') fs.writeFileSync(source,generateList(size));
  else if(family==='fib') {
    let a=0n,b=1n;for(let n=0;n<size;n++) [a,b]=[b,a+b];
    fs.writeFileSync(source,'fn fib(n: i64) -> i64 { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }\n'+main(`assert_eq!(fib(${size}), ${a}i64);`));
  } else {
    const expected=BigInt(size)*(BigInt(size)-1n)/2n;
    fs.writeFileSync(source,main(`let mut values: Vec<i64> = Vec::new(); let mut i: i64 = 0;
 while i < ${size} { values.push(i); i += 1; }
 let mut sum: i64 = 0; let mut j: usize = 0;
 while j < values.len() { sum += values[j]; j += 1; }
 assert_eq!(sum, ${expected}i64);`));
  }
  function host() {
    const text=fs.readFileSync('/proc/meminfo','utf8');
    const bytes=name=>Number(text.match(new RegExp('^'+name+':\\s+(\\d+)','m'))?.[1]||0)*1024;
    const groups=fs.readFileSync('/proc/self/cgroup','utf8');
    const relative=groups.split('\n').find(line=>line.startsWith('0::'))?.slice(3);
    const cgroups=[];
    if(relative!==undefined) {
      let current=path.join('/sys/fs/cgroup',relative);
      for(;;) {
        const values={path:current};
        for(const name of ['memory.max','memory.current','memory.swap.max','memory.swap.current']) {
          try { values[name]=fs.readFileSync(path.join(current,name),'utf8').trim(); }
          catch(error) { if(error.code!=='ENOENT') throw error; }
        }
        cgroups.push(values);
        if(current==='/sys/fs/cgroup')break;
        const parent=path.dirname(current); if(parent===current)break;current=parent;
      }
    }
    return {physical_bytes:bytes('MemTotal'),available_bytes:bytes('MemAvailable'),swap_bytes:bytes('SwapTotal'),swap_free_bytes:bytes('SwapFree'),cgroups};
  }
  // Host-only measurement adapter. This is not linked into emitted programs.
  const helperSource=`#define _GNU_SOURCE
#include <errno.h>
#include <stdio.h>
#include <sys/resource.h>
#include <sys/wait.h>
#include <unistd.h>
int main(int argc, char **argv) {
  if (argc < 3) return 2;
  pid_t child = fork();
  if (child < 0) { perror("fork"); return 2; }
  if (child == 0) { execv(argv[2], argv + 2); perror("execv"); _exit(127); }
  struct rusage usage; int status;
  while (wait4(child, &status, 0, &usage) < 0) { if (errno != EINTR) { perror("wait4"); return 2; } }
  FILE *out = fopen(argv[1], "w"); if (!out) { perror("fopen"); return 2; }
  int code = WIFEXITED(status) ? WEXITSTATUS(status) : -1;
  int signal = WIFSIGNALED(status) ? WTERMSIG(status) : 0;
  fprintf(out, "{\\"exit_code\\":%d,\\"signal\\":%d,\\"peak_rss_bytes\\":%ld,\\"user_ms\\":%.3f,\\"system_ms\\":%.3f,\\"minor_faults\\":%ld,\\"major_faults\\":%ld}\\n", code, signal, usage.ru_maxrss*1024L,
    usage.ru_utime.tv_sec*1000.0+usage.ru_utime.tv_usec/1000.0, usage.ru_stime.tv_sec*1000.0+usage.ru_stime.tv_usec/1000.0, usage.ru_minflt, usage.ru_majflt);
  fclose(out); return code == 0 && signal == 0 ? 0 : 1;
}
`;
  const helper=path.join(directory,'wait4-'+sha(helperSource).slice(0,16));
  if(!fs.existsSync(helper)) {
    fs.writeFileSync(helper+'.c',helperSource);
    await new Promise((resolve,reject)=>{
      const child=spawn('cc',['-O2','-o',helper,helper+'.c'],{stdio:'inherit'});
      child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(Error('measurement helper build failed: '+code)));
    });
  }
  const active=new Set();
  for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>{
    for(const child of active)try { process.kill(-child.pid,signal); } catch(error) { if(error.code!=='ESRCH')throw error; }
    process.exitCode=signal==='SIGINT'?130:143;
  });
  function processMemory(pid) {
    let rss=0,peak=0;const pending=[pid];
    while(pending.length) {
      const current=pending.pop();
      try {
        const status=fs.readFileSync(`/proc/${current}/status`,'utf8');
        rss+=Number(status.match(/^VmRSS:\s+(\d+)/m)?.[1]||0)*1024;
        peak=Math.max(peak,Number(status.match(/^VmHWM:\s+(\d+)/m)?.[1]||0)*1024);
        const children=fs.readFileSync(`/proc/${current}/task/${current}/children`,'utf8').trim();
        if(children)pending.push(...children.split(/\s+/).map(Number));
      } catch(error) { if(!['ENOENT','ESRCH'].includes(error.code))throw error; }
    }
    return {sampled_tree_rss_bytes:rss,sampled_process_peak_rss_bytes:peak};
  }
  async function measured(name,command,arguments_,phaseMarkers=[]) {
    const stem=prefix+'.'+name, started=performance.now(), before=host();
    fs.rmSync(stem+'.usage.json',{force:true});
    const stdout=fs.openSync(stem+'.stdout','w'), stderr=fs.openSync(stem+'.stderr','w');
    const child=spawn(helper,[stem+'.usage.json',command,...arguments_],{detached:true,env:{...process.env,LKJSCRIPT_TIMINGS:'1'},stdio:['ignore','pipe','pipe']});
    active.add(child);
    const phases=[];let current, boundary=started, pending='', found=[], peak=0;
    const sample=()=>{const memory=processMemory(child.pid);peak=Math.max(peak,memory.sampled_tree_rss_bytes);return memory;};
    const interval=setInterval(sample,100);
    child.stdout.on('data',chunk=>{
      fs.writeSync(stdout,chunk);pending+=chunk.toString();
      let end;
      while((end=pending.indexOf('\n'))>=0) {
        const line=pending.slice(0,end);pending=pending.slice(end+1);
        if(phaseMarkers.includes(line)) {
          const now=performance.now(), memory=sample();
          if(current)phases.push({phase:current,completed:true,wall_ms:now-boundary,...memory});
          found.push(line);current=line==='done'?undefined:line;boundary=now;
          console.log(JSON.stringify({family,size,stage:name,marker:line,elapsed_ms:now-started,...memory}));
        }
      }
    });
    child.stderr.on('data',chunk=>fs.writeSync(stderr,chunk));
    const completion=await new Promise(resolve=>{
      child.on('error',error=>resolve({error:error.message}));
      child.on('close',(status,signal)=>resolve({status,signal}));
    });
    clearInterval(interval);active.delete(child);fs.closeSync(stdout);fs.closeSync(stderr);
    const wall=performance.now()-started;
    if(current)phases.push({phase:current,completed:false,wall_ms:performance.now()-boundary});
    const usage=fs.existsSync(stem+'.usage.json')?JSON.parse(fs.readFileSync(stem+'.usage.json','utf8')):{};
    const output=fs.readFileSync(stem+'.stdout','utf8');
    const stages=fs.readFileSync(stem+'.stderr','utf8').split('\n').filter(line=>line.startsWith('LKJ_TIMING ')).map(line=>JSON.parse(line.slice(11)));
    return {...completion,...usage,wall_ms:wall,sampled_peak_tree_rss_bytes:peak,host_before:before,host_after:host(),phases,
      markers:found,output,compile_stages:stages,ok:!completion.error&&completion.status===0&&(!phaseMarkers.length||output===phaseMarkers.join('\n')+'\n')};
  }
  const metadata={tool:'owned-route',command:process.argv.slice(1),family,size,seed:'deterministic',repeats,
    binary_sha256:sha(fs.readFileSync(binary)),tool_sha256:sha(fs.readFileSync(fileURLToPath(import.meta.url))),
    source_bytes:fs.statSync(source).size,source_sha256:sha(fs.readFileSync(source))};
  const build=await measured('build',binary,['build',source,'--engine','native','--entry','main','-o',elf]);
  const samples=[];
  if(build.ok)for(let repeat=0;repeat<repeats;repeat++) {
    const sample=await measured('run'+repeat,elf,[],markers);samples.push(sample);if(!sample.ok)break;
  }
  const result={...metadata,outcome:build.ok&&samples.length===repeats&&samples.every(x=>x.ok)?'ok':'failed',build,samples};
  if(build.ok) { result.emitted_bytes=fs.statSync(elf).size;result.elf_sha256=sha(fs.readFileSync(elf)); }
  fs.writeFileSync(prefix+'.json',JSON.stringify(result,null,2)+'\n');
  console.log(JSON.stringify(result));
  if(result.outcome!=='ok')process.exitCode=1;
}
