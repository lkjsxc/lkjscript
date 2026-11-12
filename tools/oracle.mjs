// Reference results are immutable observations of source, rustc and flags.
// The key includes the generator/seed where supplied; compiler edits never
// invalidate an unchanged oracle. Infrastructure failures are not cached.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
const rustc=spawnSync('rustc',['--version','--verbose'],{encoding:'utf8'}).stdout;
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
 return {status:result.status,stdoutHex:Buffer.from(result.stdout).toString('hex'),stderr:result.stderr.toString()};
}
export function unpackResult(result,encoding='utf8') {
 const stdout=Buffer.from(result.stdoutHex,'hex'),stderr=Buffer.from(result.stderr);
 return {status:result.status,signal:null,error:undefined,stdout:encoding?stdout.toString(encoding):stdout,stderr:encoding?stderr.toString(encoding):stderr};
}
