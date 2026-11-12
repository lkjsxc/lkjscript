#!/usr/bin/env node
// Phase probe for the kit's boxed list: normal checked compilation, World byte
// markers, and the same worker construction/output bridges as the Node route.
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import {createHash} from 'node:crypto';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {isMainThread, parentPort, workerData} from 'node:worker_threads';
import {run} from '../process.mjs';

const self = fileURLToPath(import.meta.url);
const root = path.resolve(path.dirname(self), '../..');
const digest = data => createHash('sha256').update(data).digest('hex');
const expectedOutput = 'construct\nsum\ndrop\ndone\n';
const median = values => [...values].sort((a,b) => a-b)[Math.floor(values.length / 2)];
const record = value => JSON.stringify(value, (_, item) => typeof item === 'bigint' ? item.toString() : item);

export function generateList(depth) {
  if (!Number.isSafeInteger(depth) || depth < 1) throw new Error('depth must be a positive safe integer');
  return `struct N { next: Option<Box<N>>, v: i64 }
fn sum(list: &Option<Box<N>>) -> i64 {
    match list { Some(node) => node.v + sum(&node.next), None => 0 }
}
pub fn main(mut world: World) {
    let mut out: Stdout = world.stdout();
    out.write("construct\\n".as_bytes());
    let mut head: Option<Box<N>> = None;
    let mut i: i64 = 0;
    while i < ${depth} {
        head = Some(Box::new(N { next: head, v: 1 }));
        i = i + 1;
    }
    out.write("sum\\n".as_bytes());
    let value: i64 = sum(&head);
    assert_eq!(value, ${depth});
    out.write("drop\\n".as_bytes());
    drop(head);
    out.write("done\\n".as_bytes());
}
`;
}

function memoryObservation() {
  const memory = fs.readFileSync('/proc/meminfo', 'utf8');
  const field = name => Number(memory.match(new RegExp('^' + name + ':\\s+(\\d+)', 'm'))?.[1] || 0) * 1024;
  return {physical_bytes: field('MemTotal'), available_bytes: field('MemAvailable'),
    swap_bytes: field('SwapTotal'), swap_free_bytes: field('SwapFree'), load_average: os.loadavg()};
}
function processObservation() {
  const text = fs.readFileSync('/proc/self/status', 'utf8');
  const field = name => Number(text.match(new RegExp('^' + name + ':\\s+(\\d+)', 'm'))?.[1] || 0) * 1024;
  return {rss_bytes:field('VmRSS'), high_water_rss_bytes:field('VmHWM'),
    mapped_address_bytes:field('VmSize'), swap_bytes:field('VmSwap')};
}
// Mirror bridge/run.mjs's resource-only policy without importing that CLI's
// entry point. Stack bytes reserve address space; they are not committed RSS.
function stackReservation() {
  const memory = memoryObservation();
  const groups = fs.readFileSync('/proc/self/cgroup', 'utf8');
  const group = groups.split('\n').find(line => line.startsWith('0::'))?.slice(3);
  const bound = (name, extent) => {
    if (group === undefined) return extent;
    let directory = path.join('/sys/fs/cgroup', group);
    for (;;) {
      try {
        const text = fs.readFileSync(path.join(directory,name),'utf8').trim();
        if (/^[0-9]+$/.test(text)) extent = Math.min(extent, Number(text));
      } catch(error) { if(error.code !== 'ENOENT') throw error; }
      if(directory === '/sys/fs/cgroup') return extent;
      const parent=path.dirname(directory);
      if(parent===directory) return extent;
      directory=parent;
    }
  };
  let bytes=bound('memory.max',memory.physical_bytes)+bound('memory.swap.max',memory.swap_bytes);
  const limit=fs.readFileSync('/proc/self/limits','utf8').match(/^Max address space\s+(\d+)/m);
  if(limit) bytes=Math.min(bytes,Math.max(0,Number(limit[1])-processObservation().mapped_address_bytes)/2);
  if(!Number.isSafeInteger(Math.floor(bytes)) || bytes<=0) throw new Error('no representable worker stack reservation');
  return Math.floor(bytes);
}
const usageDifference=(end,start)=>({
  user_cpu_ms:(end.userCPUTime-start.userCPUTime)/1000,
  system_cpu_ms:(end.systemCPUTime-start.systemCPUTime)/1000,
  minor_faults:end.minorPageFault-start.minorPageFault,
  major_faults:end.majorPageFault-start.majorPageFault,
  involuntary_switches:end.involuntaryContextSwitches-start.involuntaryContextSwitches,
  voluntary_switches:end.voluntaryContextSwitches-start.voluntaryContextSwitches,
});

