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
const [binary='target/debug/lkjscript',seedText='1301',countText='84',output='target/generators']=process.argv.slice(2);
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
function pair(index){
 const a=1+random(50),b=1+random(50),c=1+random(50),marker=60+random(30),yes=random(2)===0?'true':'false';
 let sugar='',expanded='',sugarHelpers='',expandedHelpers='';let family,rustSugar=true;
 switch(index%28){
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

 }
 const tail=`\ntick(&mut world,${90+random(10)}u8);`+(random(9)===0?' assert!(false);':'');
 return {family,rustSugar,sugar:prelude+sugarHelpers+'\npub fn main(mut world:World) {\n'+sugar+tail+'\n}\n',expanded:prelude+expandedHelpers+'\npub fn main(mut world:World) {\n'+expanded+tail+'\n}\n'};
}
fs.mkdirSync(dir,{recursive:true});
const pairs=Array.from({length:count},(_,i)=>pair(i)),files=[];
for(let i=0;i<count;i++)for(const variant of ['sugar','expanded']){const file=path.join(dir,`${i}-${variant}.rs`);fs.writeFileSync(file,pairs[i][variant]);files.push(file);}
const generatorHash=createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).digest('hex');
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
