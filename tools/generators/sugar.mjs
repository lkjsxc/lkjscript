#!/usr/bin/env node
// The primary oracle is each generated sugar against an independently emitted
// expansion on every route. LKJ_SUGAR_RUST=0 keeps that oracle without rustc.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { run } from '../process.mjs';
import { runCases } from '../batch.mjs';
import { oracleCache, packResult, unpackResult } from '../oracle.mjs';
const [binary='target/debug/lkjscript',seedText='1301',countText='52',output='target/generators']=process.argv.slice(2);
const bin=path.resolve(binary),count=Number(countText),dir=path.resolve(output,`sugar-${seedText}`);
const engines=(process.env.LKJ_ENGINES||'k,node').split(',');
const compareRust=process.env.LKJ_SUGAR_RUST!=='0';
let state=BigInt(seedText)||1n;
function random(n){state^=(state<<13n)&0xffffffffffffffffn;state^=state>>7n;state^=(state<<17n)&0xffffffffffffffffn;return Number(state%BigInt(n));}
const prelude=`struct M { out: Stdout, b: u8 }
impl Drop for M { fn drop(&mut self) { self.out.write(&[self.b]); } }
struct P { a: M, b: M }
fn tick(world: &mut World, byte: u8) { let mut out = world.stdout(); out.write(&[byte]); }
fn make(world: &mut World, byte: u8) -> M { tick(world, byte+100); M { out: world.stdout(), b: byte } }
`;
function pair(index){
 const a=1+random(50),b=1+random(50),c=1+random(50),marker=60+random(30),yes=random(2)===0?'true':'false';
 let sugar='',expanded='',sugarHelpers='',expandedHelpers='';let family;
 switch(index%13){
  case 0:{family='if-let';const value=`if ${yes} { Some(make(&mut world,${a}u8)) } else { None }`;
   sugar=`if let Some(m) = ${value} { tick(&mut world,${marker}u8); drop(m); } else { tick(&mut world,${b}u8); }`;
   expanded=`match ${value} { Some(m) => { tick(&mut world,${marker}u8); drop(m); }, None => tick(&mut world,${b}u8) }`;break;}
  case 1:{family='while-let';const prefix=`let mut v: Vec<M> = Vec::new(); v.push(make(&mut world,${a}u8)); v.push(make(&mut world,${b}u8));`;
   sugar=prefix+`while let Some(m) = v.pop() { tick(&mut world,${marker}u8); drop(m); }`;
   expanded=prefix+`loop { match v.pop() { Some(m) => { tick(&mut world,${marker}u8); drop(m); }, None => break } }`;break;}
  case 2:{family='let-else';const header='fn take(world: &mut World, yes: bool) -> Option<M> { ';const value=`if yes { Some(make(world,${a}u8)) } else { None }`;const tail=` tick(world,${marker}u8); Some(m) }`;
   sugarHelpers=header+`let Some(m) = (${value}) else { tick(world,${b}u8); return None; };`+tail;
   expandedHelpers=header+`let m = match ${value} { Some(m) => m, None => { tick(world,${b}u8); return None; } };`+tail;
   sugar=expanded=`let answer=take(&mut world,${yes}); tick(&mut world,${c}u8); drop(answer);`;break;}
  case 3:{family='result-question';const header=`fn step(world: &mut World, yes: bool) -> Result<M,M> { let input: Result<M,M> = if yes { Ok(make(world,${a}u8)) } else { Err(make(world,${b}u8)) }; `;const tail=` tick(world,${marker}u8); Ok(m) }`;
   sugarHelpers=header+'let m = input?;'+tail;
   expandedHelpers=header+'let m = match input { Ok(m) => m, Err(e) => return Err(e) };'+tail;
   sugar=expanded=`let answer=step(&mut world,${yes}); tick(&mut world,${c}u8); drop(answer);`;break;}
  case 4:{family='option-question';const header=`fn step(world: &mut World, yes: bool) -> Option<M> { let input = if yes { Some(make(world,${a}u8)) } else { None }; `;const tail=` tick(world,${marker}u8); Some(m) }`;
   sugarHelpers=header+'let m = input?;'+tail;
   expandedHelpers=header+'let m = match input { Some(m) => m, None => return None };'+tail;
   sugar=expanded=`let answer=step(&mut world,${yes}); tick(&mut world,${c}u8); drop(answer);`;break;}
  case 5:{family='or-guard-order';const left=random(2),right=random(2),target=random(2);const prefix=`let left=${left}i64; let right=${right}i64;`;
   sugar=prefix+`match (left,right) { (0,x) | (x,0) if { tick(&mut world,${marker}u8); x == ${target}i64 } => tick(&mut world,${a}u8), _ => tick(&mut world,${b}u8) }`;
   expanded=prefix+`if left == 0 && { tick(&mut world,${marker}u8); right == ${target}i64 } { tick(&mut world,${a}u8); } else if right == 0 && { tick(&mut world,${marker}u8); left == ${target}i64 } { tick(&mut world,${a}u8); } else { tick(&mut world,${b}u8); }`;break;}
  case 6:{family='nested-pattern';const prefix=`let v: Option<Option<M>> = if ${yes} { Some(Some(make(&mut world,${a}u8))) } else { Some(None) };`;const found=`{ tick(&mut world,${marker}u8); drop(m); }`,missing=`tick(&mut world,${b}u8)`;
   sugar=prefix+`match v { Some(Some(m)) => ${found}, _ => ${missing} }`;
   expanded=prefix+`match v { Some(inner) => match inner { Some(m) => ${found}, None => ${missing} }, None => ${missing} }`;break;}
  case 7:{family='struct-update';const prefix=`let p=P { a:make(&mut world,${a}u8), b:make(&mut world,${b}u8) };`;
   sugar=prefix+`let q=P { b:make(&mut world,${c}u8), ..p }; tick(&mut world,${marker}u8); drop(q);`;
   expanded=prefix+`let next=make(&mut world,${c}u8); let q=P { a:p.a, b:next }; tick(&mut world,${marker}u8); drop(q);`;break;}
  case 8:{family='break-value';
   sugar=`let answer=loop { if ${yes} { tick(&mut world,${marker}u8); break ${a}i64; } break ${b}i64; }; tick(&mut world,answer as u8);`;
   expanded=`let answer: i64; loop { if ${yes} { tick(&mut world,${marker}u8); answer=${a}i64; break; } answer=${b}i64; break; } tick(&mut world,answer as u8);`;break;}
  case 9:{family='reference-pattern';const prefix=`let p=(${a}u8,${b}u8);`;
   sugar=prefix+'match &p { &(a,b) => { tick(&mut world,a); tick(&mut world,b); } }';
   expanded=prefix+'let a=p.0; let b=p.1; tick(&mut world,a); tick(&mut world,b);';break;}
  case 10:{family='named-binding';const prefix=`let p=P { a:make(&mut world,${a}u8), b:make(&mut world,${b}u8) };`;
   sugar=prefix+`let P { a, .. }=p; tick(&mut world,${marker}u8); drop(a);`;
   expanded=prefix+`let a=p.a; tick(&mut world,${marker}u8); drop(a);`;break;}
  case 11:{family='guard-keeps-owner';const target=random(2)?a:b;const prefix=`let v=Some(make(&mut world,${a}u8));`;const success=`{ tick(&mut world,${c}u8); drop(m); }`;
   sugar=prefix+`match v { Some(m) if { tick(&mut world,${marker}u8); m.b==${target}u8 } => ${success}, Some(m) => drop(m), None => () }`;
   expanded=prefix+`let yes=match &v { Some(m) => { tick(&mut world,${marker}u8); m.b==${target}u8 }, None => false }; if yes { match v { Some(m) => ${success}, None => () } } else { match v { Some(m) => drop(m), None => () } }`;break;}
  case 12:{family='shared-or-product';const width=4+random(8),values=Array.from({length:width},()=>random(3));const tuple='('+values.map(n=>n+'i64').join(',')+')';
   sugar=`let p=${tuple}; match p { (${values.map(()=>'0|1').join(',')}) => tick(&mut world,${a}u8), _ => tick(&mut world,${b}u8) }`;
   expanded=`let p=${tuple}; if ${values.map((_,i)=>`(p.${i}==0 || p.${i}==1)`).join(' && ')} { tick(&mut world,${a}u8); } else { tick(&mut world,${b}u8); }`;break;}
 }
 const tail=`\ntick(&mut world,${90+random(10)}u8);`+(random(9)===0?' assert!(false);':'');
 return {family,sugar:prelude+sugarHelpers+'\npub fn main(mut world:World) {\n'+sugar+tail+'\n}\n',expanded:prelude+expandedHelpers+'\npub fn main(mut world:World) {\n'+expanded+tail+'\n}\n'};
}
fs.mkdirSync(dir,{recursive:true});
const pairs=Array.from({length:count},(_,i)=>pair(i)),files=[];
for(let i=0;i<count;i++)for(const variant of ['sugar','expanded']){const file=path.join(dir,`${i}-${variant}.rs`);fs.writeFileSync(file,pairs[i][variant]);files.push(file);}
const generatorHash=createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).digest('hex');
let references;
if(compareRust){
 let source='#![allow(warnings)]\nmod host {\n'+fs.readFileSync('bridge/output.rs','utf8')+'\n}\n';
 for(let i=0;i<count;i++)for(const variant of ['sugar','expanded'])source+=`mod c${i}_${variant} { use crate::host::{World,Stdout};\n${pairs[i][variant]} }\n`;
 source+='fn main(){match std::env::args().nth(1).unwrap().as_str(){\n';
 for(let i=0;i<count;i++)for(const variant of ['sugar','expanded'])source+=`"${i}-${variant}"=>host::enter(c${i}_${variant}::main),\n`;
 source+='_=>std::process::exit(2)}}\n';
 const rustSource=path.join(dir,'oracle.rs'),rustBinary=path.join(dir,'oracle');fs.writeFileSync(rustSource,source);
 const flags=['--edition=2024','-C','overflow-checks=yes','-A','warnings'];
 references=await oracleCache({generator:generatorHash,seed:seedText,count,flags,source},async()=>{
  const built=await run('rustc',[...flags,rustSource,'-o',rustBinary]);if(built.error||built.status!==0)throw Error('invalid generated Rust: '+built.stderr);
  const results=[];for(let i=0;i<count;i++)for(const variant of ['sugar','expanded'])results.push(packResult(await run(rustBinary,[`${i}-${variant}`],{encoding:null})));return results;
 });
}
function ending(result,rust=false){
 if(result.error)return result.error.code==='ETIMEDOUT'?'timeout':'crash';
 if(result.checkStatus!==undefined&&result.checkStatus!==0)return 'rejected:'+(result.stderr.toString().match(/\bE_[A-Z_]+\b/)?.[0]||'unknown');
 if(result.status===0&&!result.signal)return 'normal';
 const text=result.stderr.toString(),machine=text.match(/\bM_[A-Z_]+\b/)?.[0];if(machine)return'machine:'+machine;
 const code=text.match(rust?/LKJ_RUST_TRAP (R[0-9]+)/:/: (R[0-9]+)\b/)?.[1];return code?(Number(code.slice(1))>=16?'R0004':code):'crash';
}
const tally={},failures=[];
for(const engine of engines){
 const results=await runCases(bin,files.map(path=>({path})),{engine,output:dir,encoding:null});tally[engine]={};
 for(let i=0;i<count;i++){
  const sugar=results[2*i],expanded=results[2*i+1],a=ending(sugar),b=ending(expanded);
  let category=a.startsWith('rejected:')?'sugar-'+a:b.startsWith('rejected:')?'expansion-'+b:a!==b?'different-ending':!sugar.stdout.equals(expanded.stdout)?'different-output':a==='normal'?'equal-output-exit':a.startsWith('R')?'equal-output-trap':a;
  if(category.startsWith('equal-')&&references){const left=unpackResult(references[2*i],null),right=unpackResult(references[2*i+1],null);if(ending(left,true)!==ending(right,true)||!left.stdout.equals(right.stdout))category='oracle-pair-disagrees';else if(ending(left,true)!==a)category='rust-ending-differs';else if(!left.stdout.equals(sugar.stdout))category='rust-output-differs';}
  tally[engine][category]=(tally[engine][category]||0)+1;
  if(!category.startsWith('equal-'))failures.push({index:i,family:pairs[i].family,engine,category,sugar:files[2*i],expanded:files[2*i+1],diagnostic:sugar.stderr.toString()+'\n'+expanded.stderr.toString()});
 }
}
const record={generator:'sugar',sha256:generatorHash,seed:seedText,pairs:count,rust:compareRust,tally};
console.log(JSON.stringify(record));fs.writeFileSync(path.join(dir,'failures.json'),JSON.stringify(failures,null,2)+'\n');fs.appendFileSync(path.join(output,'runs.jsonl'),JSON.stringify(record)+'\n');if(failures.length){console.error(JSON.stringify(failures.slice(0,4),null,2));process.exitCode=1;}
