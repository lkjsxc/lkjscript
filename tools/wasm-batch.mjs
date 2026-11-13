// One Node process, a fresh module instance and output buffer per case.
import fs from 'node:fs';
import { executeModule, resultText } from '../bridge/engine.mjs';
for(const job of JSON.parse(fs.readFileSync(process.argv[2],'utf8'))) {
 const chunks=[];
 const result=await executeModule(job,(_fd,bytes,start,length)=>{chunks.push(Buffer.from(bytes.subarray(start,start+length)));return length;});
 if(result.ok)chunks.push(Buffer.from(resultText(result.result,job.kind)));
 const stderr=result.ok?'':`${job.source}:${result.problem.line}: ${result.problem.code} ${result.problem.message}`;
 console.log(JSON.stringify({index:job.index,engine:"node",checkStatus:0,status:result.ok?0:1,stdoutHex:Buffer.concat(chunks).toString('hex'),stderr}));
}
