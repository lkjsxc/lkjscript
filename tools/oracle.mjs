// Reference results are immutable observations of source, rustc and flags.
// The key includes the generator/seed where supplied; compiler edits never
// invalidate an unchanged oracle. Infrastructure failures are not cached.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { run } from './process.mjs';
const rustc=(await run('rustc',['--version','--verbose'])).stdout;
export async function oracleCache(key,produce) {
 const hash=createHash('sha256').update(JSON.stringify({rustc,...key})).digest('hex');
 const file=path.resolve('target/oracle-cache',hash+'.json');
 if(fs.existsSync(file))return JSON.parse(fs.readFileSync(file,'utf8'));
 const value=await produce();
 fs.mkdirSync(path.dirname(file),{recursive:true});
 const temporary=file+'.'+process.pid;fs.writeFileSync(temporary,JSON.stringify(value));fs.renameSync(temporary,file);
 return value;
}
export function packResult(result) {
 if(result.error||result.signal)throw new Error('oracle process failed: '+(result.error?.message||result.signal));
 return {status:result.status,stdoutHex:Buffer.from(result.stdout).toString('hex'),stderrHex:Buffer.from(result.stderr).toString('hex'),
   ...(result.control===undefined?{}:{controlHex:Buffer.from(result.control).toString('hex')})};
}
export function unpackResult(result,encoding='utf8') {
 const stdout=Buffer.from(result.stdoutHex,'hex'),stderr=result.stderrHex===undefined?Buffer.from(result.stderr):Buffer.from(result.stderrHex,'hex');
 return {status:result.status,signal:null,error:undefined,stdout:encoding?stdout.toString(encoding):stdout,stderr:encoding?stderr.toString(encoding):stderr,
   ...(result.controlHex===undefined?{}:{control:Buffer.from(result.controlHex,'hex')})};
}

// Only the oracle wrapper writes the separate control descriptor. Guest stderr,
// including an apparent trap marker and exit 101, carries no termination evidence.
export function worldOutcome(result) {
 if(result.error||result.signal||!Number.isInteger(result.status)||!Buffer.isBuffer(result.control)||result.control.length!==24)return {kind:'crash'};
 const code=result.control.readBigUInt64LE(0),reserved=result.control.readBigUInt64LE(8),status=result.control.readBigUInt64LE(16);
 if(reserved!==0n)return {kind:'crash'};
 if(code===0n)return status<=255n&&result.status===Number(status)?{kind:'normal',status:Number(status)}:{kind:'crash'};
 return status===0n&&result.status===101?{kind:'trap',code:'R'+code.toString().padStart(4,'0')}:{kind:'crash'};
}
