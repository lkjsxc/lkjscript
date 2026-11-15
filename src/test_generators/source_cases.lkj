// Pure generators. Every paired case has a separately written expansion.
use super::Random;

pub struct Case {
    pub family: String,
    pub source: String,
    pub reference: Option<String>,
    pub expectation: String,
}

pub struct Parameters {
    a: usize,
    b: usize,
    c: usize,
    marker: usize,
    yes: bool,
    length: usize,
}
impl Parameters {
    pub fn new(random: &mut Random) -> Self {
        Self {
            a: 1 + random.pick(50),
            b: 1 + random.pick(50),
            c: 1 + random.pick(50),
            marker: 60 + random.pick(25),
            yes: random.pick(2) == 0,
            length: random.pick(5),
        }
    }
    pub fn render(&self, template: &str) -> String {
        let mut text = String::new();
        let mut at = 0usize;
        let bytes = template.as_bytes();
        while at < bytes.len() {
            if bytes[at] == b'$' && at + 1 < bytes.len() {
                let replacement = match bytes[at + 1] {
                    b'A' => format!("{}", self.a),
                    b'B' => format!("{}", self.b),
                    b'C' => format!("{}", self.c),
                    b'M' => format!("{}", self.marker),
                    b'Y' => format!("{}", self.yes),
                    b'N' => format!("{}", self.length),
                    _ => String::new(),
                };
                if !replacement.is_empty() {
                    text.push_str(&replacement);
                    at += 2;
                    continue;
                }
            }
            let start = at;
            at += 1;
            while at < bytes.len() && (bytes[at] & 192) == 128 {
                at += 1;
            }
            text.push_str(&template[start..at]);
        }
        text
    }
}
fn prelude() -> &'static str {
    "struct M { out: Stdout, b: u8 }\nimpl Drop for M { fn drop(&mut self) { self.out.write(&[self.b]); } }\nstruct P { a: M, b: M }\nfn tick(world:&mut World,b:u8){world.stdout().write(&[b]);}\nfn make(world:&mut World,b:u8)->M{tick(world,b+100);M{out:world.stdout(),b}}\nfn see(m:&M)->u8{m.b}\n"
}
pub fn paired(
    parameters: &Parameters,
    name: &str,
    helpers: &str,
    expanded_helpers: &str,
    body: &str,
    expanded: &str,
) -> Case {
    let source = parameters.render(&format!(
        "{}{}\npub fn main(mut world:World){{ {} }}\n",
        prelude(),
        helpers,
        body
    ));
    let reference = parameters.render(&format!(
        "{}{}\npub fn main(mut world:World){{ {} }}\n",
        prelude(),
        expanded_helpers,
        expanded
    ));
    Case {
        family: name.to_string(),
        source,
        reference: Some(reference),
        expectation: "compare".to_string(),
    }
}
fn sugars(random: &mut Random, index: usize) -> Case {
    let index = index % 85;
    if index >= 34 {
        return super::extended_cases::sugar(random, index - 34);
    }
    let p = Parameters::new(random);
    let (name, helpers, expanded_helpers, body, expanded) = match index {
        0 => ("if-let", "", "", "if let Some(m)=if $Y{Some(make(&mut world,$Au8))}else{None}{tick(&mut world,$Mu8);drop(m);}else{tick(&mut world,$Bu8);}", "match if $Y{Some(make(&mut world,$Au8))}else{None}{Some(m)=>{tick(&mut world,$Mu8);drop(m);},None=>tick(&mut world,$Bu8)}"),
        1 => ("while-let", "", "", "let mut v=vec![make(&mut world,$Au8),make(&mut world,$Bu8)];while let Some(m)=v.pop(){tick(&mut world,$Mu8);drop(m);}", "let mut v=vec![make(&mut world,$Au8),make(&mut world,$Bu8)];loop{match v.pop(){Some(m)=>{tick(&mut world,$Mu8);drop(m);},None=>break}}"),
        2 => ("let-else", "fn take(world:&mut World)->Option<M>{let Some(m)=(if $Y{Some(make(world,$Au8))}else{None})else{tick(world,$Bu8);return None;};tick(world,$Mu8);Some(m)}", "fn take(world:&mut World)->Option<M>{let m=match if $Y{Some(make(world,$Au8))}else{None}{Some(m)=>m,None=>{tick(world,$Bu8);return None;}};tick(world,$Mu8);Some(m)}", "drop(take(&mut world));", "drop(take(&mut world));"),
        3 => ("result-question", "fn take(world:&mut World)->Result<M,M>{let input=if $Y{Ok(make(world,$Au8))}else{Err(make(world,$Bu8))};let m=input?;tick(world,$Mu8);Ok(m)}", "fn take(world:&mut World)->Result<M,M>{let input=if $Y{Ok(make(world,$Au8))}else{Err(make(world,$Bu8))};let m=match input{Ok(m)=>m,Err(e)=>return Err(e)};tick(world,$Mu8);Ok(m)}", "drop(take(&mut world));", "drop(take(&mut world));"),
        4 => ("option-question", "fn take(world:&mut World)->Option<M>{let input=if $Y{Some(make(world,$Au8))}else{None};let m=input?;tick(world,$Mu8);Some(m)}", "fn take(world:&mut World)->Option<M>{let input=if $Y{Some(make(world,$Au8))}else{None};let m=match input{Some(m)=>m,None=>return None};tick(world,$Mu8);Some(m)}", "drop(take(&mut world));", "drop(take(&mut world));"),
        5 => ("or-guard-order", "", "", "let p=(0i64,0i64);match p{(0,x)|(x,0) if {tick(&mut world,$Mu8);$Y}=>tick(&mut world,$Au8),_=>tick(&mut world,$Bu8)}", "let p=(0i64,0i64);if p.0==0&&{tick(&mut world,$Mu8);$Y}{tick(&mut world,$Au8);}else if p.1==0&&{tick(&mut world,$Mu8);$Y}{tick(&mut world,$Au8);}else{tick(&mut world,$Bu8);}"),
        6 => ("nested-pattern", "", "", "let v=Some(if $Y{Some(make(&mut world,$Au8))}else{None});match v{Some(Some(m))=>{tick(&mut world,$Mu8);drop(m);},_=>tick(&mut world,$Bu8)}", "let v=Some(if $Y{Some(make(&mut world,$Au8))}else{None});match v{Some(inner)=>match inner{Some(m)=>{tick(&mut world,$Mu8);drop(m);},None=>tick(&mut world,$Bu8)},None=>tick(&mut world,$Bu8)}"),
        7 => ("struct-update", "", "", "let p=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let q=P{b:make(&mut world,$Cu8),..p};tick(&mut world,$Mu8);drop(q);", "let p=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let next=make(&mut world,$Cu8);let q=P{a:p.a,b:next};tick(&mut world,$Mu8);drop(q);"),
        8 => ("break-value", "", "", "let answer=loop{if $Y{break $Au8;}break $Bu8;};tick(&mut world,answer);", "let answer:u8;loop{if $Y{answer=$Au8;break;}answer=$Bu8;break;}tick(&mut world,answer);"),
        9 => ("reference-pattern", "", "", "let value=($Au8,$Bu8,$Cu8);let(first,..,last)=&value;tick(&mut world,*first);tick(&mut world,*last);", "let value=($Au8,$Bu8,$Cu8);let first=&value.0;let last=&value.2;tick(&mut world,*first);tick(&mut world,*last);"),
        10 => ("named-binding", "", "", "let p=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let P{a,..}=p;tick(&mut world,$Mu8);drop(a);", "let p=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let a=p.a;tick(&mut world,$Mu8);drop(a);"),
        11 => ("guard-keeps-owner", "", "", "let v=Some(make(&mut world,$Au8));match v{Some(m) if {tick(&mut world,$Mu8);$Y}=>{tick(&mut world,$Cu8);drop(m);},Some(m)=>drop(m),None=>()}", "let v=Some(make(&mut world,$Au8));tick(&mut world,$Mu8);if $Y{match v{Some(m)=>{tick(&mut world,$Cu8);drop(m);},None=>()}}else{match v{Some(m)=>drop(m),None=>()}}"),
        12 => ("repeated-vector", "", "", "let values=vec![$Au8;$Nusize];for item in values{tick(&mut world,item);}", "let value=$Au8;let size=$Nusize;let mut values=Vec::new();let mut index=0usize;while index<size{values.push(value);index+=1;}for item in values{tick(&mut world,item);}"),
        13 => ("collection-for-cleanup", "", "", "let values=vec![make(&mut world,$Au8),make(&mut world,$Bu8)];for item in &values{let local=make(&mut world,$Cu8);tick(&mut world,item.b);if $Y{continue;}drop(local);}", "let values=vec![make(&mut world,$Au8),make(&mut world,$Bu8)];let mut iterator=values.iter();loop{match iterator.next(){Some(item)=>{let local=make(&mut world,$Cu8);tick(&mut world,item.b);if $Y{continue;}drop(local);},None=>break}}"),
        14 => ("mutable-range", "", "", "let mut values=vec![$Au8,$Bu8,$Cu8];{let range=&mut values[1..];range[0]=$Mu8;}for item in values{tick(&mut world,item);}", "let mut values=vec![$Au8,$Bu8,$Cu8];values[1]=$Mu8;for item in values{tick(&mut world,item);}"),
        15 => ("text-range", "", "", "let text=\"aλ🦀z\";world.stdout().write(text[1..7].as_bytes());", "world.stdout().write(\"λ🦀\".as_bytes());"),
        16 => ("vec-get", "", "", "let values=vec![make(&mut world,$Au8),make(&mut world,$Bu8)];let item=values.get($Nusize);match item{Some(m)=>tick(&mut world,m.b),None=>tick(&mut world,$Mu8)}", "let values=vec![make(&mut world,$Au8),make(&mut world,$Bu8)];let index=$Nusize;let item=if index<values.len(){Some(&values[index])}else{None};match item{Some(m)=>tick(&mut world,m.b),None=>tick(&mut world,$Mu8)}"),
        17 => ("vec-first", "", "", "let values=vec![make(&mut world,$Au8),make(&mut world,$Bu8)];match values.first(){Some(m)=>tick(&mut world,m.b),None=>tick(&mut world,$Mu8)}", "let values=vec![make(&mut world,$Au8),make(&mut world,$Bu8)];let item=if values.len()==0{None}else{Some(&values[0])};match item{Some(m)=>tick(&mut world,m.b),None=>tick(&mut world,$Mu8)}"),
        18 => ("option-and", "", "", "let left=if $Y{Some(make(&mut world,$Au8))}else{None};let right=Some(make(&mut world,$Bu8));drop(left.and(right));", "let left=if $Y{Some(make(&mut world,$Au8))}else{None};let right=Some(make(&mut world,$Bu8));let value=match left{Some(_)=>right,None=>None};drop(value);"),
        19 => ("option-or", "", "", "let left=if $Y{Some(make(&mut world,$Au8))}else{None};let right=Some(make(&mut world,$Bu8));drop(left.or(right));", "let left=if $Y{Some(make(&mut world,$Au8))}else{None};let right=Some(make(&mut world,$Bu8));let value=match left{Some(value)=>{drop(right);Some(value)},None=>right};drop(value);"),
        20 => ("option-zip", "", "", "let left=if $Y{Some(make(&mut world,$Au8))}else{None};let right=Some(make(&mut world,$Bu8));drop(left.zip(right));", "let left=if $Y{Some(make(&mut world,$Au8))}else{None};let right=Some(make(&mut world,$Bu8));let value=match(left,right){(Some(x),Some(y))=>Some((x,y)),_=>None};drop(value);"),
        21 => ("option-transpose", "", "", "let input:Option<Result<M,M>>=Some(if $Y{Ok(make(&mut world,$Au8))}else{Err(make(&mut world,$Bu8))});drop(input.transpose());", "let input:Option<Result<M,M>>=Some(if $Y{Ok(make(&mut world,$Au8))}else{Err(make(&mut world,$Bu8))});let output=match input{Some(Ok(x))=>Ok(Some(x)),Some(Err(e))=>Err(e),None=>Ok(None)};drop(output);"),
        22 => ("option-as-mut", "", "", "let mut input=Some(make(&mut world,$Au8));match input.as_mut(){Some(x)=>x.b=$Bu8,None=>()}drop(input);", "let mut input=Some(make(&mut world,$Au8));match &mut input{Some(x)=>x.b=$Bu8,None=>()}drop(input);"),
        23 => ("iterator-map", "", "", "let input=vec![$Au8,$Bu8,$Cu8];for item in input.into_iter().map(|x|x+1){tick(&mut world,item);}", "let input=vec![$Au8,$Bu8,$Cu8];let mut iterator=input.into_iter();loop{match iterator.next(){Some(item)=>tick(&mut world,item+1),None=>break}}"),
        24 => ("iterator-filter", "", "", "let input=vec![$Au8,$Bu8,$Cu8];for item in input.into_iter().filter(|x|*x%2==0){tick(&mut world,item);}", "let input=vec![$Au8,$Bu8,$Cu8];for item in input{if item%2==0{tick(&mut world,item);}}"),
        25 => ("iterator-filter-map", "", "", "let input=vec![$Au8,$Bu8,$Cu8];for item in input.into_iter().filter_map(|x|if x%2==0{Some(x+1)}else{None}){tick(&mut world,item);}", "let input=vec![$Au8,$Bu8,$Cu8];for item in input{let next=if item%2==0{Some(item+1)}else{None};match next{Some(value)=>tick(&mut world,value),None=>()}}"),
        26 => ("iterator-any", "", "", "let values=vec![$Au8,$Bu8,$Cu8];let mut iter=values.into_iter();let answer=iter.any(|x|x%2==0);tick(&mut world,answer as u8);for item in iter{tick(&mut world,item);}", "let values=vec![$Au8,$Bu8,$Cu8];let mut iter=values.into_iter();let mut answer=false;loop{match iter.next(){Some(x)=>if x%2==0{answer=true;break;},None=>break}}tick(&mut world,answer as u8);for item in iter{tick(&mut world,item);}"),
        27 => ("iterator-all", "", "", "let values=vec![$Au8,$Bu8,$Cu8];let mut iter=values.into_iter();let answer=iter.all(|x|x%2==0);tick(&mut world,answer as u8);for item in iter{tick(&mut world,item);}", "let values=vec![$Au8,$Bu8,$Cu8];let mut iter=values.into_iter();let mut answer=true;loop{match iter.next(){Some(x)=>if x%2!=0{answer=false;break;},None=>break}}tick(&mut world,answer as u8);for item in iter{tick(&mut world,item);}"),
        28 => ("iterator-enumerate", "", "", "let values=vec![$Au8,$Bu8,$Cu8];for(index,item)in values.into_iter().enumerate(){tick(&mut world,index as u8);tick(&mut world,item);}", "let values=vec![$Au8,$Bu8,$Cu8];let mut index=0usize;for item in values{tick(&mut world,index as u8);tick(&mut world,item);index+=1;}"),
        29 => ("iterator-skip-take", "", "", "let values=vec![$Au8,$Bu8,$Cu8];for item in values.into_iter().skip(1).take($Nusize){tick(&mut world,item);}", "let values=vec![$Au8,$Bu8,$Cu8];let mut iter=values.into_iter();drop(iter.next());let mut left=$Nusize;while left>0{left-=1;match iter.next(){Some(item)=>tick(&mut world,item),None=>break}}"),
        30 => ("vec-extend-clone", "", "", "let mut values=vec![$Au8];let input=[$Bu8,$Cu8];values.extend_from_slice(&input);for item in values{tick(&mut world,item);}", "let mut values=vec![$Au8];let input=[$Bu8,$Cu8];let mut index=0usize;while index<input.len(){values.push(input[index]);index+=1;}for item in values{tick(&mut world,item);}"),
        31 => ("format-append", "", "", "let a=$Ai64;let b=$Bi64;let text=format!(\"{a}:{b}\");world.stdout().write(text.as_bytes());", "let a=$Ai64;let b=$Bi64;let mut text=format!(\"{a}\");text.push(':');text.push_str(&format!(\"{b}\"));world.stdout().write(text.as_bytes());"),
        32 => ("unicode-encoding", "", "", "let mut text=String::new();text.push('λ');text.push('🦀');world.stdout().write(text.as_bytes());", "world.stdout().write(&[206u8,187u8,240u8,159u8,166u8,128u8]);"),
        _ => ("option-map", "", "", "let input=if $Y{Some(make(&mut world,$Au8))}else{None};let output=input.map(|mut x|{x.b=$Bu8;x});drop(output);", "let input=if $Y{Some(make(&mut world,$Au8))}else{None};let output=match input{Some(mut x)=>{x.b=$Bu8;Some(x)},None=>None};drop(output);"),
    };
    paired(&p, name, helpers, expanded_helpers, body, expanded)
}
fn closures(random: &mut Random, index: usize) -> Case {
    let index = index % 28;
    if index >= 10 {
        return super::extended_cases::closure(random, index - 10);
    }
    let p = Parameters::new(random);
    let (name, helpers, expanded_helpers, body, expanded) = match index % 10 {
        0 => ("shared-two-roots", "fn apply<F:Fn(u8)->u8>(f:F,x:u8)->u8{f(x)}", "struct Capture<'a,'b>{a:&'a u8,b:&'b u8} fn apply(c:&Capture<'_,'_>,x:u8)->u8{x+*c.a+*c.b}", "let a=$Au8;let b=$Bu8;let f=|x|x+a+b;let result=apply(f,1);tick(&mut world,result);", "let a=$Au8;let b=$Bu8;let f=Capture{a:&a,b:&b};let result=apply(&f,1);tick(&mut world,result);"),
        1 => ("mutable-capture", "", "struct Capture<'a>{a:&'a mut u8}fn apply(c:&mut Capture<'_>)->u8{*c.a+=1;*c.a}", "let mut a=$Au8;let mut f=||{a+=1;a};let x=f();tick(&mut world,x);let x=f();tick(&mut world,x);drop(f);tick(&mut world,a);", "let mut a=$Au8;let mut f=Capture{a:&mut a};let x=apply(&mut f);tick(&mut world,x);let x=apply(&mut f);tick(&mut world,x);drop(f.a);tick(&mut world,a);"),
        2 => ("owned-once", "fn apply<F:FnOnce()->u8>(f:F)->u8{f()}", "struct Capture{m:M}fn apply(c:Capture)->u8{let m=c.m;m.b}", "let m=make(&mut world,$Au8);let x=apply(move||{let m=m;m.b});tick(&mut world,x);", "let m=make(&mut world,$Au8);let x=apply(Capture{m});tick(&mut world,x);"),
        3 => ("capture-order", "", "struct Capture{b:M,a:M}fn apply(c:&Capture)->u8{c.b.b+c.a.b}", "let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);let f=move||b.b+a.b;tick(&mut world,f());drop(f);", "let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);let f=Capture{b,a};tick(&mut world,apply(&f));drop(f);"),
        4 => ("disjoint-owned-field", "", "struct Capture{b:M}fn apply(c:&Capture)->u8{c.b.b}", "let p=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let f=move||p.b.b;drop(p.a);tick(&mut world,f());drop(f);", "let p=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};let f=Capture{b:p.b};drop(p.a);tick(&mut world,apply(&f));drop(f);"),
        5 => ("generic-once", "fn apply<F:FnOnce(M)->M>(f:F,m:M)->M{f(m)}", "fn apply(m:M)->M{m}", "let owned=make(&mut world,$Au8);let result=apply(|m|m,owned);drop(result);", "let owned=make(&mut world,$Au8);let result=apply(owned);drop(result);"),
        6 => ("same-type-factory-shared-origin", "fn choose<T,F:FnOnce()->T>(value:Option<T>,make:F)->T{match value{Some(value)=>value,None=>make()}}", "struct Capture<'a>{value:&'a u8}fn choose<'a>(value:Option<&'a u8>,capture:Capture<'a>)->&'a u8{match value{Some(value)=>value,None=>capture.value}}", "let first=$Au8;let second=$Bu8;let value:Option<&u8>=if $Y{Some(&first)}else{None};let captured=&second;let answer=choose(value,move||captured);tick(&mut world,*answer);", "let first=$Au8;let second=$Bu8;let value:Option<&u8>=if $Y{Some(&first)}else{None};let answer=choose(value,Capture{value:&second});tick(&mut world,*answer);"),
        7 => ("closure-loop", "", "struct Capture{base:u8}fn apply(c:&Capture,n:u8)->u8{c.base+n}", "let base=$Au8;let f=move|n:u8|base+n;for n in 0..$Nu8{tick(&mut world,f(n));}", "let base=$Au8;let f=Capture{base};for n in 0..$Nu8{tick(&mut world,apply(&f,n));}"),
        8 => ("capture-no-call-drop", "", "struct Capture{m:M}", "let m=make(&mut world,$Au8);let f=move||{let moved=m;moved.b};tick(&mut world,$Mu8);drop(f);", "let m=make(&mut world,$Au8);let f=Capture{m};tick(&mut world,$Mu8);drop(f);"),
        _ => ("borrowed-capture-call", "", "fn apply(m:&M)->u8{m.b}", "let m=make(&mut world,$Au8);let f=||m.b;tick(&mut world,f());drop(f);drop(m);", "let m=make(&mut world,$Au8);tick(&mut world,apply(&m));drop(m);"),
    };
    paired(&p, name, helpers, expanded_helpers, body, expanded)
}
fn drops(random: &mut Random, index: usize) -> Case {
    let p = Parameters::new(random);
    let (name, body, expanded) = match index % 8 {
        0 => ("conditional-owner", "let value=make(&mut world,$Au8);if $Y{drop(value);}tick(&mut world,$Mu8);", "if $Y{let value=make(&mut world,$Au8);drop(value);tick(&mut world,$Mu8);}else{let value=make(&mut world,$Au8);tick(&mut world,$Mu8);}"),
        1 => ("partial-fields", "let pair=P{a:make(&mut world,$Au8),b:make(&mut world,$Bu8)};drop(pair.a);tick(&mut world,$Mu8);", "let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);drop(a);tick(&mut world,$Mu8);drop(b);"),
        2 => ("replace-drop", "let mut a=make(&mut world,$Au8);a=make(&mut world,$Bu8);tick(&mut world,$Mu8);", "let old=make(&mut world,$Au8);let next=make(&mut world,$Bu8);drop(old);tick(&mut world,$Mu8);drop(next);"),
        3 => ("tuple-reverse-locals", "let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);tick(&mut world,$Mu8);", "let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);tick(&mut world,$Mu8);drop(b);drop(a);"),
        4 => ("tuple-forward-fields", "let pair=(make(&mut world,$Au8),make(&mut world,$Bu8));tick(&mut world,$Mu8);drop(pair);", "let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);tick(&mut world,$Mu8);drop(a);drop(b);"),
        5 => ("loop-cleanup", "for _ in 0..$Nusize{let value=make(&mut world,$Au8);if $Y{continue;}tick(&mut world,$Mu8);}", "for _ in 0..$Nusize{let value=make(&mut world,$Au8);if !$Y{tick(&mut world,$Mu8);}drop(value);}"),
        6 => ("option-cleanup", "let value=if $Y{Some(make(&mut world,$Au8))}else{None};tick(&mut world,$Mu8);drop(value);", "if $Y{let value=make(&mut world,$Au8);tick(&mut world,$Mu8);drop(value);}else{tick(&mut world,$Mu8);}"),
        _ => ("mem-replace", "let mut a=make(&mut world,$Au8);let b=std::mem::replace(&mut a,make(&mut world,$Bu8));tick(&mut world,$Mu8);drop(b);drop(a);", "let a=make(&mut world,$Au8);let b=make(&mut world,$Bu8);tick(&mut world,$Mu8);drop(a);drop(b);"),
    };
    paired(&p, name, "", "", body, expanded)
}
fn output(random: &mut Random, index: usize) -> Case {
    let mut bytes = Vec::new();
    for _ in 0..(1 + random.pick(40)) {
        bytes.push(random.pick(256) as u8);
    }
    let mut items = String::new();
    let mut reference = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        if index != 0 {
            items.push(',');
        }
        items.push_str(&format!("{byte}u8"));
        reference.push_str(&format!("world.stdout().write(&[{byte}u8]);"));
    }
    let body = if index % 3 == 0 {
        format!("let bytes=vec![{items}];let mut at=0usize;while at<bytes.len(){{let end=(at+3).min(bytes.len());world.stdout().write(&bytes[at..end]);at=end;}}")
    } else {
        format!("world.stdout().write(&[{items}]);")
    };
    let p = Parameters::new(random);
    paired(&p, "binary-output", "", "", &body, &reference)
}
pub fn generate(family: &str, random: &mut Random, index: usize) -> Result<Case, String> {
    match family {
        "values" => Ok(super::values::generate(random)),
        "ownership" => Ok(super::ownership::generate(random, index)),
        "sugar" => Ok(sugars(random, index)),
        "closures" => Ok(closures(random, index)),
        "drops" => Ok(drops(random, index)),
        "output" => Ok(output(random, index)),
        _ => Err(format!("E_USAGE unknown generator family {family}")),
    }
}
