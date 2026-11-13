// Source-library combinators against separately emitted match functions.
// The generator owns its expansions; production library source is not read.
const helpers={
option_map:'fn expanded<T,U,F:FnOnce(T)->U>(value:Option<T>,function:F)->Option<U>{match value{Some(value)=>Some(function(value)),None=>None}}',
option_and_then:'fn expanded<T,U,F:FnOnce(T)->Option<U>>(value:Option<T>,function:F)->Option<U>{match value{Some(value)=>function(value),None=>None}}',
option_map_or:'fn expanded<T,U,F:FnOnce(T)->U>(value:Option<T>,default:U,function:F)->U{match value{Some(value)=>function(value),None=>default}}',
option_map_or_else:'fn expanded<T,U,D:FnOnce()->U,F:FnOnce(T)->U>(value:Option<T>,default:D,function:F)->U{match value{Some(value)=>function(value),None=>default()}}',
option_or_else:'fn expanded<T,F:FnOnce()->Option<T>>(value:Option<T>,function:F)->Option<T>{match value{Some(value)=>Some(value),None=>function()}}',
option_unwrap_or_else:'fn expanded<T,F:FnOnce()->T>(value:Option<T>,function:F)->T{match value{Some(value)=>value,None=>function()}}',
option_filter:'fn expanded<T,F:FnOnce(&T)->bool>(value:Option<T>,function:F)->Option<T>{match value{Some(value)=>if function(&value){Some(value)}else{None},None=>None}}',
result_map:'fn expanded<T,E,U,F:FnOnce(T)->U>(value:Result<T,E>,function:F)->Result<U,E>{match value{Ok(value)=>Ok(function(value)),Err(error)=>Err(error)}}',
result_map_err:'fn expanded<T,E,U,F:FnOnce(E)->U>(value:Result<T,E>,function:F)->Result<T,U>{match value{Ok(value)=>Ok(value),Err(error)=>Err(function(error))}}',
result_and_then:'fn expanded<T,E,U,F:FnOnce(T)->Result<U,E>>(value:Result<T,E>,function:F)->Result<U,E>{match value{Ok(value)=>function(value),Err(error)=>Err(error)}}',
result_or_else:'fn expanded<T,E,U,F:FnOnce(E)->Result<T,U>>(value:Result<T,E>,function:F)->Result<T,U>{match value{Ok(value)=>Ok(value),Err(error)=>function(error)}}',
result_map_or:'fn expanded<T,E,U,F:FnOnce(T)->U>(value:Result<T,E>,default:U,function:F)->U{match value{Ok(value)=>function(value),Err(_)=>default}}',
result_map_or_else:'fn expanded<T,E,U,D:FnOnce(E)->U,F:FnOnce(T)->U>(value:Result<T,E>,default:D,function:F)->U{match value{Ok(value)=>function(value),Err(error)=>default(error)}}',
result_unwrap_or_else:'fn expanded<T,E,F:FnOnce(E)->T>(value:Result<T,E>,function:F)->T{match value{Ok(value)=>value,Err(error)=>function(error)}}',
};

const families=[['map','map'],['map_or','map_or'],['map_or_else','map_or_else'],['and_then','and_then'],['or_else','or_else'],['unwrap_or_else','unwrap_or_else'],['filter','map_err']];
export function combinatorPair(index,random){
 const methods=families[index],inputTag=1+random(9),captureTag=11+random(9),defaultTag=21+random(9),filterTag=inputTag+random(2);
 let sugar='',expanded='',expandedHelpers='';
 const sugarHelpers='fn option_input(world:&mut World,yes:bool,b:u8)->Option<M>{tick(world,90u8);if yes{Some(make(world,b))}else{None}}\nfn result_input(world:&mut World,yes:bool,b:u8)->Result<M,M>{tick(world,91u8);if yes{Ok(make(world,b))}else{Err(make(world,b))}}\n';
 function closure(parameter,body,tag){
  const trap=random(7)===0?'assert!(false);':'',output=body.endsWith('Ok(value)')?' -> Result<M,M> ':'';
  return '{tick(&mut world,'+(tag+60)+'u8);let capture=make(&mut world,'+tag+'u8);let mut out:Stdout=world.stdout();move|'+parameter+'|'+output+'{out.write(&['+(tag+80)+'u8]);'+trap+body+'}}';
 }
 for(let position=0;position<2;position++){
  const kind=position===0?'option':'result',method=methods[position],helper='expanded_'+kind;
  expandedHelpers+=helpers[kind+'_'+method].replace('fn expanded','fn '+helper)+'\n';
  for(const yes of [true,false]){
   const input=kind+'_input(&mut world,'+yes+','+inputTag+'u8)';let args=[];
   if(method==='map_or')args.push('make(&mut world,'+defaultTag+'u8)');
   if(method==='map_or_else')args.push(closure(kind==='option'?'':'value:M',kind==='option'?'capture':'drop(capture);value',defaultTag));
   let parameter='value:M',body='drop(capture);value';
   if(method==='and_then')body='drop(capture);'+(kind==='option'?'Some(value)':'Ok(value)');
   if(method==='or_else'){if(kind==='option'){parameter='';body='Some(capture)';}else{body='drop(capture);Ok(value)';}}
   if(method==='unwrap_or_else'&&kind==='option'){parameter='';body='capture';}
   if(method==='filter'){parameter='value:&M';body='drop(capture);value.b=='+filterTag+'u8';}
   args.push(closure(parameter,body,captureTag));
   const annotation=method==='or_else'&&kind==='result'?':Result<M,M>':'';
   sugar+='{let answer'+annotation+'='+input+'.'+method+'('+args.join(',')+');tick(&mut world,70u8);drop(answer);}';
   expanded+='{let answer'+annotation+'='+helper+'('+input+','+args.join(',')+');tick(&mut world,70u8);drop(answer);}';
  }
 }
 return{family:'closure-combinators-'+methods.join('-'),sugar,expanded,sugarHelpers,expandedHelpers:sugarHelpers+expandedHelpers};
}
