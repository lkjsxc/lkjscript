// Generated expansions exercise the contracts that carry owned values and views.
use super::source_cases::{paired, Case, Parameters};
use super::Random;

pub fn sugar(random: &mut Random, index: usize) -> Case {
    let p = Parameters::new(random);
    if index >= 24 && index < 31 {
        return combinator(&p, index - 24);
    }
    if index >= 31 && index < 35 {
        return option_extra(&p, index - 31);
    }
    if index >= 35 && index < 41 {
        return iterator_closure(&p, index - 35);
    }
    if index >= 41 && index < 47 {
        return iterator_reducer(&p, index - 41);
    }
    if index == 47 || index == 48 {
        return option_view(&p, index == 48);
    }
    if index == 49 {
        return contains(&p);
    }
    if index == 50 {
        return extend(&p);
    }
    let(name,helpers,expanded_helpers,body,expanded)=match index{
        0=>("shared-or-product","","","let p=(0i64,1i64,0i64,1i64);match p{(0|1,0|1,0|1,0|1)=>tick(&mut world,$Au8),_=>tick(&mut world,$Bu8)}","let p=(0i64,1i64,0i64,1i64);if (p.0==0||p.0==1)&&(p.1==0||p.1==1)&&(p.2==0||p.2==1)&&(p.3==0||p.3==1){tick(&mut world,$Au8);}else{tick(&mut world,$Bu8);}"),
        1=>("bound-or-product","enum Pick{A(M),B(M)}","enum Pick{A(M),B(M)}","let value=(Pick::A(make(&mut world,$Au8)),Pick::B(make(&mut world,$Bu8)),Pick::A(make(&mut world,$Cu8)));match value{(Pick::A(x)|Pick::B(x),Pick::A(y)|Pick::B(y),Pick::A(z)|Pick::B(z))=>{tick(&mut world,$Mu8);}}","let value=(Pick::A(make(&mut world,$Au8)),Pick::B(make(&mut world,$Bu8)),Pick::A(make(&mut world,$Cu8)));{let x=match value.0{Pick::A(x)=>x,Pick::B(x)=>x};let y=match value.1{Pick::A(x)=>x,Pick::B(x)=>x};let z=match value.2{Pick::A(x)=>x,Pick::B(x)=>x};tick(&mut world,$Mu8);}"),
        2=>("guard-product-selection","","","let value=((0i64,4i64),(0i64,2i64),(0i64,1i64));match value{((x,_)|(_,x),(y,_)|(_,y),(z,_)|(_,z)) if {tick(&mut world,$Mu8);x+y+z==$Ni64}=>tick(&mut world,$Au8),_=>tick(&mut world,$Bu8)}","let value=((0i64,4i64),(0i64,2i64),(0i64,1i64));let mut n=0i64;loop{let x=if n&4!=0{value.0.1}else{value.0.0};let y=if n&2!=0{value.1.1}else{value.1.0};let z=if n&1!=0{value.2.1}else{value.2.0};tick(&mut world,$Mu8);if x+y+z==$Ni64{tick(&mut world,$Au8);break;}n+=1;if n==8{tick(&mut world,$Bu8);break;}}"),
        3=>("guard-mutable-view","enum Pick{A(Box<i64>),B(Box<i64>)}","enum Pick{A(Box<i64>),B(Box<i64>)}","let mut value=if $Y{Pick::A(Box::new($Ai64))}else{Pick::B(Box::new($Bi64))};match value{Pick::A(ref mut x)|Pick::B(ref mut x) if {tick(&mut world,$Mu8);**x>0}=>{**x+=1;},_=>()}match value{Pick::A(x)|Pick::B(x)=>tick(&mut world,*x as u8)}","let mut value=if $Y{Pick::A(Box::new($Ai64))}else{Pick::B(Box::new($Bi64))};match value{Pick::A(ref mut x)=>{tick(&mut world,$Mu8);if **x>0{**x+=1;}},Pick::B(ref mut x)=>{tick(&mut world,$Mu8);if **x>0{**x+=1;}}}match value{Pick::A(x)|Pick::B(x)=>tick(&mut world,*x as u8)}"),
        4=>("guard-outer-controls","","","let mut n=0i64;let answer=loop{n+=1;match(0i64,0i64){(0,_)|(_,0) if {tick(&mut world,$Mu8);if n==1{continue;}if n==3{break $Au8;}false}=>(),_=>()}};tick(&mut world,answer);","let mut n=0i64;let answer=loop{n+=1;if {tick(&mut world,$Mu8);if n==1{continue;}if n==3{break $Au8;}false}{}else if {tick(&mut world,$Mu8);if n==1{continue;}if n==3{break $Au8;}false}{}};tick(&mut world,answer);"),
        5=>("guard-cached-discriminant","","","let value=if $Y{Some(make(&mut world,$Au8))}else{None};match value{Some(_) if {drop(value);false}=>(),Some(_)=>tick(&mut world,$Mu8),None=>tick(&mut world,$Bu8)}","let value=if $Y{Some(make(&mut world,$Au8))}else{None};let some=match &value{Some(_)=>true,None=>false};if some{drop(value);tick(&mut world,$Mu8);}else{tick(&mut world,$Bu8);}"),
        6=>("shared-guard-or-32","","","",""),
        7=>("infeasible-or-subtree-32","","","",""),
        8=>("repeated-vector-clone","struct C{v:u8}impl Clone for C{fn clone(&self)->Self{C{v:self.v+1}}}fn element(world:&mut World)->C{tick(world,$Au8);C{v:$Bu8}}fn count(world:&mut World)->usize{tick(world,$Mu8);$Nusize}","struct C{v:u8}impl Clone for C{fn clone(&self)->Self{C{v:self.v+1}}}fn element(world:&mut World)->C{tick(world,$Au8);C{v:$Bu8}}fn count(world:&mut World)->usize{tick(world,$Mu8);$Nusize}","let values=vec![element(&mut world);count(&mut world)];for value in values{tick(&mut world,value.v);}","let value=element(&mut world);let size=count(&mut world);let mut values=Vec::new();if size>0{let mut index=1usize;while index<size{values.push(value.clone());index+=1;}values.push(value);}else{drop(value);}for value in values{tick(&mut world,value.v);}"),
        9=>("format-padding-unicode","","","let a=$Ai64;let b=$Bi64;let label=\"水\";let value=format!(\"{{{label:😀^5}}}:{b:04}:{a}\");world.stdout().write(value.as_bytes());","let a=$Ai64;let b=$Bi64;let mut value=\"{😀😀水😀😀}:\".to_string();let digits=format!(\"{b}\");for _ in digits.len()..4usize{value.push('0');}value.push_str(&digits);value.push(':');value.push_str(&format!(\"{a}\"));world.stdout().write(value.as_bytes());"),
        10=>("display-temporary-cleanup","struct Shown{owner:M,value:i64}impl std::fmt::Display for Shown{fn fmt(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{write!(f,\"[{}]\",self.value)}}fn shown(world:&mut World)->Shown{Shown{owner:make(world,$Au8),value:$Bi64}}","struct Shown{owner:M,value:i64}fn shown(world:&mut World)->Shown{Shown{owner:make(world,$Au8),value:$Bi64}}","let text=format!(\"{0}-{0}\",shown(&mut world));tick(&mut world,$Mu8);world.stdout().write(text.as_bytes());","let value=shown(&mut world);let text=\"[$B]-[$B]\".to_string();drop(value);tick(&mut world,$Mu8);world.stdout().write(text.as_bytes());"),
        11=>("format-error-stops","use std::fmt::Write;struct Bad;impl std::fmt::Display for Bad{fn fmt(&self,f:&mut std::fmt::Formatter)->std::fmt::Result{f.write_str(\"!\").unwrap();Err(std::fmt::Error)}}","","let mut text=String::new();let result=write!(&mut text,\"before{}after\",Bad);assert!(result.is_err());world.stdout().write(text.as_bytes());","world.stdout().write(b\"before!\");"),
        12=>("result-owned-combinators","fn input(world:&mut World,yes:bool,b:u8)->Result<M,M>{let value=make(world,b);if yes{Ok(value)}else{Err(value)}}","fn input(world:&mut World,yes:bool,b:u8)->Result<M,M>{let value=make(world,b);if yes{Ok(value)}else{Err(value)}}fn and(a:Result<M,M>,b:Result<M,M>)->Result<M,M>{match a{Ok(_)=>b,Err(e)=>Err(e)}}fn or(a:Result<M,M>,b:Result<M,M>)->Result<M,M>{match a{Ok(v)=>Ok(v),Err(_)=>b}}","drop(input(&mut world,$Y,$Au8).and(input(&mut world,!$Y,$Bu8)));drop(input(&mut world,$Y,$Au8).or(input(&mut world,!$Y,$Bu8)));let result:Result<Option<M>,M>=if $Y{Ok(Some(make(&mut world,$Au8)))}else{Err(make(&mut world,$Bu8))};drop(result.transpose());","drop(and(input(&mut world,$Y,$Au8),input(&mut world,!$Y,$Bu8)));drop(or(input(&mut world,$Y,$Au8),input(&mut world,!$Y,$Bu8)));let result:Result<Option<M>,M>=if $Y{Ok(Some(make(&mut world,$Au8)))}else{Err(make(&mut world,$Bu8))};let result=match result{Ok(Some(v))=>Some(Ok(v)),Ok(None)=>None,Err(e)=>Some(Err(e))};drop(result);"),
        13=>("option-insert-replace","","fn insert(v:&mut Option<M>,n:M)->&mut M{*v=Some(n);match v{Some(v)=>v,None=>unreachable!()}}fn get(v:&mut Option<M>,n:M)->&mut M{if v.is_none(){*v=Some(n);}match v{Some(v)=>v,None=>unreachable!()}}","let mut v=if $Y{Some(make(&mut world,$Au8))}else{None};{let r=v.get_or_insert(make(&mut world,$Bu8));tick(&mut world,r.b);}{let r=v.insert(make(&mut world,$Cu8));r.b=$Au8;}drop(v.replace(make(&mut world,$Bu8)));","let mut v=if $Y{Some(make(&mut world,$Au8))}else{None};{let r=get(&mut v,make(&mut world,$Bu8));tick(&mut world,r.b);}{let r=insert(&mut v,make(&mut world,$Cu8));r.b=$Au8;}drop(std::mem::replace(&mut v,Some(make(&mut world,$Bu8))));"),
        14=>("temporary-iterable-controls","fn values(world:&mut World)->Vec<M>{vec![make(world,$Au8),make(world,$Bu8)]}fn visit(world:&mut World,mode:u8){for item in &values(world){let local=make(world,$Cu8);tick(world,item.b);if mode==0{continue;}if mode==1{break;}if mode==2{return;}drop(local);}tick(world,$Mu8);}","fn values(world:&mut World)->Vec<M>{vec![make(world,$Au8),make(world,$Bu8)]}fn into<I:IntoIterator>(v:I)->I::IntoIter{v.into_iter()}fn visit(world:&mut World,mode:u8){match into(&values(world)){mut iterator=>{loop{match iterator.next(){Some(item)=>{let local=make(world,$Cu8);tick(world,item.b);if mode==0{continue;}if mode==1{break;}if mode==2{return;}drop(local);},None=>break}}}}tick(world,$Mu8);}","for mode in 0..4u8{visit(&mut world,mode);}","for mode in 0..4u8{visit(&mut world,mode);}"),
        15=>("iterator-patterns","","","let values=vec![($Au8,$Bu8),($Bu8,$Cu8)];for(left,right)in values.iter(){tick(&mut world,*left);tick(&mut world,*right);}for ref mut value in 0..$Ni64{*value+=10;tick(&mut world,*value as u8);}","let values=vec![($Au8,$Bu8),($Bu8,$Cu8)];let mut iterator=values.iter();loop{match iterator.next(){Some(pair)=>{tick(&mut world,pair.0);tick(&mut world,pair.1);},None=>break}}let mut n=0i64;while n<$Ni64{let mut item=n;let value=&mut item;*value+=10;tick(&mut world,*value as u8);n+=1;}"),
        16=>("shared-range-forms","","","let values=[$Au16,$Bu16,$Cu16,$Mu16];let a=&values[1..=2];for v in a{tick(&mut world,*v as u8);}let a=&values[..];for v in a{tick(&mut world,*v as u8);}","let values=[$Au16,$Bu16,$Cu16,$Mu16];for i in 1..3usize{tick(&mut world,values[i] as u8);}for i in 0..values.len(){tick(&mut world,values[i] as u8);}"),
        17=>("returned-captured-range","fn selected<'a>(v:&'a[u8])->&'a[u8]{&v[1..3]}","struct Chosen<'a>{data:&'a[u8],start:usize,end:usize}fn selected<'a>(v:&'a[u8])->Chosen<'a>{Chosen{data:v,start:1,end:3}}","let values=vec![$Au8,$Bu8,$Cu8,$Mu8];let captured=||selected(&values);let view=captured();drop(captured);for v in view{tick(&mut world,*v);}","let values=vec![$Au8,$Bu8,$Cu8,$Mu8];let captured=||selected(&values);let view=captured();drop(captured);for i in view.start..view.end{tick(&mut world,view.data[i]);}"),
        18=>("range-operand-order","fn source<'a>(w:&mut World,v:&'a mut[u8])->&'a mut[u8]{tick(w,80);v}fn start(w:&mut World)->usize{tick(w,81);1}fn end(w:&mut World)->usize{tick(w,82);3}","fn source<'a>(w:&mut World,v:&'a mut[u8])->&'a mut[u8]{tick(w,80);v}fn start(w:&mut World)->usize{tick(w,81);1}fn end(w:&mut World)->usize{tick(w,82);3}","let mut values=vec![$Au8,$Bu8,$Cu8,$Mu8];let selected=&mut source(&mut world,&mut values)[start(&mut world)..end(&mut world)];selected[0]=$Au8;world.stdout().write(selected);","let mut values=vec![$Au8,$Bu8,$Cu8,$Mu8];let whole=source(&mut world,&mut values);let left=start(&mut world);let right=end(&mut world);whole[left]=$Au8;for i in left..right{tick(&mut world,whole[i]);}"),
        19=>("range-bounds-trap","","","let bytes=vec![$Au8,$Bu8,$Cu8];let end=$Nusize;let selected=&bytes[..end];world.stdout().write(selected);","let bytes=vec![$Au8,$Bu8,$Cu8];let end=$Nusize;if end>bytes.len(){let invalid=bytes.len();tick(&mut world,bytes[invalid]);}for i in 0..end{tick(&mut world,bytes[i]);}"),
        20=>("traced-skip-take","","","",""),
        21=>("traced-zip","","","",""),
        22=>("traced-chain","","","",""),
        _=>("ordered-collection-iteration","","","let mut map=std::collections::BTreeMap::new();map.insert(2u8,make(&mut world,$Bu8));map.insert(1u8,make(&mut world,$Au8));let mut set=std::collections::BTreeSet::new();set.insert(2u8);set.insert(1u8);set.insert(2u8);for(k,v)in &map{tick(&mut world,*k);tick(&mut world,v.b);}for k in map.keys(){tick(&mut world,*k);}for v in map.values(){tick(&mut world,v.b);}for v in &set{tick(&mut world,*v);}","let mut map=std::collections::BTreeMap::new();map.insert(2u8,make(&mut world,$Bu8));map.insert(1u8,make(&mut world,$Au8));let mut set=std::collections::BTreeSet::new();set.insert(2u8);set.insert(1u8);set.insert(2u8);for k in 1..=2u8{tick(&mut world,k);tick(&mut world,map.get(&k).unwrap().b);}tick(&mut world,1);tick(&mut world,2);tick(&mut world,map.get(&1u8).unwrap().b);tick(&mut world,map.get(&2u8).unwrap().b);tick(&mut world,1);tick(&mut world,2);"),
    };
    if index == 6 || index == 7 {
        let mut values = String::new();
        let mut patterns = String::new();
        let mut conditions = String::new();
        for n in 0..32usize {
            if n != 0 {
                values.push(',');
                patterns.push(',');
                conditions.push_str("&&");
            }
            values.push_str(if index == 7 && n == 31 {
                "1i64"
            } else {
                "0i64"
            });
            patterns.push_str("0|0");
            conditions.push_str(&format!("value.0.{n}==0"));
        }
        let prefix = format!("let value=(({values}),make(&mut world,$Au8));");
        let source = if index == 6 {
            format!("{prefix}match value{{(({patterns}),m) if {{tick(&mut world,$Mu8);true}}=>drop(m),_=>tick(&mut world,$Bu8)}}")
        } else {
            format!("{prefix}match value{{(({patterns}),m)|(_,m) if {{tick(&mut world,$Mu8);true}}=>drop(m),_=>tick(&mut world,$Bu8)}}")
        };
        let reference = if index == 6 {
            format!("{prefix}if {conditions}{{let m=value.1;tick(&mut world,$Mu8);drop(m);}}else{{tick(&mut world,$Bu8);}}")
        } else {
            format!("{prefix}let m=value.1;tick(&mut world,$Mu8);drop(m);")
        };
        return paired(&p, name, "", "", &source, &reference);
    }
    if index >= 20 && index <= 22 {
        return traced(&p, index - 20);
    }
    paired(&p, name, helpers, expanded_helpers, body, expanded)
}
fn traced(p: &Parameters, index: usize) -> Case {
    let helpers="struct Trace{values:Vec<M>,out:Stdout,tag:u8}impl Drop for Trace{fn drop(&mut self){self.out.write(&[self.tag+1]);}}impl Iterator for Trace{type Item=M;fn next(&mut self)->Option<M>{self.out.write(&[self.tag]);self.values.pop()}}fn traced(world:&mut World,tag:u8,count:usize)->Trace{let mut values=Vec::new();for i in 0..count{values.push(make(world,$Au8+i as u8));}Trace{values,out:world.stdout(),tag}}";
    let (name,source,reference)=match index{
        0=>("iterator-enumerate-skip-take","let mut iterator=traced(&mut world,70,3).skip($Nusize).take(2).enumerate();while let Some((index,value))=iterator.next(){tick(&mut world,index as u8);tick(&mut world,value.b);if $Y{break;}drop(value);}tick(&mut world,$Mu8);drop(iterator);","let mut iterator=traced(&mut world,70,3);let mut skipping=$Nusize;let mut remaining=2usize;let mut index=0usize;loop{if remaining==0{break;}remaining-=1;let mut ended=false;while skipping>0{skipping-=1;match iterator.next(){Some(value)=>drop(value),None=>{skipping=0;ended=true;break;}}}let next=if ended{None}else{iterator.next()};match next{Some(value)=>{tick(&mut world,index as u8);tick(&mut world,value.b);if $Y{break;}drop(value);index+=1;},None=>break}}tick(&mut world,$Mu8);drop(iterator);"),
        1=>("iterator-zip-next-order","let mut iterator=traced(&mut world,70,$Nusize).zip(traced(&mut world,80,2).take(9));while let Some((x,y))=iterator.next(){tick(&mut world,x.b);tick(&mut world,y.b);drop(y);drop(x);if $Y{break;}}let last=iterator.next();drop(last);tick(&mut world,$Mu8);drop(iterator);","let mut left=traced(&mut world,70,$Nusize);let mut right=traced(&mut world,80,2);loop{match left.next(){Some(x)=>match right.next(){Some(y)=>{tick(&mut world,x.b);tick(&mut world,y.b);drop(y);drop(x);if $Y{break;}},None=>{drop(x);break;}},None=>break}}let last=match left.next(){Some(x)=>match right.next(){Some(y)=>Some((x,y)),None=>{drop(x);None}},None=>None};drop(last);tick(&mut world,$Mu8);drop(left);drop(right);"),
        _=>("iterator-chain-exhaustion","let mut iterator=traced(&mut world,70,$Nusize).chain(traced(&mut world,80,2));while let Some(value)=iterator.next(){tick(&mut world,value.b);drop(value);if $Y{break;}}tick(&mut world,$Mu8);drop(iterator);","let mut left=Some(traced(&mut world,70,$Nusize));let mut right=traced(&mut world,80,2);loop{let mut next=None;match left.as_mut(){Some(iterator)=>{next=iterator.next();},None=>()}if next.is_none(){left=None;next=right.next();}match next{Some(value)=>{tick(&mut world,value.b);drop(value);if $Y{break;}},None=>break}}tick(&mut world,$Mu8);drop(left);drop(right);"),
    };
    paired(p, name, helpers, helpers, source, reference)
}

