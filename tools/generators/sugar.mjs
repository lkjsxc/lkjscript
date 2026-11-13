#!/usr/bin/env node
// The primary oracle is each generated sugar against an independently emitted
// expansion on every route. LKJ_SUGAR_RUST=0 keeps that oracle without rustc.
// Rust's 32-way nested or product exceeds the observed oracle compilation budget;
// those cases still compare every route with their independently expanded Rust forms.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { run } from '../process.mjs';
import { runRoutes } from '../batch.mjs';
import { oracleCache, packResult, unpackResult } from '../oracle.mjs';
import { combinatorPair, optionExtraPair, optionViewPair } from './combinators.mjs';
import { sequencePair, extendSlicePair } from './sequence.mjs';
import { iteratorClosurePair } from './iterator_closures.mjs';
import { iteratorReducerPair } from './iterator_reducers.mjs';
const [binary='target/debug/lkjscript',seedText='1301',countText='216',output='target/generators']=process.argv.slice(2);
const bin=path.resolve(binary),count=Number(countText),dir=path.resolve(output,`sugar-${seedText}`);
const engines=(process.env.LKJ_ENGINES||'native,k').split(',');
const compareRust=process.env.LKJ_SUGAR_RUST!=='0';
let state=BigInt(seedText)||1n;
function random(n){state^=(state<<13n)&0xffffffffffffffffn;state^=state>>7n;state^=(state<<17n)&0xffffffffffffffffn;return Number(state%BigInt(n));}
const prelude=`struct M { out: Stdout, b: u8 }
impl Drop for M { fn drop(&mut self) { self.out.write(&[self.b]); } }
struct P { a: M, b: M }
fn tick(world: &mut World, byte: u8) { let mut out = world.stdout(); out.write(&[byte]); }
fn make(world: &mut World, byte: u8) -> M { tick(world, byte+100); M { out: world.stdout(), b: byte } }
`;
const rangeExpansion="fn invalid_range(world:&mut World,bytes:&[u8]){let invalid=bytes.len();tick(world,bytes[invalid]);}\nfn print_range(world:&mut World,bytes:&[u8],start:usize,mut end:usize,inclusive:bool,text:bool){\n if inclusive{if end>=bytes.len(){invalid_range(world,bytes);}end=end+1;}\n if start>end||end>bytes.len(){invalid_range(world,bytes);}\n if text{if start<bytes.len()&&(bytes[start]&192u8)==128u8{invalid_range(world,bytes);}if end<bytes.len()&&(bytes[end]&192u8)==128u8{invalid_range(world,bytes);}}\n let mut index=start;while index<end{tick(world,bytes[index]);index=index+1;}\n}\n";
function pair(index){
 const a=1+random(50),b=1+random(50),c=1+random(50),marker=60+random(30),yes=random(2)===0?'true':'false';
 let sugar='',expanded='',sugarHelpers='',expandedHelpers='';let family,rustSugar=true;
 switch(index%72){
  case 0:{family='if-let';const value=`if ${yes} { Some(make(&mut world,${a}u8)) } else { None }`;
   sugar=`if let Some(m) = ${value} { tick(&mut world,${marker}u8); drop(m); } else { tick(&mut world,${b}u8); }`;
   expanded=`match ${value} { Some(m) => { tick(&mut world,${marker}u8); drop(m); }, None => tick(&mut world,${b}u8) }`;break;}
  case 1:{family='while-let';const prefix=`let mut v: Vec<M> = Vec::new(); v.push(make(&mut world,${a}u8)); v.push(make(&mut world,${b}u8));`;
   sugar=prefix+`while let Some(m) = v.pop() { tick(&mut world,${marker}u8); drop(m); }`;
   expanded=prefix+`loop { match v.pop() { Some(m) => { tick(&mut world,${marker}u8); drop(m); }, None => break } }`;break;}
  case 2:{family='let-else';const header='fn take(world: &mut World, yes: bool) -> Option<M> { ';const value=`if yes { Some(make(world,${a}u8)) } else { None }`;const tail=` tick(world,${marker}u8); Some(m) }`;
   sugarHelpers=header+`let Some(m) = (${value}) else { tick(world,${b}u8); return None; };`+tail;
   expandedHelpers=header+`let m = match ${value} { Some(m) => m, None => { tick(world,${b}u8); return None; } };`+tail;
   sugar=expanded=`let answer=take(&mut world,${yes}); tick(&mut world,${c}u8); drop(answer);`;
   const shared=`let mut cell=${a}u8;`,referenceTail=`*slot=*slot+1u8;drop(slot);tick(&mut world,cell);`;
   sugar+=shared+'let Some(slot):Option<&mut u8>=Some(&mut cell) else{return};'+referenceTail;
   expanded+=shared+'let input:Option<&mut u8>=Some(&mut cell);let slot=match input{Some(slot)=>slot,None=>return};'+referenceTail;break;}
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
   expanded=prefix+'let a=p.0; let b=p.1; tick(&mut world,a); tick(&mut world,b);';
   const rest=`let tuple=(${a}u8,${c}u8,${b}u8);`;
   sugar+=rest+'let (first,..,last)=&tuple;tick(&mut world,*first);tick(&mut world,*last);';
   expanded+=rest+'let first=&tuple.0;let last=&tuple.2;tick(&mut world,*first);tick(&mut world,*last);';break;}
  case 10:{family='named-binding';const prefix=`let p=P { a:make(&mut world,${a}u8), b:make(&mut world,${b}u8) };`;
   sugar=prefix+`let P { a, .. }=p; tick(&mut world,${marker}u8); drop(a);`;
   expanded=prefix+`let a=p.a; tick(&mut world,${marker}u8); drop(a);`;
   const rest=`let triple=(make(&mut world,${a}u8),make(&mut world,${b}u8),make(&mut world,${c}u8));`;
   sugar+=rest+`{let(first,..,last)=triple;tick(&mut world,${marker}u8);drop(last);drop(first);}`;
   expanded+=rest+`{let first=triple.0;let last=triple.2;tick(&mut world,${marker}u8);drop(last);drop(first);}`;break;}
  case 11:{family='guard-keeps-owner';const target=random(2)?a:b;const prefix=`let v=Some(make(&mut world,${a}u8));`;const success=`{ tick(&mut world,${c}u8); drop(m); }`;
   sugar=prefix+`match v { Some(m) if { tick(&mut world,${marker}u8); m.b==${target}u8 } => ${success}, Some(m) => drop(m), None => () }`;
   expanded=prefix+`let yes=match &v { Some(m) => { tick(&mut world,${marker}u8); m.b==${target}u8 }, None => false }; if yes { match v { Some(m) => ${success}, None => () } } else { match v { Some(m) => drop(m), None => () } }`;break;}
  case 12:{family='shared-or-product';const width=4+random(8),values=Array.from({length:width},()=>random(3));const tuple='('+values.map(n=>n+'i64').join(',')+')';
   sugar=`let p=${tuple}; match p { (${values.map(()=>'0|1').join(',')}) => tick(&mut world,${a}u8), _ => tick(&mut world,${b}u8) }`;
   expanded=`let p=${tuple}; if ${values.map((_,i)=>`(p.${i}==0 || p.${i}==1)`).join(' && ')} { tick(&mut world,${a}u8); } else { tick(&mut world,${b}u8); }`;break;}
  case 13:{family='bound-or-product';const width=3+random(3),variant=Array.from({length:width},()=>random(2)?'A':'B');
   sugarHelpers=expandedHelpers='enum Pick { A(M), B(M) }\n';
   const prefix=`let value=(${variant.map((tag,i)=>`Pick::${tag}(make(&mut world,${1+(a+i)%50}u8))`).join(',')});`;
   sugar=prefix+`match value { (${variant.map((_,i)=>`Pick::A(x${i})|Pick::B(x${i})`).join(',')}) => { tick(&mut world,${marker}u8); } }`;
   expanded=prefix+'{'+variant.map((_,i)=>`let x${i}=match value.${i} { Pick::A(x)=>x, Pick::B(x)=>x };`).join('')+`tick(&mut world,${marker}u8);}`;break;}
  case 14:{family='guard-product-selection';const width=3+random(4),target=random(1<<width),bits=Array.from({length:width},(_,i)=>1<<(width-1-i));
   const prefix=`let value=(${bits.map(bit=>`(0i64,${bit}i64)`).join(',')});`,sum=bits.map((_,i)=>`x${i}`).join('+');
   sugar=prefix+`match value { (${bits.map((_,i)=>`(x${i},_)|(_,x${i})`).join(',')}) if {tick(&mut world,${marker}u8); ${sum}==${target}i64} => tick(&mut world,${a}u8), _=>tick(&mut world,${b}u8) }`;
   expanded=prefix+'let mut candidate=0i64;loop{'+bits.map((bit,i)=>`let x${i}=if (candidate & ${bit}i64)!=0 {value.${i}.1}else{value.${i}.0};`).join('')+`tick(&mut world,${marker}u8);if ${sum}==${target}i64{tick(&mut world,${a}u8);break;}candidate=candidate+1;if candidate==${1<<width}i64{tick(&mut world,${b}u8);break;}}`;break;}
  case 15:{family='guard-mutable-view';sugarHelpers=expandedHelpers='enum Pick { A(Box<i64>), B(Box<i64>) }\n';const tag=random(2)?'A':'B',number=random(3)-1;
   const prefix=`let mut value=Pick::${tag}(Box::new(${number}i64));`;
   const guard=`{tick(&mut world,${marker}u8);**x>0}`,body='{**x=**x+1;}',tail=`match value {Pick::A(x)=>tick(&mut world,(*x+10) as u8),Pick::B(x)=>tick(&mut world,(*x+10) as u8)}`;
   sugar=prefix+`match value {Pick::A(ref mut x)|Pick::B(ref mut x) if ${guard}=>${body},_=>()} `+tail;
   expanded=prefix+`match value {Pick::A(ref mut x)=>{if ${guard}${body}},Pick::B(ref mut x)=>{if ${guard}${body}}} `+tail;break;}
  case 16:{family='guard-outer-controls';const guard=`{tick(&mut world,${marker}u8);if round==1{continue;}if round==3{break ${a}i64;}false}`;
   sugar=`let mut round=0i64;let answer=loop{round=round+1;match (0i64,0i64){(0,_)|(_,0) if ${guard}=>(),_=>()}};tick(&mut world,answer as u8);`;
   expanded=`let mut round=0i64;let answer=loop{round=round+1;if ${guard} {} else if ${guard} {}};tick(&mut world,answer as u8);`;break;}
  case 17:{family='guard-cached-discriminant';const prefix=`let value=if ${yes}{Some(make(&mut world,${a}u8))}else{None};`;
   sugar=prefix+`match value {Some(_) if {drop(value);false}=>(),Some(_)=>tick(&mut world,${marker}u8),None=>tick(&mut world,${b}u8)}`;
   expanded=prefix+`let some=match &value{Some(_)=>true,None=>false};if some{drop(value);tick(&mut world,${marker}u8);}else{tick(&mut world,${b}u8);}`;break;}
  case 18:{family='shared-guard-or-32';rustSugar=false;const width=32,values=Array(width).fill(0);if(random(2))values[random(width)]=1;
   const prefix=`let value=((${values.map(n=>n+'i64').join(',')}),make(&mut world,${a}u8));`;
   sugar=prefix+`match value { ((${values.map(()=>'0|0').join(',')}),m) if {tick(&mut world,${marker}u8);true} => drop(m),_=>tick(&mut world,${b}u8) }`;
   expanded=prefix+`if ${values.map((_,i)=>`value.0.${i}==0`).join(' && ')}{let m=value.1;tick(&mut world,${marker}u8);drop(m);}else{tick(&mut world,${b}u8);}`;break;}
  case 19:{family='infeasible-or-subtree-32';rustSugar=false;const width=32,values=Array(width).fill(0);values[width-1]=1;
   const prefix=`let value=((${values.map(n=>n+'i64').join(',')}),make(&mut world,${a}u8));`;
   sugar=prefix+`match value { ((${values.map(()=>'0|0').join(',')}),m)|(_,m) if {tick(&mut world,${marker}u8);true}=>drop(m),_=>tick(&mut world,${b}u8) }`;
   expanded=prefix+`let m=value.1;tick(&mut world,${marker}u8);drop(m);`;break;}
  case 20:{family='repeated-vector';const length=random(5);sugarHelpers=expandedHelpers=`struct CloneMark{v:u8}
impl Clone for CloneMark{fn clone(&self)->Self{CloneMark{v:self.v+1u8}}}
fn element(world:&mut World)->CloneMark{tick(world,${a+100}u8);CloneMark{v:${a}u8}}
fn count(world:&mut World)->usize{tick(world,${marker}u8);${length}usize}
`;
   sugar='let values=vec![element(&mut world);count(&mut world)];';
   expanded='let value=element(&mut world);let size=count(&mut world);let mut values:Vec<CloneMark>=Vec::new();if size>0{let mut index=1usize;while index<size{values.push(value.clone());index=index+1;}values.push(value);}else{drop(value);}';
   const visit='let mut index=0usize;while index<values.len(){tick(&mut world,values[index].v);index=index+1;}';sugar+=visit;expanded+=visit;break;}
  case 21:{family='unicode-character-encoding';const scalars=[0,0x7f,0x80,0x7ff,0x800,0xd7ff,0xe000,0xffff,0x10000,0x10ffff],chosen=Array.from({length:3},()=>scalars[random(scalars.length)]),bytes=[...Buffer.from(String.fromCodePoint(...chosen))];
   sugar='let mut text=String::new();'+chosen.map(code=>`text.push('\\u{${code.toString(16)}}');`).join('')+'let mut out=world.stdout();out.write(text.as_bytes());';
   expanded=`let bytes:Vec<u8>=vec![${bytes.map(byte=>byte+'u8').join(',')}];let mut out=world.stdout();out.write(&bytes);`;break;}

  case 22:{family='format-appends';const left=-(1+random(50)),right=1+random(50);sugarHelpers=expandedHelpers='fn argument(world:&mut World,mark:u8,value:i64)->i64{tick(world,mark);value}\n';
   const first=`argument(&mut world,${a}u8,${left}i64)`,second=`argument(&mut world,${b}u8,${right}i64)`;
   sugar=`let label="水";let text=format!("{{{label:😀^5}}}:{1:04}:{0}:{1:x}",${first},${second});let mut out=world.stdout();out.write(text.as_bytes());`;
   const expected='{😀😀水😀😀}:'+String(right).padStart(4,'0')+':'+left+':'+right.toString(16);
   expanded=`let first=${first};let second=${second};let mut text=String::new();text.push_str(${JSON.stringify(expected)});let mut out=world.stdout();out.write(text.as_bytes());`;break;}
  case 23:{family='display-temporary-cleanup';sugarHelpers=expandedHelpers=`struct Shown{owner:M,value:i64}
impl std::fmt::Display for Shown{fn fmt(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{write!(f,"[{}]",self.value)}}
fn shown(world:&mut World)->Shown{Shown{owner:make(world,${a}u8),value:${b}i64}}
`;
   sugar=`let text=format!("{0}-{0}",shown(&mut world));tick(&mut world,${marker}u8);let mut out=world.stdout();out.write(text.as_bytes());`;
   expanded=`let value=shown(&mut world);let mut text=String::new();text.push_str("[${b}]-[${b}]");drop(value);tick(&mut world,${marker}u8);let mut out=world.stdout();out.write(text.as_bytes());`;break;}
  case 24:{family='format-error-stops-appends';sugarHelpers=expandedHelpers='use std::fmt::Write;\nstruct Bad;\nimpl std::fmt::Display for Bad{fn fmt(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{f.write_str("!").unwrap();Err(std::fmt::Error)}}\n';
   sugar='let mut text=String::new();let result=write!(&mut text,"before{}after",Bad);assert!(result.is_err());let mut out=world.stdout();out.write(text.as_bytes());';
   expanded='let mut text=String::new();text.push_str("before");text.push_str("!");let mut out=world.stdout();out.write(text.as_bytes());';break;}

  case 25:{family='option-owned-combinators';
   const other=random(2)===0?'true':'false';
   sugarHelpers=expandedHelpers=`fn left(world:&mut World)->Option<M>{tick(world,${marker}u8);if ${yes}{Some(make(world,${a}u8))}else{None}}
fn right(world:&mut World)->Option<M>{tick(world,${marker+1}u8);if ${other}{Some(make(world,${b}u8))}else{None}}
`;
   expandedHelpers+=`fn expanded_and(left:Option<M>,right:Option<M>)->Option<M>{match left{Some(_)=>right,None=>None}}
fn expanded_or(left:Option<M>,right:Option<M>)->Option<M>{match left{Some(value)=>Some(value),None=>right}}
fn expanded_xor(left:Option<M>,right:Option<M>)->Option<M>{match(left,right){(Some(value),None)=>Some(value),(None,Some(value))=>Some(value),_=>None}}
fn expanded_zip(left:Option<M>,right:Option<M>)->Option<(M,M)>{match(left,right){(Some(left),Some(right))=>Some((left,right)),_=>None}}
fn expanded_flatten(value:Option<Option<M>>)->Option<M>{match value{Some(value)=>value,None=>None}}
fn expanded_unzip(value:Option<(M,M)>)->(Option<M>,Option<M>){match value{Some((left,right))=>(Some(left),Some(right)),None=>(None,None)}}
`;
   for(const method of ['and','or','xor','zip']){
    sugar+=`{let value=left(&mut world).${method}(right(&mut world));tick(&mut world,${marker}u8);drop(value);}`;
    expanded+=`{let value=expanded_${method}(left(&mut world),right(&mut world));tick(&mut world,${marker}u8);drop(value);}`;
   }
   const nested=`if ${other}{Some(left(&mut world))}else{None}`;
   sugar+=`{let value=(${nested}).flatten();tick(&mut world,${marker}u8);drop(value);}`;
   expanded+=`{let value=expanded_flatten(${nested});tick(&mut world,${marker}u8);drop(value);}`;
   const pair=`if ${yes}{Some((make(&mut world,${a}u8),make(&mut world,${b}u8)))}else{None}`;
   sugar+=`{let value=(${pair}).unzip();tick(&mut world,${marker}u8);drop(value);}`;
   expanded+=`{let value=expanded_unzip(${pair});tick(&mut world,${marker}u8);drop(value);}`;break;}
  case 26:{family='result-owned-combinators';const other=random(2)===0?'true':'false';
   sugarHelpers=expandedHelpers=`fn result(world:&mut World,yes:bool,tag:u8)->Result<M,M>{let value=make(world,tag);if yes{Ok(value)}else{Err(value)}}
`;
   expandedHelpers+=`fn expanded_and(left:Result<M,M>,right:Result<M,M>)->Result<M,M>{match left{Ok(_)=>right,Err(error)=>Err(error)}}
fn expanded_or(left:Result<M,M>,right:Result<M,M>)->Result<M,M>{match left{Ok(value)=>Ok(value),Err(_)=>right}}
fn expanded_option_transpose(value:Option<Result<M,M>>)->Result<Option<M>,M>{match value{Some(Ok(value))=>Ok(Some(value)),Some(Err(error))=>Err(error),None=>Ok(None)}}
fn expanded_result_transpose(value:Result<Option<M>,M>)->Option<Result<M,M>>{match value{Ok(Some(value))=>Some(Ok(value)),Ok(None)=>None,Err(error)=>Some(Err(error))}}
`;
   for(const method of ['and','or']){
    const left=`result(&mut world,${yes},${a}u8)`,right=`result(&mut world,${other},${b}u8)`;
    sugar+=`{let value=${left}.${method}(${right});tick(&mut world,${marker}u8);drop(value);}`;
    expanded+=`{let value=expanded_${method}(${left},${right});tick(&mut world,${marker}u8);drop(value);}`;
   }
   const option=`if ${yes}{Some(result(&mut world,${other},${c}u8))}else{None}`;
   sugar+=`{let value=(${option}).transpose();tick(&mut world,${marker}u8);drop(value);}`;
   expanded+=`{let value=expanded_option_transpose(${option});tick(&mut world,${marker}u8);drop(value);}`;
   const value=`if ${yes}{Ok(if ${other}{Some(make(&mut world,${a}u8))}else{None})}else{Err(make(&mut world,${b}u8))}`;
   sugar+=`{let value=(${value}).transpose();tick(&mut world,${marker}u8);drop(value);}`;
   expanded+=`{let value=expanded_result_transpose(${value});tick(&mut world,${marker}u8);drop(value);}`;break;}
  case 27:{family='option-result-view-combinators';
   expandedHelpers=`fn view(value:&Option<M>)->Option<&M>{match value{Some(value)=>Some(value),None=>None}}
fn view_mut(value:&mut Option<M>)->Option<&mut M>{match value{Some(value)=>Some(value),None=>None}}
fn insert(value:&mut Option<M>,next:M)->&mut M{*value=Some(next);match value{Some(value)=>value,None=>unreachable!()}}
fn get_or_insert(value:&mut Option<M>,next:M)->&mut M{if value.is_none(){*value=Some(next);}match value{Some(value)=>value,None=>unreachable!()}}
fn result_view(value:&Result<M,M>)->Result<&M,&M>{match value{Ok(value)=>Ok(value),Err(error)=>Err(error)}}
fn result_view_mut(value:&mut Result<M,M>)->Result<&mut M,&mut M>{match value{Ok(value)=>Ok(value),Err(error)=>Err(error)}}
`;
   const prefix=`let mut value=if ${yes}{Some(make(&mut world,${a}u8))}else{None};`;
   const read=value=>`match ${value}{Some(value)=>tick(&mut world,value.b),None=>tick(&mut world,${marker}u8)}`;
   const change=value=>`match ${value}{Some(value)=>{value.b=${b}u8;},None=>()}`;
   sugar=prefix+read('value.as_ref()')+change('value.as_mut()');
   expanded=prefix+read('view(&value)')+change('view_mut(&mut value)');
   sugar+=`{let value=value.get_or_insert(make(&mut world,${c}u8));tick(&mut world,value.b);}`;
   expanded+=`{let value=get_or_insert(&mut value,make(&mut world,${c}u8));tick(&mut world,value.b);}`;
   sugar+=`{let value=value.insert(make(&mut world,${a}u8));value.b=${b}u8;tick(&mut world,value.b);}`;
   expanded+=`{let value=insert(&mut value,make(&mut world,${a}u8));value.b=${b}u8;tick(&mut world,value.b);}`;
   sugar+=`let old=value.replace(make(&mut world,${c}u8));drop(old);`;
   expanded+=`let old=std::mem::replace(&mut value,Some(make(&mut world,${c}u8)));drop(old);`;
   const result=`let mut result:Result<M,M>=if ${yes}{Ok(make(&mut world,${a}u8))}else{Err(make(&mut world,${b}u8))};`;
   sugar+=result+`match result.as_ref(){Ok(value)|Err(value)=>tick(&mut world,value.b)}match result.as_mut(){Ok(value)|Err(value)=>{value.b=${c}u8;}}`;
   expanded+=result+`match result_view(&result){Ok(value)|Err(value)=>tick(&mut world,value.b)}match result_view_mut(&mut result){Ok(value)|Err(value)=>{value.b=${c}u8;}}`;break;}

  case 28:{family='collection-for-cleanup';
   const prefix=`let values=vec![make(&mut world,${a}u8),make(&mut world,${b}u8)];`;
   const body=`let local=make(&mut world,${c}u8);tick(&mut world,item.b);if ${yes}{continue;}tick(&mut world,${marker}u8);drop(local);`;
   expandedHelpers='fn expanded_into<I:IntoIterator>(value:I)->I::IntoIter{value.into_iter()}\nfn expanded_next<I:Iterator>(value:&mut I)->Option<I::Item>{value.next()}\n';
   sugar=prefix+`for item in &values{${body}}`;
   expanded=prefix+`match expanded_into(&values){mut iterator=>{while true{match expanded_next(&mut iterator){Some(mut next)=>{let item=next;let _:()={${body}};},None=>break}}}}`;break;}
  case 29:{family='temporary-iterable-controls';
   const source=`fn values(world:&mut World)->Vec<M>{tick(world,${marker}u8);vec![make(world,${a}u8),make(world,${b}u8)]}\n`;
   const body=`let local=make(world,${c}u8);tick(world,item.b);if mode==0{continue;}if mode==1{break;}if mode==2{return;}drop(local);`;
   const finish=`tick(world,${marker+1}u8);}\n`;
   sugarHelpers=source+'fn visit(world:&mut World,mode:u8){for item in &values(world){'+body+'}'+finish;
   expandedHelpers=source+'fn expanded_into<I:IntoIterator>(value:I)->I::IntoIter{value.into_iter()}\nfn expanded_next<I:Iterator>(value:&mut I)->Option<I::Item>{value.next()}\n'+
    'fn visit(world:&mut World,mode:u8){match expanded_into(&values(world)){mut iterator=>{while true{match expanded_next(&mut iterator){Some(mut next)=>{let item=next;let _:()={'+body+'};},None=>break}}}}'+finish;
   sugar=expanded='visit(&mut world,0);visit(&mut world,1);visit(&mut world,2);visit(&mut world,3);';break;}
  case 30:{family='iterator-value-pattern';
   const prefix=`let values=vec![(${a}u8,${b}u8),(${b}u8,${c}u8)];`,body='tick(&mut world,*left);tick(&mut world,*right);';
   expandedHelpers='fn expanded_into<I:IntoIterator>(value:I)->I::IntoIter{value.into_iter()}\nfn expanded_next<I:Iterator>(value:&mut I)->Option<I::Item>{value.next()}\n';
   sugar=prefix+`for (left,right) in values.iter(){${body}}`;
   expanded=prefix+`match expanded_into(values.iter()){mut iterator=>{while true{match expanded_next(&mut iterator){Some(mut next)=>{let (left,right)=next;let _:()={${body}};},None=>break}}}}`;break;}
  case 31:{family='range-binding-pattern';const start=random(5),end=start+1+random(5);
   sugar=`for ref mut value in ${start}i64..${end}i64{*value=*value+10;tick(&mut world,*value as u8);}for _ in 0i64..3i64{tick(&mut world,${marker}u8);}`;
   expanded=`let mut counter=${start}i64;while counter<${end}i64{let mut item=counter;let ref mut value=item;*value=*value+10;tick(&mut world,*value as u8);counter=counter+1;}let mut index=0i64;while index<3i64{tick(&mut world,${marker}u8);index=index+1;}`;break;}

  case 32:{family='shared-range-forms';const shape=random(3),kind=['u8','u16','i64','char','M'][random(5)],form=random(6),bounds=[[1,3],[0,3],[1,4],[0,4],[1,3],[0,3]][form],range=['1..3','..3','1..','..','1..=2','..=2'][form];
   const values=[a,b,c,marker].map(n=>kind==='M'?'make(&mut world,'+n+'u8)':kind==='char'?"'\\u{"+n.toString(16)+"}'":n+kind).join(',');
   const prefix=shape===0?'let values=vec!['+values+'];':shape===1?'let values=['+values+'];':'let storage=['+values+'];let values:&['+kind+']=&storage;';
   const value=name=>kind==='M'?name+'.b':name+' as u8';
   sugar=prefix+'let selected=&values['+range+'];let mut index=0usize;while index<selected.len(){tick(&mut world,'+value('selected[index]')+');index=index+1;}';
   expanded=prefix+'let whole:&['+kind+']='+(shape===2?'values':'&values')+';let mut index='+bounds[0]+'usize;while index<'+bounds[1]+'usize{tick(&mut world,'+value('whole[index]')+');index=index+1;}';break;}
  case 33:{family='mutable-nested-ranges';const shape=random(3),kind=['u8','u16','i64'][random(3)],values='0'+kind+',1'+kind+',2'+kind+',3'+kind;
   const prefix=shape===0?'let mut values=vec!['+values+'];':shape===1?'let mut values=['+values+'];':'let mut storage=['+values+'];let values:&mut['+kind+']=&mut storage;';
   sugar=prefix+'{let selected=&mut values[1..];let nested=&mut selected[..2];nested[0]='+a+kind+';nested[1]='+b+kind+';}';
   expanded=prefix+'{let whole:&mut['+kind+']=&mut '+(shape===2?'*values':'values')+';whole[1]='+a+kind+';whole[2]='+b+kind+';}';
   const visit='let mut index=0usize;while index<values.len(){tick(&mut world,values[index] as u8);index=index+1;}';sugar+=visit;expanded+=visit;break;}
  case 34:{family='utf8-text-range';const bounds=[[1,7],[1,3],[3,7],[3,3],[2,3],[1,2],[4,7],[8,8],[0,9],[7,3]][random(10)];
   const prefix='let text='+(random(2)?'"aλ🦀z".to_string()':'"aλ🦀z"')+';let start='+bounds[0]+'usize;let end='+bounds[1]+'usize;';
   sugar=prefix+'let selected=&text[start..end];let mut out=world.stdout();out.write(selected.as_bytes());';
   expandedHelpers=rangeExpansion;
   expanded=prefix+'print_range(&mut world,text.as_bytes(),start,end,false,true);';break;}
  case 35:{family='returned-captured-range';const prefix='let values=vec!['+a+'u8,'+b+'u8,'+c+'u8,'+marker+'u8];let captured=||selected(&values);let view=captured();drop(captured);';
   sugarHelpers="fn selected<'a>(bytes:&'a[u8])->&'a[u8]{&bytes[1..3]}\n";
   expandedHelpers="struct Chosen<'a>{data:&'a[u8],start:usize,end:usize}\nfn selected<'a>(bytes:&'a[u8])->Chosen<'a>{Chosen{data:bytes,start:1,end:3}}\n";
   sugar=prefix+'for item in view{tick(&mut world,*item);}';
   expanded=prefix+'let mut index=view.start;while index<view.end{tick(&mut world,view.data[index]);index=index+1;}';break;}
  case 36:{family='range-operand-order';const shape=random(3),mutable=random(2)===0,referent=shape===0?'[u8;4]':shape===1?'Vec<u8>':'[u8]',mode=mutable?'mut ':'';
   const source="fn source<'a>(world:&mut World,values:&'a "+mode+referent+")->&'a "+mode+referent+'{tick(world,'+marker+'u8);values}\nfn start(world:&mut World)->usize{tick(world,'+(marker+1)+'u8);1}\nfn end(world:&mut World)->usize{tick(world,'+(marker+2)+'u8);3}\n';
   sugarHelpers=expandedHelpers=source;
   const prefix='let '+(mutable?'mut ':'')+'values='+(shape===1?'vec!':'')+'['+a+'u8,'+b+'u8,'+c+'u8,'+marker+'u8];',base='source(&mut world,&'+mode+'values)';
   sugar=prefix+'{let selected=&'+mode+base+'[start(&mut world)..end(&mut world)];'+(mutable?'selected[0]='+a+'u8;':'')+'let mut index=0usize;while index<selected.len(){tick(&mut world,selected[index]);index=index+1;}}';
   expanded=prefix+'{let whole='+base+';let left=start(&mut world);let right=end(&mut world);'+(mutable?'whole[left]='+a+'u8;':'')+'let mut index=left;while index<right{tick(&mut world,whole[index]);index=index+1;}}';break;}
  case 37:{family='range-bounds-trap';const choices=[['0','4',false],['4','4',false],['2','1',false],['0','18446744073709551615',true],['3','2',true],['0','2',true],['3','3',false],['0','0',false],['0','3',false],['4','3',true]],bound=choices[random(choices.length)];
   const prefix='let bytes=vec!['+a+'u8,'+b+'u8,'+c+'u8];let start='+bound[0]+'usize;let end='+bound[1]+'usize;';
   sugar=prefix+'let selected=&bytes[start..'+(bound[2]?'=':'')+'end];let mut out=world.stdout();out.write(selected);';
   expandedHelpers=rangeExpansion;
   expanded=prefix+'print_range(&mut world,&bytes,start,end,'+bound[2]+',false);';break;}

  case 38:{family='iterator-enumerate-skip-take';const skip=random(5),limit=random(5);
   sugarHelpers=expandedHelpers=`struct Trace{values:Vec<M>,out:Stdout,tag:u8}\nimpl Drop for Trace{fn drop(&mut self){self.out.write(&[self.tag+1]);}}\nimpl Iterator for Trace{type Item=M;fn next(&mut self)->Option<M>{self.out.write(&[self.tag]);self.values.pop()}}\nfn traced(world:&mut World,tag:u8)->Trace{Trace{values:vec![make(world,${a}u8),make(world,${b}u8),make(world,${c}u8)],out:world.stdout(),tag}}\n`;
   const body=`tick(&mut world,index as u8);tick(&mut world,value.b);if ${yes}{break;}drop(value);`;
   sugar=`let mut iterator=traced(&mut world,70).skip(${skip}usize).take(${limit}usize).enumerate();while let Some((index,value))=iterator.next(){${body}}tick(&mut world,${marker}u8);drop(iterator);`;
   expanded=`let mut iterator=traced(&mut world,70);let mut skipping=${skip}usize;let mut remaining=${limit}usize;let mut index=0usize;loop{if remaining==0{break;}remaining=remaining-1;let mut ended=false;while skipping>0{skipping=skipping-1;match iterator.next(){Some(value)=>drop(value),None=>{skipping=0;ended=true;break;}}}let next:Option<M>=if ended{None}else{iterator.next()};match next{Some(value)=>{${body}index=index+1;},None=>break}}tick(&mut world,${marker}u8);drop(iterator);`;break;}
  case 39:{family='iterator-zip-next-order';const left=random(4),right=random(4);
   sugarHelpers=expandedHelpers=`struct Trace{values:Vec<M>,out:Stdout,tag:u8}\nimpl Drop for Trace{fn drop(&mut self){self.out.write(&[self.tag+1]);}}\nimpl Iterator for Trace{type Item=M;fn next(&mut self)->Option<M>{self.out.write(&[self.tag]);self.values.pop()}}\nfn traced(world:&mut World,tag:u8,count:usize)->Trace{let mut values:Vec<M>=Vec::new();let mut index=0usize;while index<count{values.push(make(world,${a}u8+index as u8));index=index+1;}Trace{values,out:world.stdout(),tag}}\n`;
   const body=`tick(&mut world,x.b);tick(&mut world,y.b);drop(y);drop(x);if ${yes}{break;}`;
   // Take imposes no effective limit but supplies the source IntoIterator contract.
   sugar=`let mut iterator=traced(&mut world,70,${left}usize).zip(traced(&mut world,80,${right}usize).take(9));while let Some((x,y))=iterator.next(){${body}}let last=iterator.next();drop(last);tick(&mut world,${marker}u8);drop(iterator);`;
   expanded=`let mut left=traced(&mut world,70,${left}usize);let mut right=traced(&mut world,80,${right}usize);loop{match left.next(){Some(x)=>match right.next(){Some(y)=>{${body}},None=>{drop(x);break;}},None=>break}}let last=match left.next(){Some(x)=>match right.next(){Some(y)=>Some((x,y)),None=>{drop(x);None}},None=>None};drop(last);tick(&mut world,${marker}u8);drop(left);drop(right);`;break;}
  case 40:{family='iterator-chain-exhaustion';const left=random(4),right=random(4);
   sugarHelpers=expandedHelpers=`struct Trace{values:Vec<M>,out:Stdout,tag:u8}\nimpl Drop for Trace{fn drop(&mut self){self.out.write(&[self.tag+1]);}}\nimpl Iterator for Trace{type Item=M;fn next(&mut self)->Option<M>{self.out.write(&[self.tag]);self.values.pop()}}\nfn traced(world:&mut World,tag:u8,count:usize)->Trace{let mut values:Vec<M>=Vec::new();let mut index=0usize;while index<count{values.push(make(world,${a}u8+index as u8));index=index+1;}Trace{values,out:world.stdout(),tag}}\n`;
   const body=`tick(&mut world,value.b);drop(value);if ${yes}{break;}`;
   sugar=`let mut iterator=traced(&mut world,70,${left}usize).chain(traced(&mut world,80,${right}usize));while let Some(value)=iterator.next(){${body}}tick(&mut world,${marker}u8);drop(iterator);`;
   expanded=`let mut left=Some(traced(&mut world,70,${left}usize));let mut right=traced(&mut world,80,${right}usize);loop{let mut next:Option<M>=None;match left.as_mut(){Some(iterator)=>{next=iterator.next();},None=>()}if next.is_none(){left=None;next=right.next();}match next{Some(value)=>{${body}},None=>break}}tick(&mut world,${marker}u8);drop(left);drop(right);`;break;}
  case 41:{family='ordered-collection-iteration';
   const prefix=`let mut map:std::collections::BTreeMap<u8,M>=std::collections::BTreeMap::new();map.insert(2u8,make(&mut world,${b}u8));map.insert(1u8,make(&mut world,${a}u8));let mut set:std::collections::BTreeSet<u8>=std::collections::BTreeSet::new();set.insert(2u8);set.insert(1u8);set.insert(2u8);`;
   sugar=prefix+'for (key,value) in &map{tick(&mut world,*key);tick(&mut world,value.b);}for key in map.keys(){tick(&mut world,*key);}for value in map.values(){tick(&mut world,value.b);}for value in &set{tick(&mut world,*value);}';
   expanded=prefix+'let mut key=1u8;while key<=2{tick(&mut world,key);tick(&mut world,map.get(&key).unwrap().b);key=key+1;}tick(&mut world,1);tick(&mut world,2);tick(&mut world,map.get(&1u8).unwrap().b);tick(&mut world,map.get(&2u8).unwrap().b);tick(&mut world,1);tick(&mut world,2);';break;}

  case 42:case 43:case 44:case 45:case 46:case 47:case 48:{
   ({family,sugar,expanded,sugarHelpers,expandedHelpers}=combinatorPair(index%72-42,random));break;}

  case 49:case 50:case 51:case 52:{
   ({family,sugar,expanded,sugarHelpers,expandedHelpers}=optionExtraPair(index%72-49,random));break;}

  case 53:case 54:case 55:case 56:{
   ({family,sugar,expanded,sugarHelpers,expandedHelpers}=sequencePair(index%72-53,random));break;}
  case 57:case 58:case 59:case 60:case 61:case 62:{
   ({family,sugar,expanded,sugarHelpers,expandedHelpers}=iteratorClosurePair(index%72-57,random));break;}
  case 63:{({family,sugar,expanded,sugarHelpers,expandedHelpers}=extendSlicePair(random));break;}
  case 64:case 65:case 66:case 67:case 68:case 69:{
   ({family,sugar,expanded,sugarHelpers,expandedHelpers}=iteratorReducerPair(index%72-64,random));break;}
  case 70:case 71:{
   ({family,sugar,expanded,sugarHelpers,expandedHelpers}=optionViewPair(index%72-70,random));break;}

 }
 const tail=`\ntick(&mut world,${90+random(10)}u8);`+(random(9)===0?' assert!(false);':'');
 return {family,rustSugar,sugar:prelude+sugarHelpers+'\npub fn main(mut world:World) {\n'+sugar+tail+'\n}\n',expanded:prelude+expandedHelpers+'\npub fn main(mut world:World) {\n'+expanded+tail+'\n}\n'};
}
fs.mkdirSync(dir,{recursive:true});
const pairs=Array.from({length:count},(_,i)=>pair(i)),files=[];
for(let i=0;i<count;i++)for(const variant of ['sugar','expanded']){const file=path.join(dir,`${i}-${variant}.rs`);fs.writeFileSync(file,pairs[i][variant]);files.push(file);}
const generatorHash=createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).update(fs.readFileSync(new URL('./combinators.mjs',import.meta.url))).update(fs.readFileSync(new URL('./sequence.mjs',import.meta.url))).update(fs.readFileSync(new URL('./iterator_closures.mjs',import.meta.url))).update(fs.readFileSync(new URL('./iterator_reducers.mjs',import.meta.url))).digest('hex');
let references;
if(compareRust){
 let source='#![allow(warnings)]\nmod host {\n'+fs.readFileSync('bridge/output.rs','utf8')+'\n}\n';
 for(let i=0;i<count;i++)for(const variant of ['sugar','expanded'])if(variant!=='sugar'||pairs[i].rustSugar)source+=`mod c${i}_${variant} { use crate::host::{World,Stdout};\n${pairs[i][variant]} }\n`;
 source+='fn main(){match std::env::args().nth(1).unwrap().as_str(){\n';
 for(let i=0;i<count;i++)for(const variant of ['sugar','expanded'])if(variant!=='sugar'||pairs[i].rustSugar)source+=`"${i}-${variant}"=>host::enter(c${i}_${variant}::main),\n`;
 source+='_=>std::process::exit(2)}}\n';
 const rustSource=path.join(dir,'oracle.rs'),rustBinary=path.join(dir,'oracle');fs.writeFileSync(rustSource,source);
 const flags=['--edition=2024','-C','overflow-checks=yes','-A','warnings'];
 references=await oracleCache({generator:generatorHash,seed:seedText,count,flags,source},async()=>{
  const built=await run('rustc',[...flags,rustSource,'-o',rustBinary]);if(built.error||built.status!==0)throw Error('invalid generated Rust: '+built.stderr);
  const results=[];for(let i=0;i<count;i++)for(const variant of ['sugar','expanded'])results.push(variant==='sugar'&&!pairs[i].rustSugar?null:packResult(await run(rustBinary,[`${i}-${variant}`],{encoding:null})));return results;
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
const routes=await runRoutes(bin,files.map(path=>({path})),{engines,output:dir,encoding:null});
for(const engine of engines){
 const results=routes[engine];tally[engine]={};
 for(let i=0;i<count;i++){
  const sugar=results[2*i],expanded=results[2*i+1],a=ending(sugar),b=ending(expanded);
  let category=a.startsWith('rejected:')?'sugar-'+a:b.startsWith('rejected:')?'expansion-'+b:a!==b?'different-ending':!sugar.stdout.equals(expanded.stdout)?'different-output':a==='normal'?'equal-output-exit':a.startsWith('R')?'equal-output-trap':a;
  if(category.startsWith('equal-')&&references){
   const available=[references[2*i],references[2*i+1]].filter(Boolean).map(value=>unpackResult(value,null));
   if(available.length===2&&(ending(available[0],true)!==ending(available[1],true)||!available[0].stdout.equals(available[1].stdout)))category='oracle-pair-disagrees';
   else for(const reference of available){if(ending(reference,true)!==a){category='rust-ending-differs';break;}if(!reference.stdout.equals(sugar.stdout)){category='rust-output-differs';break;}}
  }
  tally[engine][category]=(tally[engine][category]||0)+1;
  if(!category.startsWith('equal-'))failures.push({index:i,family:pairs[i].family,engine,category,sugar:files[2*i],expanded:files[2*i+1],diagnostic:sugar.stderr.toString()+'\n'+expanded.stderr.toString()});
 }
}
const record={generator:'sugar',sha256:generatorHash,seed:seedText,pairs:count,rust:compareRust,rustSugarPairs:compareRust?pairs.filter(p=>p.rustSugar).length:0,rustExpansionPairs:compareRust?count:0,tally};
console.log(JSON.stringify(record));fs.writeFileSync(path.join(dir,'failures.json'),JSON.stringify(failures,null,2)+'\n');fs.appendFileSync(path.join(output,'runs.jsonl'),JSON.stringify(record)+'\n');if(failures.length){console.error(JSON.stringify(failures.slice(0,4),null,2));process.exitCode=1;}
