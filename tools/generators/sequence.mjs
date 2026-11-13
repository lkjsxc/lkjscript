// Vec method pairs are independently emitted here, without reading the library.
export function sequencePair(index, random) {
 const tag=1+random(30),marker=60+random(20),length=random(4);
 const common=`fn receiver<'a,T>(world:&mut World,values:&'a Vec<T>)->&'a Vec<T>{tick(world,88u8);values}\nfn chosen_index(world:&mut World,index:usize)->usize{tick(world,89u8);index}\n`;
 let sugarHelpers=common,expandedHelpers=common,sugar='',expanded='',family;
 const prefix='let mut values:Vec<M>=Vec::new();'+Array.from({length},(_,i)=>`values.push(make(&mut world,${tag+i}u8));`).join('');
 if(index===0){
  family='vec-is-empty';
  sugar=prefix+'let answer=receiver(&mut world,&values).is_empty();';
  expanded=prefix+'let borrowed=receiver(&mut world,&values);let answer=borrowed.len()==0;';
  const tail=`tick(&mut world,if answer{${marker}u8}else{${marker+1}u8});drop(values);`;
  sugar+=tail;expanded+=tail;
 }else if(index===1){
  family='vec-get';const bound=random(5),at=bound===4?'18446744073709551615usize':bound+'usize';
  sugar=prefix+`{let item=receiver(&mut world,&values).get(chosen_index(&mut world,${at}));`;
  expanded=prefix+`{let borrowed=receiver(&mut world,&values);let index=chosen_index(&mut world,${at});let item=if index<borrowed.len(){Some(&borrowed[index])}else{None};`;
  const tail=`let copied=item;drop(item);match copied{Some(value)=>tick(&mut world,value.b),None=>tick(&mut world,${marker}u8)}}drop(values);`;
  sugar+=tail;expanded+=tail;
 }else if(index===2){
  family='vec-first';
  sugar=prefix+'{let item=receiver(&mut world,&values).first();';
  expanded=prefix+'{let borrowed=receiver(&mut world,&values);let item=if borrowed.len()==0{None}else{Some(&borrowed[0usize])};';
  const tail=`let copied=item;drop(item);match copied{Some(value)=>tick(&mut world,value.b),None=>tick(&mut world,${marker}u8)}}drop(values);`;
  sugar+=tail;expanded+=tail;
 }else{
  family='vec-contains';const mode=random(4),values=[[1,42,-1],[-1,42],[1,2],[]][mode];
  sugarHelpers=expandedHelpers=common+`struct Q{mark:M,value:i64}\nimpl PartialEq for Q{fn eq(&self,other:&Self)->bool{assert!(other.value==42);assert!(self.value!=-1);self.value==other.value}fn ne(&self,_other:&Self)->bool{false}}\nfn query(world:&mut World)->Q{Q{mark:make(world,${tag+10}u8),value:42}}\n`;
  const prefix='let mut values:Vec<Q>=Vec::new();'+values.map((v,i)=>`values.push(Q{mark:make(&mut world,${tag+i}u8),value:${v}i64});`).join('');
  sugar=prefix+'let answer=receiver(&mut world,&values).contains(&query(&mut world));';
  expanded=prefix+'let borrowed=receiver(&mut world,&values);let query=query(&mut world);let mut at=0usize;let mut answer=false;while at<borrowed.len(){if borrowed[at]==query{answer=true;break;}at=at+1;}drop(query);';
  const tail=`tick(&mut world,if answer{${marker}u8}else{${marker+1}u8});drop(values);`;
  sugar+=tail;expanded+=tail;
 }
 return {family,sugar,expanded,sugarHelpers,expandedHelpers};
}

// Clone is selected under a declared generic bound, so an inherent method with
// the same spelling cannot substitute for the element's Clone implementation.
export function extendSlicePair(random) {
 const base=1+random(30), length=random(5), failing=random(4)===0;
 const failAt=length===0?0:random(length);
 const common=`struct C{value:u8,fail:bool,marker:Option<M>}
impl C{fn clone(&self)->Self{C{value:255u8,fail:false,marker:None}}}
impl Clone for C{fn clone(&self)->Self{assert!(!self.fail);C{value:self.value+1u8,fail:false,marker:None}}}
fn element(world:&mut World,value:u8,fail:bool)->C{C{value,fail,marker:Some(make(world,value))}}
fn destination<'a>(world:&mut World,values:&'a mut Vec<C>)->&'a mut Vec<C>{tick(world,88u8);values}
fn input<'a>(world:&mut World,values:&'a Vec<C>)->&'a [C]{tick(world,89u8);values}
`;
 const prefix='let mut source:Vec<C>=Vec::new();'+Array.from({length},(_,i)=>`source.push(element(&mut world,${base+i}u8,${failing&&i===failAt}));`).join('')+`let mut values=vec![element(&mut world,${base+10}u8,false)];`;
 const receiver='destination(&mut world,&mut values)',source='input(&mut world,&source)';
 const tail='tick(&mut world,90u8);let mut index=0usize;while index<values.len(){tick(&mut world,values[index].value);index=index+1;}drop(values);drop(source);tick(&mut world,91u8);';
 return {family:'vec-extend-from-slice',sugarHelpers:common,expandedHelpers:common+'fn append<T:Clone>(target:&mut Vec<T>,source:&[T]){let mut index=0usize;while index<source.len(){let copied=source[index].clone();target.push(copied);index=index+1;}}\n',sugar:prefix+receiver+'.extend_from_slice('+source+');'+tail,expanded:prefix+'append('+receiver+','+source+');'+tail};
}