fn callback(kind: &str, method: &str, default: bool) -> String {
    let (parameter, body) = if default {
        if kind == "option" {
            ("", "capture")
        } else {
            ("value:M", "drop(capture);value")
        }
    } else if method == "and_then" {
        if kind == "option" {
            ("value:M", "drop(capture);Some(value)")
        } else {
            ("value:M", "drop(capture);Ok(value)")
        }
    } else if method == "or_else" {
        if kind == "option" {
            ("", "Some(capture)")
        } else {
            ("value:M", "drop(capture);Ok(value)")
        }
    } else if method == "unwrap_or_else" && kind == "option" {
        ("", "capture")
    } else if method == "filter" {
        ("value:&M", "drop(capture);value.b==$Au8")
    } else {
        ("value:M", "drop(capture);value")
    };
    let annotation = if body.ends_with("Ok(value)") {
        "->Result<M,M>"
    } else {
        ""
    };
    format!("{{tick(&mut world,70u8);let capture=make(&mut world,$Cu8);let mut out=world.stdout();move|{parameter}|{annotation}{{out.write(&[80u8]);if $Y&&$Nusize==0{{assert!(false);}}{body}}}}}")
}
fn combinator_helper(kind: &str, method: &str) -> &'static str {
    if kind == "option" {
        match method{
            "map"=>"fn expanded<T,U,F:FnOnce(T)->U>(v:Option<T>,f:F)->Option<U>{match v{Some(v)=>Some(f(v)),None=>None}}",
            "map_or"=>"fn expanded<T,U,F:FnOnce(T)->U>(v:Option<T>,d:U,f:F)->U{match v{Some(v)=>f(v),None=>d}}",
            "map_or_else"=>"fn expanded<T,U,D:FnOnce()->U,F:FnOnce(T)->U>(v:Option<T>,d:D,f:F)->U{match v{Some(v)=>f(v),None=>d()}}",
            "and_then"=>"fn expanded<T,U,F:FnOnce(T)->Option<U>>(v:Option<T>,f:F)->Option<U>{match v{Some(v)=>f(v),None=>None}}",
            "or_else"=>"fn expanded<T,F:FnOnce()->Option<T>>(v:Option<T>,f:F)->Option<T>{match v{Some(v)=>Some(v),None=>f()}}",
            "unwrap_or_else"=>"fn expanded<T,F:FnOnce()->T>(v:Option<T>,f:F)->T{match v{Some(v)=>v,None=>f()}}",
            _=>"fn expanded<T,F:FnOnce(&T)->bool>(v:Option<T>,f:F)->Option<T>{match v{Some(v)=>if f(&v){Some(v)}else{None},None=>None}}",
        }
    } else {
        match method{
            "map"=>"fn expanded<T,E,U,F:FnOnce(T)->U>(v:Result<T,E>,f:F)->Result<U,E>{match v{Ok(v)=>Ok(f(v)),Err(e)=>Err(e)}}",
            "map_or"=>"fn expanded<T,E,U,F:FnOnce(T)->U>(v:Result<T,E>,d:U,f:F)->U{match v{Ok(v)=>f(v),Err(_)=>d}}",
            "map_or_else"=>"fn expanded<T,E,U,D:FnOnce(E)->U,F:FnOnce(T)->U>(v:Result<T,E>,d:D,f:F)->U{match v{Ok(v)=>f(v),Err(e)=>d(e)}}",
            "and_then"=>"fn expanded<T,E,U,F:FnOnce(T)->Result<U,E>>(v:Result<T,E>,f:F)->Result<U,E>{match v{Ok(v)=>f(v),Err(e)=>Err(e)}}",
            "or_else"=>"fn expanded<T,E,U,F:FnOnce(E)->Result<T,U>>(v:Result<T,E>,f:F)->Result<T,U>{match v{Ok(v)=>Ok(v),Err(e)=>f(e)}}",
            "unwrap_or_else"=>"fn expanded<T,E,F:FnOnce(E)->T>(v:Result<T,E>,f:F)->T{match v{Ok(v)=>v,Err(e)=>f(e)}}",
            _=>"fn expanded<T,E,U,F:FnOnce(E)->U>(v:Result<T,E>,f:F)->Result<T,U>{match v{Ok(v)=>Ok(v),Err(e)=>Err(f(e))}}",
        }
    }
}
fn combinator(p: &Parameters, index: usize) -> Case {
    let method = match index {
        0 => "map",
        1 => "map_or",
        2 => "map_or_else",
        3 => "and_then",
        4 => "or_else",
        5 => "unwrap_or_else",
        _ => "filter",
    };
    let helpers="fn option_input(w:&mut World,yes:bool)->Option<M>{tick(w,90);if yes{Some(make(w,$Au8))}else{None}}fn result_input(w:&mut World,yes:bool)->Result<M,M>{tick(w,91);if yes{Ok(make(w,$Au8))}else{Err(make(w,$Au8))}}";
    let mut expanded_helpers = helpers.to_string();
    let mut source = String::new();
    let mut reference = String::new();
    for position in 0..2usize {
        let kind = if position == 0 { "option" } else { "result" };
        let selected = if index == 6 && position == 1 {
            "map_err"
        } else {
            method
        };
        expanded_helpers.push_str(
            &combinator_helper(kind, selected)
                .replace("fn expanded", &format!("fn expanded_{kind}")),
        );
        for present in 0..2usize {
            let yes = present == 0;
            let input = format!("{kind}_input(&mut world,{yes})");
            let mut args = String::new();
            if selected == "map_or" {
                args.push_str("make(&mut world,$Bu8),");
            }
            if selected == "map_or_else" {
                args.push_str(&callback(kind, selected, true));
                args.push(',');
            }
            args.push_str(&callback(kind, selected, false));
            let annotation = if selected == "or_else" && kind == "result" {
                ":Result<M,M>"
            } else {
                ""
            };
            source.push_str(&format!("{{let answer{annotation}={input}.{selected}({args});tick(&mut world,60);drop(answer);}}"));
            reference.push_str(&format!("{{let answer{annotation}=expanded_{kind}({input},{args});tick(&mut world,60);drop(answer);}}"));
        }
    }
    paired(
        p,
        &format!("closure-combinator-{method}"),
        helpers,
        &expanded_helpers,
        &source,
        &reference,
    )
}
fn option_extra(p: &Parameters, index: usize) -> Case {
    let(method,expanded)=match index{
        0=>("is_some_and","fn expanded<T,F:FnOnce(T)->bool>(v:Option<T>,f:F)->bool{match v{Some(v)=>f(v),None=>false}}"),
        1=>("is_none_or","fn expanded<T,F:FnOnce(T)->bool>(v:Option<T>,f:F)->bool{match v{Some(v)=>f(v),None=>true}}"),
        2=>("ok_or","fn expanded<T,E>(v:Option<T>,e:E)->Result<T,E>{match v{Some(v)=>Ok(v),None=>Err(e)}}"),
        _=>("ok_or_else","fn expanded<T,E,F:FnOnce()->E>(v:Option<T>,f:F)->Result<T,E>{match v{Some(v)=>Ok(v),None=>Err(f())}}"),
    };
    let common="fn input(w:&mut World,yes:bool)->Option<M>{tick(w,60);if yes{Some(make(w,$Au8))}else{None}}";
    let mut source = String::new();
    let mut reference = String::new();
    for present in 0..2usize {
        let yes = present == 0;
        let input = format!("input(&mut world,{yes})");
        let argument = if index == 2 {
            "make(&mut world,$Bu8)".to_string()
        } else {
            let parameter = if index < 2 { "value:M" } else { "" };
            let body = if index < 2 {
                "let answer=value.b<$Bu8;drop(capture);answer"
            } else {
                "capture"
            };
            format!("{{tick(&mut world,70);let capture=make(&mut world,$Cu8);let mut out=world.stdout();move|{parameter}|{{out.write(&[80u8]);if $Y&&$Nusize==0{{assert!(false);}}{body}}}}}")
        };
        let tail = if index < 2 {
            "tick(&mut world,answer as u8);"
        } else {
            "match answer{Ok(v)=>{tick(&mut world,10);tick(&mut world,v.b);},Err(e)=>{tick(&mut world,11);tick(&mut world,e.b);}}"
        };
        source.push_str(&format!(
            "{{let answer={input}.{method}({argument});tick(&mut world,90);{tail}}}"
        ));
        reference.push_str(&format!(
            "{{let answer=expanded({input},{argument});tick(&mut world,90);{tail}}}"
        ));
    }
    paired(
        p,
        &format!("option-{method}"),
        common,
        &format!("{common}{expanded}"),
        &source,
        &reference,
    )
}
fn option_view(p: &Parameters, clone: bool) -> Case {
    let common="#[derive(Copy)]struct C{value:u8,fail:bool}impl Clone for C{fn clone(&self)->Self{assert!(!self.fail);C{value:self.value+1,fail:false}}}fn input<'a>(w:&mut World,v:&'a mut C,present:bool)->Option<&'a mut C>{tick(w,87);if present{Some(v)}else{None}}";
    let helper = if clone {
        "fn convert<T:Clone>(v:Option<&mut T>)->Option<T>{match v{Some(v)=>Some((*v).clone()),None=>None}}"
    } else {
        "fn convert<T:Copy>(v:Option<&mut T>)->Option<T>{match v{Some(v)=>Some(*v),None=>None}}"
    };
    let method = if clone { "cloned" } else { "copied" };
    let prefix = "let owner=make(&mut world,$Au8);let mut value=C{value:$Au8,fail:$Nusize==0};";
    let tail="tick(&mut world,88);value.value=0;match answer{Some(v)=>tick(&mut world,v.value),None=>tick(&mut world,89)}drop(owner);tick(&mut world,90);";
    paired(
        p,
        &format!("option-reference-{method}"),
        common,
        &format!("{common}{helper}"),
        &format!("{prefix}let answer=input(&mut world,&mut value,$Y).{method}();{tail}"),
        &format!("{prefix}let answer=convert(input(&mut world,&mut value,$Y));{tail}"),
    )
}
fn contains(p: &Parameters) -> Case {
    let helpers="struct Q{mark:M,value:i64}impl PartialEq for Q{fn eq(&self,other:&Self)->bool{assert!(other.value==42);assert!(self.value!=-1);self.value==other.value}fn ne(&self,_other:&Self)->bool{false}}fn query(w:&mut World)->Q{Q{mark:make(w,$Cu8),value:42}}fn receiver<'a>(w:&mut World,v:&'a Vec<Q>)->&'a Vec<Q>{tick(w,88);v}";
    let prefix="let mut values=Vec::new();values.push(Q{mark:make(&mut world,$Au8),value:if $Y{42}else{1}});values.push(Q{mark:make(&mut world,$Bu8),value:-1});";
    let tail = "tick(&mut world,answer as u8);drop(values);";
    paired(p,"vec-contains-short-circuit",helpers,helpers,&format!("{prefix}let answer=receiver(&mut world,&values).contains(&query(&mut world));{tail}"),&format!("{prefix}let borrowed=receiver(&mut world,&values);let query=query(&mut world);let mut at=0usize;let mut answer=false;while at<borrowed.len(){{if borrowed[at]==query{{answer=true;break;}}at+=1;}}drop(query);{tail}"))
}
fn extend(p: &Parameters) -> Case {
    let common="struct C{value:u8,fail:bool,marker:Option<M>}impl C{fn clone(&self)->Self{C{value:255,fail:false,marker:None}}}impl Clone for C{fn clone(&self)->Self{assert!(!self.fail);C{value:self.value+1,fail:false,marker:None}}}fn element(w:&mut World,v:u8,fail:bool)->C{C{value:v,fail,marker:Some(make(w,v))}}fn destination<'a>(w:&mut World,v:&'a mut Vec<C>)->&'a mut Vec<C>{tick(w,88);v}fn input<'a>(w:&mut World,v:&'a Vec<C>)->&'a[C]{tick(w,89);v}";
    let prefix="let mut source=Vec::new();for i in 0..$Nusize{source.push(element(&mut world,$Au8+i as u8,$Y&&i==1));}let mut values=vec![element(&mut world,$Bu8,false)];";
    let tail="tick(&mut world,90);for value in &values{tick(&mut world,value.value);}drop(values);drop(source);tick(&mut world,91);";
    paired(p,"vec-extend-clone-contract",common,&format!("{common}fn append<T:Clone>(target:&mut Vec<T>,source:&[T]){{let mut index=0usize;while index<source.len(){{let copied=source[index].clone();target.push(copied);index+=1;}}}}"),&format!("{prefix}destination(&mut world,&mut values).extend_from_slice(input(&mut world,&source));{tail}"),&format!("{prefix}append(destination(&mut world,&mut values),input(&mut world,&source));{tail}"))
}

