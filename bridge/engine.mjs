// Shared WebAssembly instance execution for the interactive host and test batch.
import { readFile } from 'node:fs/promises';
import { writeSync } from 'node:fs';
import { writeBytes } from './output.mjs';
const messages={1:'integer overflow',2:'division by zero',3:'index out of bounds',4:'explicit trap',5:'assertion failed',6:'allocation failed',7:'host output failure'};
export function describe(problem) {return {message:problem instanceof Error?problem.message:String(problem),code:problem.code??'E_ENGINE',line:problem.line??0};}
export function resultText(result,kind) {
 if(kind==='f64'){const data=new DataView(new ArrayBuffer(8));data.setBigUint64(0,BigInt.asUintN(64,result),true);return String(data.getFloat64(0,true))+'\n';}
 if(kind==='bool')return (result===0n?'false':'true')+'\n';
 if(kind==='unit'||kind==='--unit')return '';
 return (kind==='u64'||kind==='usize'?BigInt.asUintN(64,result):result).toString()+'\n';
}
export async function executeModule({path,entry,args=[]},write=writeSync) {
 const metrics={};let stage='module_read',stageStarted=performance.now();
 try {
  const bytes=await readFile(path);metrics.module_read_ms=performance.now()-stageStarted;
  stage='engine_load';stageStarted=performance.now();
  const module=await WebAssembly.compile(bytes);
  const metadata=WebAssembly.Module.customSections(module,'lkjscript.traps');
  const userMessages=metadata.length?JSON.parse(Buffer.from(metadata[0]).toString('utf8')):[];
  const instance=await WebAssembly.instantiate(module,{lkjscript:{stdout(pointer,length,line){return writeBytes(instance.exports.memory,pointer,length,line,write);}}});
  metrics.engine_load_ms=performance.now()-stageStarted;
  const fn=instance.exports['fn:'+entry];if(typeof fn!=='function')throw new Error('entry function not found: '+entry);
  stage='run';stageStarted=performance.now();
  try {
   instance.exports.__lkj_trap_code.value=0n;instance.exports.__lkj_trap_line.value=0n;
   const result=fn(...args.map(BigInt));
   return {ok:true,result,metrics:{...metrics,run_ms:performance.now()-stageStarted,stopped_stage:'done'}};
  } catch(problem) {
   const code=instance.exports.__lkj_trap_code.value;
   if(problem instanceof WebAssembly.RuntimeError&&code!==0n){const trapped=new Error(messages[Number(code)]??userMessages[Number(code)-16]??'trap');trapped.code='R'+code.toString().padStart(4,'0');trapped.line=instance.exports.__lkj_trap_line.value;throw trapped;}
   throw problem;
  }
 }catch(problem){metrics[stage+'_ms']??=performance.now()-stageStarted;return {ok:false,problem:describe(problem),metrics:{...metrics,stopped_stage:stage}};}
}
