#!/usr/bin/env node
// Checked closures against explicit environment structs/functions. This oracle
// remains usable without Rust; reference outputs are cached by source and seed.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { run } from '../process.mjs';
import { runRoutes } from '../batch.mjs';
import { oracleCache, packResult, unpackResult, worldOutcome } from '../oracle.mjs';
const [binary='target/debug/lkjscript',seed='1501',countText='88',output='target/generators']=process.argv.slice(2);
const count=Number(countText),bin=path.resolve(binary),dir=path.resolve(output,`closures-${seed}`);
const engines=(process.env.LKJ_ENGINES||'native,k').split(','),rust=process.env.LKJ_CLOSURE_RUST!=='0';
let state=BigInt(seed)||1n;
function random(n){state^=(state<<13n)&0xffffffffffffffffn;state^=state>>7n;state^=(state<<17n)&0xffffffffffffffffn;return Number(state%BigInt(n));}
const prelude=`struct M { out: Stdout, byte: u8 }
impl Drop for M { fn drop(&mut self) { self.out.write(&[self.byte]); } }
struct P { left: M, right: M }
fn tick(world:&mut World,byte:u8) { let mut out=world.stdout();out.write(&[byte]); }
fn make(world:&mut World,byte:u8)->M {tick(world,byte-32u8);M {out:world.stdout(),byte}}
fn see(value:&M)->u8 {value.byte}
`;
function pair(index){
 const a=97+random(10),b=107+random(10),x=1+random(10),y=1+random(10),calls=1+random(4),mark=48+random(10);
 let source='',expanded='',sourceHelpers='',expandedHelpers='',family;
 switch(index%22){
 case 0:{family='shared-two-roots';const prefix=`let first=${x}u8;let second=${y}u8;`;
  sourceHelpers='fn apply<F:Fn(u8)->u8>(f:F,value:u8)->u8{f(value)}';
  expandedHelpers="struct Capture<'a,'b>{first:&'a u8,second:&'b u8} fn invoke(value:&Capture<'_,'_>,argument:u8)->u8{argument+*value.first+*value.second}";
  source=prefix+'let f=|n|n+first+second;let answer=apply(f,1u8);tick(&mut world,answer);';
  expanded=prefix+'let f=Capture{first:&first,second:&second};let answer=invoke(&f,1u8);tick(&mut world,answer);';break;}
 case 1:{family='mutable-two-roots';const prefix=`let mut first=${x}u8;let mut second=${y}u8;`;
  expandedHelpers="struct Capture<'a,'b>{first:&'a mut u8,second:&'b mut u8} fn invoke(value:&mut Capture<'_,'_>)->u8{*value.first=*value.first+1u8;*value.second=*value.second+2u8;*value.first+*value.second}";
  source=prefix+'let mut f=||{first=first+1u8;second=second+2u8;first+second};';
  expanded=prefix+'let mut f=Capture{first:&mut first,second:&mut second};';
  for(let i=0;i<calls;i++){source+='let answer=f();tick(&mut world,answer);';expanded+='let answer=invoke(&mut f);tick(&mut world,answer);';}
  expanded+='drop(f.first);drop(f.second);';source+='tick(&mut world,first);tick(&mut world,second);';expanded+='tick(&mut world,first);tick(&mut world,second);';break;}
 case 2:{family='owned-fnonce';const prefix=`let owned=make(&mut world,${a}u8);`;
  sourceHelpers='fn invoke<F:FnOnce()->u8>(f:F)->u8{f()}';
  expandedHelpers='struct Capture{owned:M} fn invoke(c:Capture)->u8{let moved=c.owned;moved.byte}';
  source=prefix+'let answer=invoke(move||{let moved=owned;moved.byte});tick(&mut world,answer);';
  expanded=prefix+'let answer=invoke(Capture{owned});tick(&mut world,answer);';break;}
 case 3:{family='capture-first-use-order';const prefix=`let alpha=make(&mut world,${a}u8);let zeta=make(&mut world,${b}u8);`;
  expandedHelpers='struct Capture{zeta:M,alpha:M} fn invoke(c:&Capture)->u8{see(&c.zeta)-see(&c.alpha)}';
  source=prefix+'let f=move||see(&zeta)-see(&alpha);let answer=f();tick(&mut world,answer);drop(f);';
  expanded=prefix+'let f=Capture{zeta,alpha};let answer=invoke(&f);tick(&mut world,answer);drop(f);';break;}
 case 4:{family='disjoint-owned-field';const prefix=`let pair=P{left:make(&mut world,${a}u8),right:make(&mut world,${b}u8)};`;
  expandedHelpers='struct Capture{right:M} fn invoke(c:&Capture)->u8{see(&c.right)}';
  source=prefix+'let f=move||see(&pair.right);drop(pair.left);let answer=f();tick(&mut world,answer);drop(f);';
  expanded=prefix+'let f=Capture{right:pair.right};drop(pair.left);let answer=invoke(&f);tick(&mut world,answer);drop(f);';break;}
 case 5:{family='partial-capture-cleanup';const prefix=`let mut count=0u8;let owned=make(&mut world,${a}u8);let counter=&mut count;`;
  expandedHelpers="struct Capture<'a>{owned:M,counter:&'a mut u8} fn invoke(c:&mut Capture<'_>){see(&c.owned);*c.counter=*c.counter+1u8;}";
  source=prefix+'let mut f=move||{see(&owned);*counter=*counter+1u8;};';expanded=prefix+'let mut f=Capture{owned,counter};';
  for(let i=0;i<calls;i++){source+='f();';expanded+='invoke(&mut f);';}
  expanded+='drop(f.counter);';source+='tick(&mut world,count+48u8);';expanded+='tick(&mut world,count+48u8);';break;}
 case 6:{family='fnonce-forwards-fn-drop';const prefix=`let owned=make(&mut world,${a}u8);`;
  sourceHelpers=`fn invoke<F:FnOnce()->u8>(f:F,world:&mut World)->u8{let answer=f();tick(world,${mark}u8);answer}`;
  expandedHelpers=`struct Capture{owned:M} fn shared(c:&Capture)->u8{see(&c.owned)} fn once(c:Capture)->u8{let answer=shared(&c);drop(c);answer} fn invoke(c:Capture,world:&mut World)->u8{let answer=once(c);tick(world,${mark}u8);answer}`;
  source=prefix+'let answer=invoke(move||see(&owned),&mut world);tick(&mut world,answer);';
  expanded=prefix+'let answer=invoke(Capture{owned},&mut world);tick(&mut world,answer);';break;}
 case 7:{family='nested-shadowed-captures';const prefix=`let first=${x}u8;let second=${y}u8;`;
  expandedHelpers="struct Outer<'a,'b>{first:&'a u8,second:&'b u8} struct Inner<'a,'b>{first:&'a u8,second:&'b u8} fn inner(c:&Inner<'_,'_>)->u8{*c.first+*c.second} fn outer(c:&Outer<'_,'_>)->u8{let first=*c.first;let nested=Inner{first:&first,second:c.second};inner(&nested)}";
  source=prefix+'let f=||{let first=first;let g=||first+second;g()};let answer=f();tick(&mut world,answer);';
  expanded=prefix+'let f=Outer{first:&first,second:&second};let answer=outer(&f);tick(&mut world,answer);';break;}
 case 8:{family='contextual-parameter-pattern';
  sourceHelpers='fn invoke<F:Fn((u8,u8))->u8>(f:F,pair:(u8,u8))->u8{f(pair)}';
  expandedHelpers='struct Capture{} fn invoke(_c:&Capture,pair:(u8,u8))->u8{let left=pair.0;let right=pair.1;left+right}';
  source=`let answer=invoke(|(left,right)|left+right,(${x}u8,${y}u8));tick(&mut world,answer);`;
  expanded=`let answer=invoke(&Capture{},(${x}u8,${y}u8));tick(&mut world,answer);`;break;}
 case 9:{family='fnonce-forwards-fnmut-drop';const prefix=`let mut count=0u8;let owned=make(&mut world,${a}u8);let counter=&mut count;`;
  sourceHelpers=`fn invoke<F:FnOnce()>(f:F,world:&mut World){f();tick(world,${mark}u8);}`;
  expandedHelpers=`struct Capture<'a>{owned:M,counter:&'a mut u8} fn mutable(c:&mut Capture<'_>){see(&c.owned);*c.counter=*c.counter+1u8;} fn invoke(mut c:Capture<'_>,world:&mut World){mutable(&mut c);drop(c);tick(world,${mark}u8);}`;
  source=prefix+'invoke(move||{see(&owned);*counter=*counter+1u8;},&mut world);tick(&mut world,count+48u8);';
  expanded=prefix+'invoke(Capture{owned,counter},&mut world);tick(&mut world,count+48u8);';break;}
 case 10:{family='capture-coalescing-order';const prefix=`let pair=P{left:make(&mut world,${a}u8),right:make(&mut world,${b}u8)};let middle=make(&mut world,120u8);`;
  sourceHelpers=expandedHelpers='fn see_pair(value:&P)->u8{value.right.byte-value.left.byte}';
  expandedHelpers+='struct Capture{pair:P,middle:M} fn invoke(c:&Capture)->u8{see(&c.pair.left);see(&c.middle);see_pair(&c.pair)}';
  source=prefix+'let f=move||{see(&pair.left);see(&middle);see_pair(&pair)};let answer=f();tick(&mut world,answer);drop(f);';
  expanded=prefix+'let f=Capture{pair,middle};let answer=invoke(&f);tick(&mut world,answer);drop(f);';break;}
 case 11:{family='assignment-capture-order';const prefix=`let mut first=make(&mut world,${a}u8);let second=make(&mut world,${b}u8);`;
  expandedHelpers='struct Capture{first:M,second:M}';
  source=prefix+'let f=move||{first=second;};drop(f);';expanded=prefix+'let f=Capture{first,second};drop(f);';break;}
 case 12:{family='index-capture-order';const prefix=`let mut values:Vec<M>=Vec::new();values.push(make(&mut world,${a}u8));let tag=make(&mut world,${b}u8);`;
  expandedHelpers='struct Reader{tag:M} struct Capture{values:Vec<M>,reader:Reader}';
  source=prefix+'let reader=move||{see(&tag);0usize};let lookup=move||values[reader()].byte;drop(lookup);';
  expanded=prefix+'let reader=Reader{tag};let lookup=Capture{values,reader};drop(lookup);';break;}
 case 13:{family='interleaved-field-capture-order';const prefix=`let pair=P{left:make(&mut world,${a}u8),right:make(&mut world,${b}u8)};let middle=make(&mut world,120u8);`;
  expandedHelpers='struct Capture{left:M,right:M,middle:M}';
  source=prefix+'let f=move||{see(&pair.right);see(&middle);see(&pair.left)};drop(f);';
  expanded=prefix+'let f=Capture{left:pair.left,right:pair.right,middle};drop(f);';break;}
 case 14:{family='generic-result-origin';const captured=random(2)===0,prefix=`let first=${x}u8;let second=${y}u8;`;
  sourceHelpers='fn apply<A,B,F:FnOnce(A)->B>(f:F,value:A)->B{f(value)}';
  expandedHelpers="struct Capture<'a>{value:&'a u8} fn invoke<'a,'b>(c:Capture<'a>,argument:&'b u8)->&"+(captured?"'a":"'b")+" u8{"+(captured?'c.value':'let _=*c.value;argument')+"}";
  source=prefix+'let captured=&first;let answer=apply(move|argument:&u8|{'+(captured?'captured':'let _=*captured;argument')+'},&second);tick(&mut world,*answer);';
  expanded=prefix+'let answer=invoke(Capture{value:&first},&second);tick(&mut world,*answer);';break;}
 case 15:{family='generic-unrelated-short-capture';const prefix=`let first=${x}u8;let answer:&u8;`;
  sourceHelpers='fn apply<A,B,F:FnOnce(A)->B>(f:F,value:A)->B{f(value)}';
  expandedHelpers="struct Capture<'a>{value:&'a u8} fn invoke<'a,'b>(c:Capture<'b>,argument:&'a u8)->&'a u8{let _=*c.value;argument}";
  source=prefix+`{let short=${y}u8;let captured=&short;answer=apply(move|argument:&u8|{let _=*captured;argument},&first);}tick(&mut world,*answer);`;
  expanded=prefix+`{let short=${y}u8;answer=invoke(Capture{value:&short},&first);}tick(&mut world,*answer);`;break;}
 case 16:{family='generic-mutable-leaf-permutation';const prefix=`let mut first=${x}u8;let mut second=${y}u8;`;
  sourceHelpers='fn apply<A,B,F:FnOnce(A)->B>(f:F,value:A)->B{f(value)}';
  expandedHelpers="fn invoke<'a,'b>(pair:(&'a mut u8,&'b mut u8))->(&'b mut u8,&'a mut u8){(pair.1,pair.0)}";
  source=prefix+'let answer=apply(|pair:(&mut u8,&mut u8)|(pair.1,pair.0),(&mut first,&mut second));';
  expanded=prefix+'let answer=invoke((&mut first,&mut second));';
  const finish='*answer.0=*answer.0+1u8;*answer.1=*answer.1+2u8;tick(&mut world,*answer.0);tick(&mut world,*answer.1);drop(answer);tick(&mut world,first);tick(&mut world,second);';source+=finish;expanded+=finish;break;}
 case 17:{family='associated-item-held-origins';const prefix=`let values=vec![${x}u8,${y}u8];let mut iterator=values.iter();`;
  sourceHelpers='fn next<I:Iterator>(iterator:&mut I)->Option<I::Item>{iterator.next()}';
  source=prefix+'let first=next(&mut iterator).unwrap();let second=next(&mut iterator).unwrap();drop(iterator);tick(&mut world,*first);tick(&mut world,*second);';
  expanded=prefix+'let first=iterator.next().unwrap();let second=iterator.next().unwrap();drop(iterator);tick(&mut world,*first);tick(&mut world,*second);';break;}
 case 18:{family='same-type-factory-shared-origin';const some=random(2)===0,prefix=`let first=${x}u8;let second=${y}u8;`;
  sourceHelpers='fn choose<T,F:FnOnce()->T>(value:Option<T>,make:F)->T{match value{Some(value)=>value,None=>make()}}';
  expandedHelpers="struct Capture<'a>{value:&'a u8} fn invoke<'a>(c:Capture<'a>)->&'a u8{c.value} fn choose<'a>(value:Option<&'a u8>,make:Capture<'a>)->&'a u8{match value{Some(value)=>value,None=>invoke(make)}}";
  const option=some?'Some(&first)':'None';
  source=prefix+'let value:Option<&u8>='+option+';let captured=&second;let answer=choose(value,move||captured);tick(&mut world,*answer);';
  expanded=prefix+'let value:Option<&u8>='+option+';let answer=choose(value,Capture{value:&second});tick(&mut world,*answer);';break;}
 case 19:{family='same-type-factory-mutable-origin';const some=random(2)===0,prefix=`let mut first=${x}u8;let mut second=${y}u8;`;
  sourceHelpers='fn choose<T,F:FnOnce()->T>(value:Option<T>,make:F)->T{match value{Some(value)=>value,None=>make()}}';
  expandedHelpers="struct Capture<'a>{value:&'a mut u8} fn invoke<'a>(c:Capture<'a>)->&'a mut u8{let moved=c.value;moved} fn choose<'a>(value:Option<&'a mut u8>,make:Capture<'a>)->&'a mut u8{match value{Some(value)=>value,None=>invoke(make)}}";
  const option=some?'Some(&mut first)':'None';
  source=prefix+'let value:Option<&mut u8>='+option+';let captured=&mut second;let answer=choose(value,move||captured);';
  expanded=prefix+'let value:Option<&mut u8>='+option+';let answer=choose(value,Capture{value:&mut second});';
  const finish='*answer=*answer+1u8;tick(&mut world,*answer);drop(answer);tick(&mut world,first);tick(&mut world,second);';source+=finish;expanded+=finish;break;}
 case 20:{family='inferred-optional-mutable-captures';const choice=random(3),prefix=`let mut first=${x}u8;let mut second=${y}u8;let choice=${choice}u8;`;
  expandedHelpers="struct Capture<'a>{choice:u8,first:&'a mut u8,second:&'a mut u8} fn invoke<'a>(c:Capture<'a>)->Option<&'a mut u8>{match c.choice{0=>None,1=>Some(c.first),_=>Some(c.second)}}";
  source=prefix+'let a=&mut first;let b=&mut second;let f=move||match choice{0=>None,1=>Some(a),_=>Some(b)};let result=f();';
  expanded=prefix+'let result=invoke(Capture{choice,first:&mut first,second:&mut second});';
  const finish='match result{Some(answer)=>{*answer=*answer+1u8;tick(&mut world,*answer);},None=>tick(&mut world,0u8)}tick(&mut world,first);tick(&mut world,second);';source+=finish;expanded+=finish;break;}
 case 21:{family='inferred-break-mutable-captures';const choice=random(2)===0,prefix=`let mut first=${x}u8;let mut second=${y}u8;let choice=${choice};`;
  expandedHelpers="struct Capture<'a>{choice:bool,first:&'a mut u8,second:&'a mut u8} fn invoke<'a>(c:Capture<'a>)->&'a mut u8{if c.choice{c.first}else{c.second}}";
  source=prefix+'let a=&mut first;let b=&mut second;let f=move||loop{if choice{break a;}break b;};let answer=f();';
  expanded=prefix+'let answer=invoke(Capture{choice,first:&mut first,second:&mut second});';
  const finish='*answer=*answer+1u8;tick(&mut world,*answer);drop(answer);tick(&mut world,first);tick(&mut world,second);';source+=finish;expanded+=finish;break;}

 }
 const end=`tick(&mut world,${mark}u8);`;
 return {family,source:prelude+sourceHelpers+'\npub fn main(mut world:World){'+source+end+'}\n',expanded:prelude+expandedHelpers+'\npub fn main(mut world:World){'+expanded+end+'}\n'};
}
fs.mkdirSync(dir,{recursive:true});
const pairs=Array.from({length:count},(_,index)=>pair(index)),files=[];
for(let index=0;index<count;index++)for(const variant of ['source','expanded']){const file=path.join(dir,`${index}-${variant}.rs`);fs.writeFileSync(file,pairs[index][variant]);files.push(file);}
let references;
if(rust){
 let source='#![allow(warnings)]\nmod host{\n'+fs.readFileSync('bridge/output.rs','utf8')+'\n}\n';
 for(let index=0;index<count;index++)for(const variant of ['source','expanded'])source+=`mod c${index}_${variant}{use crate::host::{World,Stdout};\n${pairs[index][variant]} }\n`;
 source+='fn main(){match std::env::var("LKJ_ORACLE_CASE").unwrap().as_str(){\n';
 for(let index=0;index<count;index++)for(const variant of ['source','expanded'])source+=`"${index}-${variant}"=>host::enter(c${index}_${variant}::main),\n`;
 source+='_=>std::process::exit(2)}}\n';
 const rustSource=path.join(dir,'oracle.rs'),rustBinary=path.join(dir,'oracle');fs.writeFileSync(rustSource,source);
 const flags=['--edition=2024','--cfg','lkj_control','-C','overflow-checks=yes','-A','warnings'];
 const generator=createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).digest('hex');
 references=await oracleCache({generator,seed,count,flags,source},async()=>{
  const built=await run('rustc',[...flags,rustSource,'-o',rustBinary]);if(built.error||built.status!==0)throw Error('invalid generated Rust: '+built.stderr);
  const results=[];for(let index=0;index<count;index++)for(const variant of ['source','expanded'])results.push(packResult(await run(rustBinary,[],{encoding:null,captureControl:true,env:{...process.env,LKJ_ORACLE_CASE:`${index}-${variant}`}})));return results;
 });
}
function ending(result){
 if(result.error)return result.error.code==='ETIMEDOUT'?'timeout':'crash';
 if(result.checkStatus!==undefined&&result.checkStatus!==0)return'rejected:'+(result.stderr.toString().match(/\bE_[A-Z_]+\b/)?.[0]||'unknown');
 if(result.status===0&&!result.signal)return'normal';
 return result.stderr.toString().match(/\bM_[A-Z_]+\b/)?.[0]||'trap-or-crash';
}
const tallies={},failures=[];
const routes=await runRoutes(bin,files.map(path=>({path})),{engines,output:dir,encoding:null});
for(const engine of engines){
 const results=routes[engine];tallies[engine]={};
 for(let index=0;index<count;index++){
  const source=results[index*2],expanded=results[index*2+1],a=ending(source),b=ending(expanded);
  let category=a!=='normal'?'source-'+a:b!=='normal'?'expansion-'+b:source.stdout.equals(expanded.stdout)?'equal-output':'different-output';
  if(category==='equal-output'&&references)for(let offset=0;offset<2;offset++){const oracle=unpackResult(references[index*2+offset],null),outcome=worldOutcome(oracle);if(outcome.kind!=='normal'||outcome.status!==0||!source.stdout.equals(oracle.stdout))category='rust-difference';}
  tallies[engine][category]=(tallies[engine][category]||0)+1;
  if(category!=='equal-output')failures.push({engine,index,family:pairs[index].family,category,source:source.stderr.toString(),expanded:expanded.stderr.toString(),output:[...source.stdout],expected:[...expanded.stdout]});
 }
}
fs.writeFileSync(path.join(dir,'results.json'),JSON.stringify({seed,count,tallies,failures},null,2)+'\n');
console.log(JSON.stringify({generator:'closures',seed,count,oracles:rust?'environment+rustc':'environment',tallies,failures:failures.length}));
if(failures.length){console.error(JSON.stringify(failures.slice(0,5),null,2));process.exitCode=1;}
