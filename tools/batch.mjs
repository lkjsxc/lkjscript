import fs from 'node:fs';
import path from 'node:path';
import { run } from './process.mjs';
let serial=0;
export async function runCases(binary,cases,{engine='k',output='target/batch-input',encoding='utf8',timeout=Number(process.env.LKJ_TEST_TIMEOUT_MS||30000),...options}={}) {
 fs.mkdirSync(output,{recursive:true});
 const manifest=path.resolve(output,`${process.pid}-${serial++}.tsv`);
 const rows=cases.map(c=>[c.mode||'run',c.entry||'',path.resolve(c.path)].map(value=>{if(/[\r\n\t]/.test(value))throw new Error('batch field contains a line delimiter');return value;}).join('\t'));
 fs.writeFileSync(manifest,rows.join('\n')+'\n');
 let result;try{result=await run(binary,['batch',manifest,'--engine',engine],{timeout,encoding:'utf8',...options});}finally{fs.rmSync(manifest,{force:true});}
 const results=new Array(cases.length);
 for(const line of result.stdout.split('\n').filter(Boolean)) {
  let record;try{record=JSON.parse(line);}catch{throw new Error('invalid batch output: '+line);}
  if(!Number.isInteger(record.index)||record.index<0||record.index>=cases.length||results[record.index])throw new Error('invalid or duplicate batch case index');
  const stdout=Buffer.from(record.stdoutHex,'hex'),stderr=Buffer.from(record.stderr);
  results[record.index]={...record,stdout:encoding?stdout.toString(encoding):stdout,stderr:encoding?stderr.toString(encoding):stderr,error:undefined,signal:null};
 }
 for(let i=0;i<cases.length;i++)if(!results[i])results[i]={checkStatus:null,status:result.status,stdout:encoding?'':Buffer.alloc(0),stderr:encoding?result.stderr:Buffer.from(result.stderr),error:result.error||new Error('batch ended without case '+i),signal:result.signal};
 if(cases.length&&(result.error||result.signal||result.status!==0)&&results.every(value=>!value.error)) {
  results[results.length-1].error=result.error||new Error('batch host failed after its final record: '+result.stderr);
 }
 return results;
}
