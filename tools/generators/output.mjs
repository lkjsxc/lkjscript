#!/usr/bin/env node
// Seeded byte/output-order probes through the same World API under all routes.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { run } from '../process.mjs';
import { runCases } from '../batch.mjs';
import { oracleCache, packResult, unpackResult } from '../oracle.mjs';
const [binary='target/debug/lkjscript',seedText='601',countText='50',output='target/generators']=process.argv.slice(2);
const bin=path.resolve(binary), seed=BigInt(seedText), count=Number(countText);
const engines=(process.env.LKJ_ENGINES || 'node,k').split(',');
let state=seed||1n;
function random(n) {state^=(state<<13n)&0xffffffffffffffffn;state^=state>>7n;state^=(state<<17n)&0xffffffffffffffffn;return Number(state%BigInt(n));}
const hash=file=>createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const generatorHash=createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).update(fs.readFileSync(new URL('../process.mjs',import.meta.url))).update(fs.readFileSync('bridge/output.rs')).digest('hex');
const sh=(command,args)=>(spawnSync(command,args,{encoding:'utf8'}).stdout||'').trim();
const metadata={compiler:sh('git',['rev-parse','HEAD']),compilerDirty:sh('git',['status','--porcelain','--untracked-files=no'])!=='',binarySha256:hash(bin),rustc:sh('rustc',['--version']),node:process.version};
const dir=path.resolve(output,`output-${seedText}`);fs.mkdirSync(dir,{recursive:true});
const sourceFiles=[],expected=[];
const prelude=`fn write(out: &mut Stdout, bytes: &[u8]) { out.write(bytes); }
struct Mark<'a> { out: &'a mut Stdout, byte: u8 }
impl<'a> Drop for Mark<'a> { fn drop(&mut self) { let b: [u8; 1] = [self.byte]; self.out.write(b.as_slice()); } }
struct OwnedMark { out: Stdout, byte: u8 }
impl Drop for OwnedMark { fn drop(&mut self) { let b: [u8; 1] = [self.byte]; self.out.write(b.as_slice()); } }
struct Marks { first: OwnedMark, second: OwnedMark }
`;
let oracle='#![allow(warnings)]\nmod __lkj_host {\n'+fs.readFileSync('bridge/output.rs','utf8')+'\n}\n';
for(let i=0;i<count;i++) {
 let source=prelude+'pub fn main(mut world: World) { let writer: Stdout = world.stdout(); let mut out: Stdout = writer;\n';
 const bytes=[];const chunks=1+random(8);
 for(let j=0;j<chunks;j++) {
   const choice=random(4);
   if(choice===0) {
     const data=Array.from({length:1+random(12)},()=>random(256));
     source+=`let b${j}: [u8; ${data.length}] = [${data.map(n=>n+'u8').join(',')}]; write(&mut out, b${j}.as_slice());\n`;bytes.push(...data);
   } else if(choice===1) {
     const value=random(256);source+=`{ let mark: Mark<'_> = Mark { out: &mut out, byte: ${value}u8 }; }\n`;bytes.push(value);
   } else if(choice===2) {
     const length=random(64);source+=`let mut v${j}: Vec<u8> = Vec::new(); let mut i${j}: usize = 0; while i${j} < ${length}usize { v${j}.push(i${j} as u8); i${j} = i${j} + 1; } out.write(v${j}.as_slice());\n`;bytes.push(...Array.from({length},(_,n)=>n));
   } else {
     const [a,b,c,d,marker]=Array.from({length:5},()=>random(256));
     const beforeMarker=random(2)===0;
     source+=`{ let mut pair=Marks { first: OwnedMark { out: world.stdout(), byte: ${a}u8 }, second: OwnedMark { out: world.stdout(), byte: ${b}u8 } };
let moved=pair.first;
pair.first=OwnedMark { out: world.stdout(), byte: ${c}u8 };
pair.second=OwnedMark { out: world.stdout(), byte: ${d}u8 };
`;
     bytes.push(b);
     if(beforeMarker){source+='drop(moved);\n';bytes.push(a);}
     source+=`let marker=[${marker}u8]; out.write(&marker);
`;bytes.push(marker);
     if(!beforeMarker){source+='drop(moved);\n';bytes.push(a);}
     source+='}\n';bytes.push(c,d);
   }
 }
 const trapped=random(5)===0;if(trapped)source+='assert!(false);\n';source+='}\n';
 const file=path.join(dir,`c${i}.rs`);fs.writeFileSync(file,source);sourceFiles.push(file);expected.push({bytes:Buffer.from(bytes),ending:trapped?'R0005':'normal'});
 oracle+=`mod c${i} { use crate::__lkj_host::{World,Stdout};\n${source}}\n`;
}
oracle+='fn main() { match std::env::args().nth(1).unwrap().as_str() {\n'+sourceFiles.map((_,i)=>`"${i}" => __lkj_host::enter(c${i}::main),`).join('\n')+'\n_ => std::process::exit(2) } }\n';
const rustSource=path.join(dir,'oracle.rs'),rustBinary=path.join(dir,'oracle');fs.writeFileSync(rustSource,oracle);
const rustFlags=['--edition=2024','-C','overflow-checks=yes','-A','warnings'];
function ending(result,oracle) {
 if(result.error)return result.error.code==='ETIMEDOUT'?'timeout':'crash';
 if(result.status===0&&!result.signal)return 'normal';
 const text=result.stderr.toString();const machine=text.match(/\bM_[A-Z_]+\b/);
 if(machine)return machine[0];
 const code=text.match(oracle?/LKJ_RUST_TRAP (R[0-9]+)/:/: (R[0-9]+)\b/)?.[1];
 return code?(Number(code.slice(1))>=16?'R0004':code):'crash';
}
const references=await oracleCache({generator:generatorHash,seed:seedText,count,flags:rustFlags,source:oracle},async()=>{
const built=await run('rustc',[...rustFlags,rustSource,'-o',rustBinary]);
if(built.error||built.status!==0) {process.stderr.write(built.stderr||String(built.error));process.exit(1);}

const results=[];for(let i=0;i<count;i++)results.push(packResult(await run(rustBinary,[String(i)],{encoding:null})));return results;
});
const actualResults={};for(const engine of engines)actualResults[engine]=await runCases(bin,sourceFiles.map(path=>({path})),{engine,output:dir,encoding:null});
const tally=Object.fromEntries(engines.map(engine=>[engine,{}])),failures=[];
for(let i=0;i<count;i++) {
 const reference=unpackResult(references[i],null);
 for(const engine of engines) {
   const actual=actualResults[engine][i];
   const end=ending(actual,false),oracleEnd=ending(reference,true);
   const category=oracleEnd!==expected[i].ending||!reference.stdout.equals(expected[i].bytes)?'oracle-differs':end==='timeout'||end==='crash'?end:end.startsWith('M_')?'machine-error:'+end:end!==oracleEnd?'different-ending':!actual.stdout.equals(reference.stdout)?'different-output':end==='normal'?'equal-output-exit':'equal-output-trap';
   tally[engine][category]=(tally[engine][category]||0)+1;
   if(!category.startsWith('equal-'))failures.push({file:sourceFiles[i],engine,category,diagnostic:actual.stderr.toString()});
 }
}
const record={generator:'output',sha256:generatorHash,...metadata,rustFlags,seed:seedText,programs:count,tally};
console.log(JSON.stringify(record));fs.appendFileSync(path.join(output,'runs.jsonl'),JSON.stringify(record)+'\n');fs.writeFileSync(path.join(dir,'failures.json'),JSON.stringify(failures,null,2)+'\n');if(failures.length)process.exitCode=1;
