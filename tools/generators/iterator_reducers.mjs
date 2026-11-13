// Short-circuit consumers against independently emitted next/callback loops.
// Neither expansion reads the source library implementation.
export function iteratorReducerPair(index, random) {
    const all = index % 2 === 1, owned = index >= 2;
    const scoped = index === 2 || index === 3, temporary = index >= 4;
    const method = all ? 'all' : 'any';
    const base = 1 + random(20), capture = 60 + random(20), length = random(5);
    const gap = random(5), selected = base + random(length + 2);
    const trap = random(5) === 0, trapAt = base + random(length + 1);
    const helpers = `struct ReducerSource { values:Vec<M>, out:Stdout, calls:usize, gap:usize }
fn reducer_source(world:&mut World)->ReducerSource {
 tick(world,80u8);let mut values:Vec<M>=Vec::new();
 ${Array.from({length}, (_, n) => `values.push(make(world,${base + n}u8));`).join('')}
 ReducerSource{values,out:world.stdout(),calls:0,gap:${gap}usize}
}
impl Iterator for ReducerSource {
 type Item=M;
 fn next(&mut self)->Option<M>{
  self.out.write(&[90u8]);self.calls=self.calls+1;
  if self.calls==self.gap{None}else{self.values.pop()}
 }
}
impl Drop for ReducerSource{fn drop(&mut self){self.out.write(&[91u8]);}}
fn reducer_receiver<'a>(world:&mut World,iterator:&'a mut ReducerSource)->&'a mut ReducerSource{tick(world,88u8);iterator}
fn reducer_capture(value:&M)->u8{value.b}
`;
    const closure = `{tick(&mut world,81u8);let capture=make(&mut world,${capture}u8);let mut out=world.stdout();move|value|{out.write(&[reducer_capture(&capture)]);${trap ? `if value.b==${trapAt}u8{assert!(false);}` : ''}value.b${all ? '!=' : '=='}${selected}u8}}`;
    const typed = closure.replace('move|value|', 'move|value:M|');
    const prefix = owned ? '' : 'let mut iterator=reducer_source(&mut world);';
    const receiver = temporary ? 'reducer_source(&mut world)' : scoped ? 'receiver' : 'reducer_receiver(&mut world,&mut iterator)';
    const setup = owned ? 'let mut receiver=reducer_source(&mut world);' : 'let receiver=reducer_receiver(&mut world,&mut iterator);';
    const condition = all ? '!predicate(value)' : 'predicate(value)';
    const expand = `{${setup}let mut predicate=${typed};let mut result=${all};loop{match receiver.next(){Some(value)=>{if ${condition}{result=${!all};break;}},None=>break}}result}`;
    const call = `${receiver}.${method}(${closure})`;
    let sugar = prefix + `let answer=${scoped ? `{let mut receiver=reducer_source(&mut world);${call}}` : call};`;
    let expanded = prefix + `let answer=${expand};`;
    const tail = 'tick(&mut world,if answer{200u8}else{201u8});' + (owned ? '' : 'let mut round=0usize;while round<2usize{match iterator.next(){Some(value)=>{tick(&mut world,value.b);drop(value);},None=>tick(&mut world,202u8)}round=round+1;}drop(iterator);') + 'tick(&mut world,83u8);';
    sugar += tail; expanded += tail;
    return {family:'iterator-'+method+(temporary?'-temporary':scoped?'-owned':'-resume'),sugar,expanded,sugarHelpers:helpers,expandedHelpers:helpers};
}
