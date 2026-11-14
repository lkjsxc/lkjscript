// Source-size and width workloads. COUNT is the size; SEED is unused.
pub fn generate(kind: &str, n: usize) -> Result<String, String> {
    if n == 0 {
        return Err("E_USAGE scale size must be positive".to_string());
    }
    let mut source = String::new();
    match kind{
        "funcs"=>{
            for i in 0..n{
                let body=if i==0{"x".to_string()}else{format!("f{}(x) + 1",i-1)};
                source.push_str(&format!("fn f{i}(x:i64)->i64{{{body}}}\n"));
            }
            source.push_str(&format!("pub fn probe()->i64{{f{}(0)}}\n",n-1));
        }
        "methods"=>{
            source.push_str("struct Owner{value:i64}\n");
            for i in 0..n{source.push_str(&format!("impl Owner{{fn method{i}()->i64{{{i}}}}}\n"));}
            source.push_str("pub fn probe()->i64{42}\n");
        }
        "ifs"|"boxes"|"vecs"=>{
            source.push_str("pub fn probe()->i64{let mut s=0i64;\n");
            for i in 0..n{
                let item=if kind=="ifs"{format!("let a{i}={i}i64;if a{i}%2==0{{s+=1;}}\n")}else if kind=="boxes"{format!("let b{i}=Box::new({i}i64);let r{i}=&*b{i};s+=*r{i};\n")}else{format!("let mut v{i}:Vec<i64>=Vec::new();v{i}.push({i});s+=v{i}[0];\n")};
                source.push_str(&item);
            }
            source.push_str("s}\n");
        }
        "structs"=>{
            for i in 0..n{
                let field=if i==0{"i64".to_string()}else{format!("S{}",i-1)};
                source.push_str(&format!("struct S{i}{{a:i64,b:{field}}}\n"));
            }
            source.push_str("pub fn probe()->i64{1}\n");
        }
        "expr"=>{
            source.push_str("pub fn probe()->i64{");
            for _ in 0..n{source.push('(');}
            source.push('1');
            for _ in 0..n{source.push_str("+1)");}
            source.push_str("}\n");
        }
        "match"=>{
            source.push_str("pub fn probe()->i64{let x=7i64;match x{\n");
            for i in 0..n{source.push_str(&format!("{i}=>{},\n",i+1));}
            source.push_str("_=>0}}\n");
        }
        "allocvec"=>source=format!("pub fn probe()->i64{{let mut v:Vec<i64>=Vec::new();let mut i=0usize;while i<{n}usize{{v.push(i as i64);i+=1;}}v[v.len()-1]}}\n"),
        "alloclist"=>source=format!("enum List{{Nil,Cons(i64,Box<List>)}}pub fn probe()->i64{{let mut v=List::Nil;let mut i=0i64;while i<{n}{{v=List::Cons(i,Box::new(v));i+=1;}}match v{{List::Cons(x,rest)=>x,List::Nil=>0}}}}\n"),
        "traits"=>{
            source.push_str("trait Val{fn val(&self)->i64;}fn get<T:Val>(x:&T)->i64{x.val()}\n");
            for i in 0..n{source.push_str(&format!("struct S{i}{{v:i64}}impl Val for S{i}{{fn val(&self)->i64{{self.v}}}}\n"));}
            source.push_str("pub fn probe()->i64{let mut t=0i64;\n");
            for i in 0..n{source.push_str(&format!("{{let s=S{i}{{v:1}};t+=get(&s);}}\n"));}
            source.push_str("t}\n");
        }
        "fields"=>{
            source.push_str("struct S{");
            for i in 0..n{source.push_str(&format!("f{i}:i64,"));}
            source.push_str("}\npub fn probe()->i64{let s=S{");
            for i in 0..n{source.push_str(&format!("f{i}:1,"));}
            source.push_str("};let mut t=0i64;\n");
            for i in 0..n{source.push_str(&format!("t+=s.f{i};\n"));}
            source.push_str("t}\n");
        }
        "variants"=>{
            source.push_str("enum E{");
            for i in 0..n{source.push_str(&format!("V{i}(i64),"));}
            source.push_str("}\nfn pick(e:E)->i64{match e{");
            for i in 0..n{source.push_str(&format!("E::V{i}(x)=>x+{i},\n"));}
            source.push_str(&format!("}}}}\npub fn probe()->i64{{pick(E::V{}(1))}}\n",n-1));
        }
        "refs"=>{
            source.push_str("pub fn probe()->i64{let x=1i64;\n");
            for i in 0..n{source.push_str(&format!("let r{i}=&x;\n"));}
            source.push_str("let mut t=0i64;\n");
            for i in 0..n{source.push_str(&format!("t+=*r{i};\n"));}
            source.push_str("t}\n");
        }
        _=>return Err(format!("E_USAGE unknown scale family {kind}")),
    }
    Ok(source)
}
