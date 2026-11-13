// Lazy source adapters and consumers against independently emitted tuple/loop
// state. The expansion never reads the library implementation.
export function iteratorClosurePair(index,random) {
    const family=['map','filter','filter_map','inspect','fold','for_each'][index];
    const base=1+random(20),capture=60+random(20),gap=random(3),calls=2+random(4);
    const trap=random(5)===0;
    const helpers=`struct Source{values:Vec<M>,out:Stdout,calls:usize,gap:usize}
fn source(world:&mut World,base:u8,gap:usize)->Source{tick(world,80u8);let mut values:Vec<M>=Vec::new();values.push(make(world,base));values.push(make(world,base+1u8));values.push(make(world,base+2u8));Source{values,out:world.stdout(),calls:0,gap}}
impl Iterator for Source{type Item=M;fn next(&mut self)->Option<M>{self.out.write(&[90u8]);self.calls=self.calls+1;if self.calls==self.gap{None}else{self.values.pop()}}}
impl Drop for Source{fn drop(&mut self){self.out.write(&[91u8]);}}
`;
    const panic=trap?'if value.b=='+(base+2)+'u8{assert!(false);}':'';
    const result=family==='map'?'value':family==='filter'?'value.b%2u8==0u8':family==='filter_map'?'if value.b%2u8==0u8{Some(value)}else{None}':family==='fold'?'drop(accumulator);value':'()';
    // Unannotated arguments exercise ordinary associated Item equations. The
    // final callable checker still validates fresh borrows and FnMut authority.
    const argument=family==='fold'?'accumulator,value':'value';
    const closure='{tick(&mut world,81u8);let capture=make(&mut world,'+capture+'u8);let mut out=world.stdout();move|'+argument+'|{out.write(&[capture.b]);'+panic+result+'}}';
    const input='source(&mut world,'+base+'u8,'+gap+'usize)';
    const typedConsumer=closure.replace('move|accumulator,value|','move|accumulator:M,value:M|').replace('move|value|','move|value:M|');
    let sugar,expanded;
    if(family==='fold') {
        const initial='make(&mut world,'+(base+3)+'u8)';
        sugar='{let answer='+input+'.fold('+initial+','+closure+');tick(&mut world,82u8);drop(answer);}tick(&mut world,83u8);';
        expanded='{let answer={let mut iterator='+input+';let mut accumulator='+initial+';let mut function='+typedConsumer+';loop{match iterator.next(){Some(value)=>accumulator=function(accumulator,value),None=>break}}accumulator};tick(&mut world,82u8);drop(answer);}tick(&mut world,83u8);';
    } else if(family==='for_each') {
        sugar=input+'.for_each('+closure+');tick(&mut world,82u8);tick(&mut world,83u8);';
        expanded='{let mut iterator='+input+';let mut function='+typedConsumer+';loop{match iterator.next(){Some(value)=>function(value),None=>break}}}tick(&mut world,82u8);tick(&mut world,83u8);';
    } else {
        const consume='match next{Some(value)=>{tick(&mut world,200u8);drop(value);},None=>tick(&mut world,201u8)}';
        const drive=expression=>'let mut round=0usize;while round<'+calls+'usize{let next:Option<M>='+expression+';'+consume+'round=round+1;}tick(&mut world,82u8);';
        const steps={
            map:'match state.0.next(){Some(value)=>Some((state.1)(value)),None=>None}',
            filter:'loop{match state.0.next(){Some(value)=>{if (state.1)(&value){break Some(value);}},None=>break None}}',
            filter_map:'loop{match state.0.next(){Some(value)=>match (state.1)(value){Some(output)=>break Some(output),None=>()},None=>break None}}',
            inspect:'match state.0.next(){Some(value)=>{(state.1)(&value);Some(value)},None=>None}',
        };
        sugar='let mut iterator='+input+'.'+family+'('+closure+');'+drive('iterator.next()')+'drop(iterator);tick(&mut world,83u8);';
        // Type annotations on expansion closures express each independent
        // callable contract, rather than borrowing context from library code.
        const typed=closure.replace('move|value|','move|value:'+(family==='filter'||family==='inspect'?'&M':'M')+'|');
        expanded='let mut state=('+input+','+typed+');'+drive(steps[family])+'drop(state);tick(&mut world,83u8);';
    }
    return {family:'iterator-closure-'+family,sugar,expanded,sugarHelpers:helpers,expandedHelpers:helpers};
}