export function summarizeGc(output) {
  let phase='startup'; const result={};
  for(const line of output.split('\n')) {
    if(['construct','sum','drop','done'].includes(line)) { phase=line;continue; }
    const match=line.match(/(Scavenge|Mark-Compact|Mark-sweep).*?, ([\d.]+) \/ ([\d.]+) ms/);
    if(!match) continue;
    const group=result[phase]??={count:0,pause_ms:0,external_pressure_count:0,kinds:{}};
    group.count++; group.pause_ms+=Number(match[2]);
    group.external_pressure_count+=Number(line.includes('external memory pressure'));
    group.kinds[match[1]]=(group.kinds[match[1]]||0)+1;
  }
  return result;
}

async function workerRun() {
  const {writeBytes}=await import(pathToFileURL(path.join(workerData.bridge,'output.mjs')).href);
  const started=performance.now();
  const bytes=fs.readFileSync(workerData.wasm);
  const module=await WebAssembly.compile(bytes);
  let instance, output='', pending='', current, boundary, usage, observation;
  const phases=[], markers=[];
  const mark=name=>{
    const now=performance.now(), endUsage=process.resourceUsage();
    const state={...processObservation(), wasm_memory_bytes:instance.exports.memory.buffer.byteLength};
    if(current) phases.push({phase:current, completed:true, wall_ms:now-boundary,
      ...usageDifference(endUsage,usage), before:observation, after:state});
    markers.push(name);
    if(name!==['construct','sum','drop','done'][markers.length-1]) throw new Error('unexpected phase marker '+name);
    current=name==='done'?undefined:name;
    observation=state; usage=process.resourceUsage(); boundary=performance.now();
  };
  try {
    instance=await WebAssembly.instantiate(module,{lkjscript:{stdout(pointer,length,line) {
      return writeBytes(instance.exports.memory,pointer,length,line,(fd,buffer,offset,count)=>{
        const copied=buffer.subarray(offset,offset+count);
        const written=fs.writeSync(fd,copied);
        output+=copied.subarray(0,written).toString('utf8');
        pending+=copied.subarray(0,written).toString('utf8');
        let newline;
        while((newline=pending.indexOf('\n'))>=0) { mark(pending.slice(0,newline)); pending=pending.slice(newline+1); }
        return written;
      });
    }}});
    const loadMs=performance.now()-started, runStarted=performance.now();
    let error;
    try { instance.exports['fn:main'](0n); }
    catch(problem) {
      const code=instance.exports.__lkj_trap_code.value;
      error={code:code!==0n?'R'+code.toString().padStart(4,'0'):'E_ENGINE',
        line:Number(instance.exports.__lkj_trap_line.value),message:problem.message};
    }
    if(!error && output!==expectedOutput) error={code:'E_PROBE',message:'missing or extra World output'};
    if(error && current) phases.push({phase:current,completed:false,wall_ms:performance.now()-boundary,
      ...usageDifference(process.resourceUsage(),usage),before:observation,
      after:{...processObservation(),wasm_memory_bytes:instance.exports.memory.buffer.byteLength}});
    parentPort.postMessage({ok:!error,error,output,phases,engine_load_ms:loadMs,
      run_ms:performance.now()-runStarted,stopped_phase:error?(current||'entry'):'done',
      peak_rss_bytes:process.resourceUsage().maxRSS*1024,final:{...processObservation(),wasm_memory_bytes:instance.exports.memory.buffer.byteLength}});
    if(error) process.exitCode=1;
  } catch(error) {
    parentPort.postMessage({ok:false,error:{code:error.code||'E_ENGINE',message:error.message},
      phases,stopped_phase:current||'engine_load',peak_rss_bytes:process.resourceUsage().maxRSS*1024});
    process.exitCode=1;
  }
}
async function execute(wasm,bridge) {
  const {startWorker}=await import(pathToFileURL(path.join(bridge,'worker.mjs')).href);
  const requested=stackReservation(), started=performance.now();
  const {worker,stackBytes}=startWorker(new URL(import.meta.url),{workerData:{wasm,bridge}},requested);
  const outcome=await new Promise((resolve,reject)=>{
    let result, failure;
    worker.on('message',value=>{result=value;});
    worker.on('error',error=>{failure=error;});
    worker.on('exit',code=>{
      if(failure) reject(failure);
      else if(!result) reject(new Error('worker exited without a phase record: '+code));
      else if(code!==0 && result.ok) reject(new Error('worker failed after its phase record'));
      else resolve(result);
    });
  });
  outcome.requested_stack_bytes=requested; outcome.stack_reservation_bytes=stackBytes;
  outcome.worker_lifetime_ms=performance.now()-started;
  process.stdout.write('LKJ_LIST_PHASES '+record(outcome)+'\n');
  if(!outcome.ok) process.exitCode=1;
}
async function main() {
  const args=process.argv.slice(2);
  const option=(name,fallback)=>{const index=args.indexOf(name);return index<0?fallback:args[index+1];};
  if(args.includes('--execute')) {
    await execute(path.resolve(option('--execute')),path.resolve(option('--bridge',path.join(root,'bridge')))); return;
  }
  if(args.includes('--help')) {
    console.log('usage: node tools/scale/list_phases.mjs <compiler-binary> [--sizes 1250000,2500000,5000000,10000000] [--repeat 3] [--out target/list-phases] [--bridge DIR] [--timeout-ms 180000] [--trace-gc]');
    return;
  }
  const binary=path.resolve(args[0]||'target/release/lkjscript');
  const bridge=path.resolve(option('--bridge',path.resolve(path.dirname(binary),'../../bridge')));
  const sizes=option('--sizes','1250000,2500000,5000000,10000000').split(',').map(Number);
  const repeats=Number(option('--repeat','3')), timeout=Number(option('--timeout-ms','180000'));
  const nodeFlags=args.includes('--trace-gc')?['--trace-gc']:[];
  if(!Number.isSafeInteger(repeats)||repeats<1||!Number.isSafeInteger(timeout)||timeout<1) throw new Error('invalid operational repeat/timeout option');
  const directory=path.resolve(option('--out','target/list-phases'));
  fs.mkdirSync(directory,{recursive:true});
  const git=await run('git',['-C',path.dirname(binary),'rev-parse','HEAD']);
  const dirty=await run('git',['-C',path.dirname(binary),'status','--porcelain','--untracked-files=no']);
  const metadata={tool:'list-phases',compiler:git.stdout.trim(),compiler_dirty:dirty.stdout.trim()!=='',binary_sha256:digest(fs.readFileSync(binary)),
    probe_sha256:digest(fs.readFileSync(self)),node:process.version,bridge,
    bridge_hashes:Object.fromEntries(['output.mjs','worker.mjs','run.mjs'].map(name=>[name,digest(fs.readFileSync(path.join(bridge,name)))])),
    command:process.argv.slice(1),repeats,node_flags:nodeFlags};
  let previous;
  for(const n of sizes) {
    const source=generateList(n), file=path.join(directory,'list-'+n+'.rs'), wasm=file.replace(/\.rs$/,'.wasm');
    fs.writeFileSync(file,source);
    const build=await run(binary,['build',file,'--entry','main','-o',wasm],{timeout,env:{...process.env,LKJSCRIPT_TIMINGS:'1'}});
    const compile=build.stderr.split('\n').filter(line=>line.startsWith('LKJ_TIMING ')).map(line=>JSON.parse(line.slice(11)));
    if(build.error||build.status!==0) {
      const failure={...metadata,n,source_bytes:Buffer.byteLength(source),outcome:'compile-failed',compile,error:build.error?.message||build.stderr};
      console.log(record(failure));fs.appendFileSync(path.join(directory,'runs.jsonl'),record(failure)+'\n');process.exitCode=1;break;
    }
    const samples=[];
    for(let repeat=0;repeat<repeats;repeat++) {
      const host_before=memoryObservation();
      const result=await run(process.execPath,[...nodeFlags,self,'--execute',wasm,'--bridge',bridge],{timeout});
      const line=result.stdout.split('\n').find(line=>line.startsWith('LKJ_LIST_PHASES '));
      const sample=line?JSON.parse(line.slice(16)):{ok:false,error:{code:result.error?.code||'E_ENGINE',message:result.stderr},stopped_phase:'worker'};
      sample.host_before=host_before; sample.host_after=memoryObservation();
      if(nodeFlags.length) sample.gc=summarizeGc(result.stdout);
      sample.status=result.status; sample.signal=result.signal;
      if(result.error) sample.process_error=result.error.code||result.error.message;
      samples.push(sample);
      fs.appendFileSync(path.join(directory,'samples.jsonl'),record({...metadata,n,repeat,...sample})+'\n');
      if(!sample.ok||result.status!==0||result.error) break;
    }
    const ok=samples.length===repeats && samples.every(sample=>sample.ok && sample.status===0 && !sample.process_error);
    const summary={...metadata,n,source_bytes:Buffer.byteLength(source),source_sha256:digest(source),emitted_bytes:fs.statSync(wasm).size,wasm_sha256:digest(fs.readFileSync(wasm)),compile,
      outcome:ok?'ok':'failed',samples:samples.length,
      medians:Object.fromEntries(['run_ms','engine_load_ms','peak_rss_bytes'].map(name=>[name,median(samples.map(sample=>sample[name]||0))])),
      phases:Object.fromEntries(['construct','sum','drop'].map(name=>{
        const entries=samples.map(sample=>sample.phases?.find(phase=>phase.phase===name)).filter(Boolean);
        return [name,entries.length?{completed:entries.every(entry=>entry.completed!==false),wall_ms:median(entries.map(entry=>entry.wall_ms)),user_cpu_ms:median(entries.map(entry=>entry.user_cpu_ms)),
          system_cpu_ms:median(entries.map(entry=>entry.system_cpu_ms)),minor_faults:median(entries.map(entry=>entry.minor_faults)),
          major_faults:median(entries.map(entry=>entry.major_faults)),wasm_memory_bytes:median(entries.map(entry=>entry.after.wasm_memory_bytes)),
          high_water_rss_bytes:median(entries.map(entry=>entry.after.high_water_rss_bytes))}:null];
      }))};
    if(!ok) {
      const sample=samples.find(sample=>!sample.ok||sample.status!==0||sample.process_error)||samples.at(-1);
      summary.failure={stopped_phase:sample.stopped_phase,error:sample.error,process_error:sample.process_error,status:sample.status,signal:sample.signal};
    }
    if(nodeFlags.length) summary.gc=Object.fromEntries(['startup','construct','sum','drop','done'].map(phase=>[phase,
      Object.fromEntries(['count','pause_ms','external_pressure_count'].map(name=>[name,median(samples.map(sample=>sample.gc?.[phase]?.[name]||0))]))]));
    if(previous) summary.ratios={input:n/previous.n,total:summary.medians.run_ms/previous.medians.run_ms,
      ...Object.fromEntries(['construct','sum','drop'].filter(name=>summary.phases[name]&&previous.phases[name])
        .map(name=>[name,summary.phases[name].wall_ms/previous.phases[name].wall_ms]))};
    console.log(record(summary));fs.appendFileSync(path.join(directory,'runs.jsonl'),record(summary)+'\n');previous=summary;
    if(!ok) { process.exitCode=1;break; }
  }
}

if(!isMainThread) await workerRun();
else if(process.argv[1]&&pathToFileURL(path.resolve(process.argv[1])).href===import.meta.url) {
  try { await main(); } catch(error) { console.error(error.stack||error.message);process.exitCode=1; }
}