fn iterator_helpers() -> &'static str {
    "struct Source{values:Vec<M>,out:Stdout,calls:usize,gap:usize}fn source(world:&mut World,base:u8,gap:usize)->Source{tick(world,80);let values=vec![make(world,base),make(world,base+1),make(world,base+2)];Source{values,out:world.stdout(),calls:0,gap}}impl Iterator for Source{type Item=M;fn next(&mut self)->Option<M>{self.out.write(&[90u8]);self.calls+=1;if self.calls==self.gap{None}else{self.values.pop()}}}impl Drop for Source{fn drop(&mut self){self.out.write(&[91u8]);}}"
}
fn iterator_closure(p: &Parameters, index: usize) -> Case {
    let(method,parameter,typed,result,step)=match index{
        0=>("map","value","value:M","value","match state.0.next(){Some(value)=>Some((state.1)(value)),None=>None}"),
        1=>("filter","value","value:&M","value.b%2u8==0u8","loop{match state.0.next(){Some(value)=>{if (state.1)(&value){break Some(value);}},None=>break None}}"),
        2=>("filter_map","value","value:M","if value.b%2u8==0u8{Some(value)}else{None}","loop{match state.0.next(){Some(value)=>match (state.1)(value){Some(output)=>break Some(output),None=>()},None=>break None}}"),
        3=>("inspect","value","value:&M","()","match state.0.next(){Some(value)=>{(state.1)(&value);Some(value)},None=>None}"),
        4=>("fold","accumulator,value","accumulator:M,value:M","drop(accumulator);value",""),
        _=>("for_each","value","value:M","()",""),
    };
    let before =
        "{tick(&mut world,81);let capture=make(&mut world,$Cu8);let mut out=world.stdout();";
    let body =
        format!("out.write(&[capture.b]);if $Y&&value.b==$Au8+2u8{{assert!(false);}}{result}");
    let closure = format!("{before}move|{parameter}|{{{body}}}}}");
    let typed_closure = format!("{before}move|{typed}|{{{body}}}}}");
    let input = "source(&mut world,$Au8,$Nusize)";
    let (source, reference) = if index == 4 {
        (format!("{{let answer={input}.fold(make(&mut world,$Bu8),{closure});tick(&mut world,82);drop(answer);}}tick(&mut world,83);"),format!("{{let answer={{let mut iterator={input};let mut accumulator=make(&mut world,$Bu8);let mut function={typed_closure};loop{{match iterator.next(){{Some(value)=>accumulator=function(accumulator,value),None=>break}}}}accumulator}};tick(&mut world,82);drop(answer);}}tick(&mut world,83);"))
    } else if index == 5 {
        (format!("{input}.for_each({closure});tick(&mut world,82);tick(&mut world,83);"),format!("{{let mut iterator={input};let mut function={typed_closure};loop{{match iterator.next(){{Some(value)=>function(value),None=>break}}}}}}tick(&mut world,82);tick(&mut world,83);"))
    } else {
        let consume="match next{Some(value)=>{tick(&mut world,200);drop(value);},None=>tick(&mut world,201)}round+=1;";
        (format!("let mut iterator={input}.{method}({closure});let mut round=0usize;while round<2+$Nusize{{let next:Option<M>=iterator.next();{consume}}}tick(&mut world,82);drop(iterator);tick(&mut world,83);"),format!("let mut state=({input},{typed_closure});let mut round=0usize;while round<2+$Nusize{{let next:Option<M>={step};{consume}}}tick(&mut world,82);drop(state);tick(&mut world,83);"))
    };
    paired(
        p,
        &format!("iterator-callback-{method}"),
        iterator_helpers(),
        iterator_helpers(),
        &source,
        &reference,
    )
}
fn iterator_reducer(p: &Parameters, index: usize) -> Case {
    let all = index % 2 == 1;
    let owned = index >= 2;
    let scoped = index == 2 || index == 3;
    let temporary = index >= 4;
    let method = if all { "all" } else { "any" };
    let relation = if all { "!=" } else { "==" };
    let body = format!(
        "out.write(&[capture.b]);if $Y&&value.b==$Au8+2u8{{assert!(false);}}value.b{relation}$Bu8"
    );
    let closure=format!("{{tick(&mut world,81);let capture=make(&mut world,$Cu8);let mut out=world.stdout();move|value|{{{body}}}}}");
    let typed=format!("{{tick(&mut world,81);let capture=make(&mut world,$Cu8);let mut out=world.stdout();move|value:M|{{{body}}}}}");
    let prefix = if owned {
        ""
    } else {
        "let mut iterator=source(&mut world,$Au8,$Nusize);"
    };
    let receiver = if temporary {
        "source(&mut world,$Au8,$Nusize)"
    } else if scoped {
        "receiver"
    } else {
        "receiver(&mut world,&mut iterator)"
    };
    let setup = if owned {
        "let mut receiver=source(&mut world,$Au8,$Nusize);"
    } else {
        "let receiver=receiver(&mut world,&mut iterator);"
    };
    let condition = if all {
        "!predicate(value)"
    } else {
        "predicate(value)"
    };
    let expanded=format!("{{{setup}let mut predicate={typed};let mut result={all};loop{{match receiver.next(){{Some(value)=>{{if {condition}{{result={};break;}}}},None=>break}}}}result}}",!all);
    let call = format!("{receiver}.{method}({closure})");
    let source_call = if scoped {
        format!("{{let mut receiver=source(&mut world,$Au8,$Nusize);{call}}}")
    } else {
        call
    };
    let mut tail = "tick(&mut world,if answer{200u8}else{201u8});".to_string();
    if !owned {
        tail.push_str("for _ in 0..2usize{match iterator.next(){Some(value)=>{tick(&mut world,value.b);drop(value);},None=>tick(&mut world,202)}}drop(iterator);");
    }
    tail.push_str("tick(&mut world,83);");
    let helpers = format!(
        "{}fn receiver<'a>(w:&mut World,v:&'a mut Source)->&'a mut Source{{tick(w,88);v}}",
        iterator_helpers()
    );
    paired(
        p,
        &format!("iterator-{method}-{index}"),
        &helpers,
        &helpers,
        &format!("{prefix}let answer={source_call};{tail}"),
        &format!("{prefix}let answer={expanded};{tail}"),
    )
}
pub fn closure(random: &mut Random, index: usize) -> Case {
    let p = Parameters::new(random);
    let(name,helpers,expanded_helpers,body,expanded)=match index%18{
        0=>("partial-capture-cleanup","","struct Capture<'a>{owned:M,counter:&'a mut u8}fn invoke(c:&mut Capture<'_>){see(&c.owned);*c.counter+=1;}","let mut count=0u8;let owned=make(&mut world,$Au8);let counter=&mut count;let mut f=move||{see(&owned);*counter+=1;};for _ in 0..$Nusize{f();}tick(&mut world,count+48);","let mut count=0u8;let owned=make(&mut world,$Au8);let counter=&mut count;let mut f=Capture{owned,counter};for _ in 0..$Nusize{invoke(&mut f);}drop(f.counter);tick(&mut world,count+48);"),
        1=>("fnonce-forwards-fn-drop","fn invoke<F:FnOnce()->u8>(f:F,w:&mut World)->u8{let answer=f();tick(w,$Mu8);answer}","struct Capture{owned:M}fn shared(c:&Capture)->u8{see(&c.owned)}fn once(c:Capture)->u8{let answer=shared(&c);drop(c);answer}fn invoke(c:Capture,w:&mut World)->u8{let answer=once(c);tick(w,$Mu8);answer}","let owned=make(&mut world,$Au8);let answer=invoke(move||see(&owned),&mut world);tick(&mut world,answer);","let owned=make(&mut world,$Au8);let answer=invoke(Capture{owned},&mut world);tick(&mut world,answer);"),
        2=>("nested-shadowed-captures","","struct Outer<'a,'b>{first:&'a u8,second:&'b u8}struct Inner<'a,'b>{first:&'a u8,second:&'b u8}fn inner(c:&Inner<'_,'_>)->u8{*c.first+*c.second}fn outer(c:&Outer<'_,'_>)->u8{let first=*c.first;let nested=Inner{first:&first,second:c.second};inner(&nested)}","let first=$Au8;let second=$Bu8;let f=||{let first=first;let g=||first+second;g()};tick(&mut world,f());","let first=$Au8;let second=$Bu8;let f=Outer{first:&first,second:&second};tick(&mut world,outer(&f));"),
        3=>("contextual-parameter-pattern","fn invoke<F:Fn((u8,u8))->u8>(f:F,pair:(u8,u8))->u8{f(pair)}","struct Capture{}fn invoke(_c:&Capture,pair:(u8,u8))->u8{let left=pair.0;let right=pair.1;left+right}","tick(&mut world,invoke(|(left,right)|left+right,($Au8,$Bu8)));","tick(&mut world,invoke(&Capture{},($Au8,$Bu8)));"),
        4=>("fnonce-forwards-fnmut-drop","fn invoke<F:FnOnce()>(f:F,w:&mut World){f();tick(w,$Mu8);}","struct Capture<'a>{owned:M,counter:&'a mut u8}fn mutable(c:&mut Capture<'_>){see(&c.owned);*c.counter+=1;}fn invoke(mut c:Capture<'_>,w:&mut World){mutable(&mut c);drop(c);tick(w,$Mu8);}","let mut count=0u8;let owned=make(&mut world,$Au8);let counter=&mut count;invoke(move||{see(&owned);*counter+=1;},&mut world);tick(&mut world,count+48);","let mut count=0u8;let owned=make(&mut world,$Au8);let counter=&mut count;invoke(Capture{owned,counter},&mut world);tick(&mut world,count+48);"),
        5=>("capture-coalescing-order","fn see_pair(p:&P)->u8{p.a.b+p.b.b}","fn see_pair(p:&P)->u8{p.a.b+p.b.b}struct Capture{pair:P,middle:M}fn invoke(c:&Capture)->u8{see(&c.pair.a);see(&c.middle);see_pair(&c.pair)}","let pair=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let middle=make(&mut world,$Cu8);let f=move||{see(&pair.a);see(&middle);see_pair(&pair)};tick(&mut world,f());drop(f);","let pair=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let middle=make(&mut world,$Cu8);let f=Capture{pair,middle};tick(&mut world,invoke(&f));drop(f);"),
        6=>("assignment-capture-order","","struct Capture{first:M,second:M}","let mut first=make(&mut world,$Au8);let second=make(&mut world,$Bu8);let f=move||{first=second;};drop(f);","let first=make(&mut world,$Au8);let second=make(&mut world,$Bu8);let f=Capture{first,second};drop(f);"),
        7=>("index-capture-order","","struct Reader{tag:M}struct Capture{values:Vec<M>,reader:Reader}","let values=vec![make(&mut world,$Au8)];let tag=make(&mut world,$Bu8);let reader=move||{see(&tag);0usize};let lookup=move||values[reader()].b;drop(lookup);","let values=vec![make(&mut world,$Au8)];let tag=make(&mut world,$Bu8);let reader=Reader{tag};let lookup=Capture{values,reader};drop(lookup);"),
        8=>("interleaved-field-capture-order","","struct Capture{a:M,b:M,middle:M}","let pair=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let middle=make(&mut world,$Cu8);let f=move||{see(&pair.b);see(&middle);see(&pair.a)};drop(f);","let pair=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let middle=make(&mut world,$Cu8);let f=Capture{a:pair.a,b:pair.b,middle};drop(f);"),
        9=>("generic-result-origin","fn apply<A,B,F:FnOnce(A)->B>(f:F,v:A)->B{f(v)}","struct Capture<'a>{value:&'a u8}fn invoke<'a,'b>(c:Capture<'a>,_argument:&'b u8)->&'a u8{c.value}","let first=$Au8;let second=$Bu8;let captured=&first;let answer=apply(move|_argument:&u8|captured,&second);tick(&mut world,*answer);","let first=$Au8;let second=$Bu8;let answer=invoke(Capture{value:&first},&second);tick(&mut world,*answer);"),
        10=>("generic-unrelated-short-capture","fn apply<A,B,F:FnOnce(A)->B>(f:F,v:A)->B{f(v)}","struct Capture<'a>{value:&'a u8}fn invoke<'a,'b>(c:Capture<'b>,argument:&'a u8)->&'a u8{let _=*c.value;argument}","let first=$Au8;let answer:&u8;{let short=$Bu8;let captured=&short;answer=apply(move|argument:&u8|{let _=*captured;argument},&first);}tick(&mut world,*answer);","let first=$Au8;let answer:&u8;{let short=$Bu8;answer=invoke(Capture{value:&short},&first);}tick(&mut world,*answer);"),
        11=>("generic-mutable-leaf-permutation","fn apply<A,B,F:FnOnce(A)->B>(f:F,v:A)->B{f(v)}","fn invoke<'a,'b>(p:(&'a mut u8,&'b mut u8))->(&'b mut u8,&'a mut u8){(p.1,p.0)}","let mut first=$Au8;let mut second=$Bu8;let answer=apply(|pair:(&mut u8,&mut u8)|(pair.1,pair.0),(&mut first,&mut second));*answer.0+=1;*answer.1+=2;tick(&mut world,*answer.0);tick(&mut world,*answer.1);","let mut first=$Au8;let mut second=$Bu8;let answer=invoke((&mut first,&mut second));*answer.0+=1;*answer.1+=2;tick(&mut world,*answer.0);tick(&mut world,*answer.1);"),
        12=>("mutable-capture-result","","struct Capture<'a>{value:&'a mut u8}fn invoke<'a>(c:Capture<'a>)->&'a mut u8{c.value}","let mut first=$Au8;let captured=&mut first;let f=move||captured;let answer=f();*answer+=1;tick(&mut world,*answer);drop(answer);tick(&mut world,first);","let mut first=$Au8;let answer=invoke(Capture{value:&mut first});*answer+=1;tick(&mut world,*answer);drop(answer);tick(&mut world,first);"),
        13=>("optional-capture-choice","fn choose<T,F:FnOnce()->T>(v:Option<T>,f:F)->T{match v{Some(v)=>v,None=>f()}}","struct Capture<'a>{value:&'a mut u8}fn choose<'a>(v:Option<&'a mut u8>,c:Capture<'a>)->&'a mut u8{match v{Some(v)=>v,None=>c.value}}","let mut first=$Au8;let mut second=$Bu8;let value=if $Y{Some(&mut first)}else{None};let captured=&mut second;let answer=choose(value,move||captured);*answer+=1;tick(&mut world,*answer);drop(answer);tick(&mut world,first);tick(&mut world,second);","let mut first=$Au8;let mut second=$Bu8;let value=if $Y{Some(&mut first)}else{None};let answer=choose(value,Capture{value:&mut second});*answer+=1;tick(&mut world,*answer);drop(answer);tick(&mut world,first);tick(&mut world,second);"),
        14=>("inferred-optional-mutable-captures","","struct Capture<'a>{choice:usize,a:&'a mut u8,b:&'a mut u8}fn invoke<'a>(c:Capture<'a>)->Option<&'a mut u8>{match c.choice{0=>None,1=>Some(c.a),_=>Some(c.b)}}","let mut first=$Au8;let mut second=$Bu8;let choice=$Nusize;let a=&mut first;let b=&mut second;let f=move||match choice{0=>None,1=>Some(a),_=>Some(b)};let result=f();match result{Some(v)=>{*v+=1;tick(&mut world,*v);},None=>tick(&mut world,0)}tick(&mut world,first);tick(&mut world,second);","let mut first=$Au8;let mut second=$Bu8;let result=invoke(Capture{choice:$Nusize,a:&mut first,b:&mut second});match result{Some(v)=>{*v+=1;tick(&mut world,*v);},None=>tick(&mut world,0)}tick(&mut world,first);tick(&mut world,second);"),
        15=>("inferred-break-mutable-captures","","struct Capture<'a>{choice:bool,a:&'a mut u8,b:&'a mut u8}fn invoke<'a>(c:Capture<'a>)->&'a mut u8{if c.choice{c.a}else{c.b}}","let mut first=$Au8;let mut second=$Bu8;let choice=$Y;let a=&mut first;let b=&mut second;let f=move||loop{if choice{break a;}break b;};let answer=f();*answer+=1;tick(&mut world,*answer);drop(answer);tick(&mut world,first);tick(&mut world,second);","let mut first=$Au8;let mut second=$Bu8;let answer=invoke(Capture{choice:$Y,a:&mut first,b:&mut second});*answer+=1;tick(&mut world,*answer);drop(answer);tick(&mut world,first);tick(&mut world,second);"),
        16=>("mutable-two-roots","","struct Capture<'a,'b>{a:&'a mut u8,b:&'b mut u8}fn invoke(c:&mut Capture<'_,'_>)->u8{*c.a+=1;*c.b+=2;*c.a+*c.b}","let mut first=$Au8;let mut second=$Bu8;let mut f=||{first+=1;second+=2;first+second};for _ in 0..$Nusize{tick(&mut world,f());}drop(f);tick(&mut world,first);tick(&mut world,second);","let mut first=$Au8;let mut second=$Bu8;let mut f=Capture{a:&mut first,b:&mut second};for _ in 0..$Nusize{tick(&mut world,invoke(&mut f));}drop(f.a);drop(f.b);tick(&mut world,first);tick(&mut world,second);"),
        _=>("owned-fnonce-environment-drop","fn apply<F:FnOnce()->u8>(f:F)->u8{f()}","struct Capture{a:M,b:M}fn apply(c:Capture)->u8{let a=c.a;let b=c.b;a.b+b.b}","let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);let answer=apply(move||{let a=a;let b=b;a.b+b.b});tick(&mut world,answer);","let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);let answer=apply(Capture{a,b});tick(&mut world,answer);"),
    };
    paired(&p, name, helpers, expanded_helpers, body, expanded)
}
