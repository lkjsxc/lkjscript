// Typed random programs, including stateful statements and calls to earlier functions.
// Every live value contributes to the result checksum; unused intermediates cannot
// accidentally hide a wrong lowering. This generator never consults the compiler.
use super::source_cases::Case;
use super::Random;

#[derive(Clone)]
struct Variable {
    name: String,
    ty: String,
    mutable: bool,
}
#[derive(Clone)]
struct Function {
    name: String,
    parameters: Vec<String>,
    result: String,
}
#[derive(Clone)]
struct Structure {
    name: String,
    fields: Vec<String>,
}
struct Generator {
    uid: usize,
    functions: Vec<Function>,
    structures: Vec<Structure>,
}

fn int_type(random: &mut Random) -> String {
    match random.pick(8) {
        0 => "i64",
        1 => "i32",
        2 => "u8",
        3 => "usize",
        4 => "u64",
        5 => "i8",
        6 => "u32",
        _ => "i16",
    }
    .to_string()
}
fn scalar_type(random: &mut Random) -> String {
    if random.pick(9) == 8 {
        "bool".to_string()
    } else {
        int_type(random)
    }
}
fn of_type(environment: &[Variable], ty: &str) -> Vec<Variable> {
    let mut out: Vec<Variable> = Vec::new();
    for variable in environment {
        if variable.ty == ty {
            out.push(variable.clone());
        }
    }
    out
}
fn literal(random: &mut Random, ty: &str) -> String {
    if ty == "bool" {
        return if random.pick(2) == 0 {
            "true".to_string()
        } else {
            "false".to_string()
        };
    }
    if random.pick(14) != 0 {
        return format!("{}{ty}", random.pick(9));
    }
    let (max, near, half) = match ty {
        "i8" => ("127", "126", "63"),
        "u8" => ("255", "254", "127"),
        "i16" => ("32767", "32766", "16383"),
        "u16" => ("65535", "65534", "32767"),
        "i32" => ("2147483647", "2147483646", "1073741823"),
        "u32" => ("4294967295", "4294967294", "2147483647"),
        "i64" | "isize" => (
            "9223372036854775807",
            "9223372036854775806",
            "4611686018427387903",
        ),
        _ => (
            "18446744073709551615",
            "18446744073709551614",
            "9223372036854775807",
        ),
    };
    let number = match random.pick(5) {
        0 => max,
        1 => near,
        2 => half,
        3 => "100",
        _ => "127",
    };
    format!("{number}{ty}")
}
fn prelude() -> &'static str {
    r#"enum En { A(i64), B(u8,bool), C }
struct Hs { v:Vec<i64>, n:i64 }
fn addto(p:&mut i64,q:&i64){*p=*p+*q;}
fn sumr(w:&Vec<i64>)->i64{let mut s=0i64;let mut i=0usize;while i<w.len(){s+=w[i];i+=1;}s}
fn sumv(w:Vec<i64>)->i64{sumr(&w)}
fn mv(h:Hs)->Hs{h}
fn first(w:&Vec<i64>)->&i64{&w[0]}
fn bump(w:&mut Vec<i64>){let n=w.len();if n>0{w[n-1]+=1;}w.push(n as i64);}
fn unbox(b:Box<i64>)->i64{*b}
fn pickopt(o:Option<Box<i64>>,d:i64)->i64{match o{Some(b)=>*b,None=>d}}
fn inferred_pair()->(i64,u8){(40,2)}
fn replace_mix(p:&mut i64,n:i64){let old=std::mem::replace(p,n);let next=std::mem::replace(p,old);*p=mix(*p,next);}
fn refill<T>(slot:&mut T,first:T,second:T)->T{let old=std::mem::replace(slot,first);drop(old);std::mem::replace(slot,second)}
fn borrow<T>(value:&mut T)->&mut T{value}
struct Slot<'a>{value:&'a mut i64}
fn projected_mix(slot:&mut i64,n:i64){let mut held=Slot{value:slot};replace_mix(held.value,n);*held.value=mix(*held.value,n);}
fn mix(h:i64,x:i64)->i64{((h%1000003)*1009+(x%1000003)+2000006)%1000003}
fn mixv(h:i64,w:&Vec<i64>)->i64{let mut s=h;let mut i=0usize;while i<w.len(){s=mix(s,w[i]);i+=1;}mix(s,w.len() as i64)}
"#
}
impl Generator {
    fn name(&mut self, prefix: &str) -> String {
        let name = format!("{prefix}{}", self.uid);
        self.uid += 1;
        name
    }
    fn expression(
        &mut self,
        random: &mut Random,
        ty: &str,
        environment: &[Variable],
        depth: usize,
    ) -> String {
        let variables = of_type(environment, ty);
        let choice = random.pick(if depth == 0 { 3 } else { 26 });
        if choice == 0 || (depth == 0 && variables.is_empty()) {
            return literal(random, ty);
        }
        if choice <= 2 && !variables.is_empty() {
            return variables[random.pick(variables.len())].name.clone();
        }
        let next = depth - 1;
        if ty == "bool" {
            if choice < 9 {
                let operand = int_type(random);
                let left = self.expression(random, &operand, environment, next);
                let right = self.expression(random, &operand, environment, next);
                let operator = match random.pick(6) {
                    0 => "<",
                    1 => "<=",
                    2 => ">",
                    3 => ">=",
                    4 => "==",
                    _ => "!=",
                };
                return format!("({left}{operator}{right})");
            }
            if choice < 14 {
                let left = self.expression(random, "bool", environment, next);
                let right = self.expression(random, "bool", environment, next);
                let operator = if random.pick(2) == 0 { "&&" } else { "||" };
                return format!("({left}{operator}{right})");
            }
            if choice < 16 {
                return format!("(!{})", self.expression(random, "bool", environment, next));
            }
            let condition = self.expression(random, "bool", environment, next);
            let left = self.expression(random, "bool", environment, next);
            let right = self.expression(random, "bool", environment, next);
            return format!("(if {condition}{{{left}}}else{{{right}}})");
        }
        if choice < 6 {
            let left = self.expression(random, ty, environment, next);
            let right = self.expression(random, ty, environment, next);
            let operation = random.pick(7);
            let operator = if ty.starts_with('i') {
                match operation {
                    0 | 2 => "+",
                    1 => "-",
                    3 => "&",
                    4 => "|",
                    5 => "^",
                    _ => "*",
                }
            } else {
                match operation {
                    0 | 1 => "+",
                    2 => "&",
                    3 => "|",
                    4 => "^",
                    5 => "%",
                    _ => "/",
                }
            };
            return format!("({left}{operator}{right})");
        }
        if choice < 7 {
            let value = self.expression(random, ty, environment, next);
            let operator = if random.pick(2) == 0 { "<<" } else { ">>" };
            return format!("({value}{operator}{}{ty})", random.pick(7));
        }
        if choice < 9 {
            let from = scalar_type(random);
            let value = self.expression(random, &from, environment, next);
            return format!("({value} as {ty})");
        }
        if choice < 10 {
            let condition = self.expression(random, "bool", environment, next);
            let left = self.expression(random, ty, environment, next);
            let right = self.expression(random, ty, environment, next);
            return format!("(if {condition}{{{left}}}else{{{right}}})");
        }
        if choice < 11 {
            let mut functions: Vec<Function> = Vec::new();
            for function in &self.functions {
                if function.result == ty {
                    functions.push(function.clone());
                }
            }
            if !functions.is_empty() {
                let function = functions[random.pick(functions.len())].clone();
                let mut arguments = String::new();
                for (index, parameter) in function.parameters.iter().enumerate() {
                    if index != 0 {
                        arguments.push(',');
                    }
                    arguments.push_str(&self.expression(random, parameter, environment, next));
                }
                return format!("{}({arguments})", function.name);
            }
        }
        if choice < 12 {
            let name = self.name("block");
            let bound = int_type(random);
            let value = self.expression(random, &bound, environment, next);
            let mut inner = environment.to_vec();
            inner.push(Variable {
                name: name.clone(),
                ty: bound.clone(),
                mutable: false,
            });
            let result = self.expression(random, ty, &inner, next);
            return format!("{{let {name}:{bound}={value};{result}}}");
        }
        if choice < 13 {
            let operand = int_type(random);
            let selected = self.expression(random, &operand, environment, next);
            let first = self.expression(random, ty, environment, next);
            let second = self.expression(random, ty, environment, next);
            let otherwise = self.expression(random, ty, environment, next);
            return format!("(match {selected}{{{}{operand}=>{first},{}{operand}..={}{operand}=>{second},_=>{otherwise}}})",random.pick(5),5+random.pick(3),9+random.pick(9));
        }
        if choice < 14 && !self.structures.is_empty() {
            let structure = self.structures[random.pick(self.structures.len())].clone();
            let mut field: Option<usize> = None;
            for (index, kind) in structure.fields.iter().enumerate() {
                if kind == ty {
                    field = Some(index);
                    break;
                }
            }
            if let Some(field) = field {
                let mut fields = String::new();
                for (index, kind) in structure.fields.iter().enumerate() {
                    let value = self.expression(random, kind, environment, next);
                    fields.push_str(&format!("f{index}:{value},"));
                }
                return format!("({}{{{fields}}}).f{field}", structure.name);
            }
        }
        if choice < 15 {
            let name = self.name("values");
            let first = self.expression(random, ty, environment, next);
            let second = self.expression(random, ty, environment, next);
            let index = self.expression(random, "usize", environment, next);
            let annotation = if self.uid % 2 == 0 {
                String::new()
            } else {
                format!(":Vec<{ty}>")
            };
            return format!("{{let mut {name}{annotation}=Vec::new();{name}.push({first});{name}.push({second});{name}[({index})%2usize]}}");
        }
        if choice < 16 {
            let name = self.name("boxed");
            let value = self.expression(random, ty, environment, next);
            let kind = random.pick(3);
            if kind == 1 {
                let first = self.expression(random, ty, environment, next);
                let second = self.expression(random, ty, environment, next);
                return format!("{{let mut {name}:Box<{ty}>=Box::new({value});let old=refill(&mut {name},Box::new({first}),Box::new({second}));*old+*{name}}}");
            }
            if kind == 2 {
                return format!("{{let mut {name}:Box<{ty}>=Box::new({value});let r:&mut {ty}=borrow(&mut {name});*r}}");
            }
            return format!("{{let {name}:Box<{ty}>=Box::new({value});*{name}}}");
        }
        if choice < 17 {
            let name = self.name("tuple");
            let value = self.expression(random, ty, environment, next);
            let condition = self.expression(random, "bool", environment, next);
            let otherwise = self.expression(random, ty, environment, next);
            return format!("{{let {name}:({ty},bool)=({value},{condition});if {name}.1{{{name}.0}}else{{{otherwise}}}}}");
        }
        if choice < 18 {
            let name = self.name("array");
            let a = self.expression(random, ty, environment, next);
            let b = self.expression(random, ty, environment, next);
            let c = self.expression(random, ty, environment, next);
            let index = self.expression(random, "usize", environment, next);
            return format!("{{let {name}:[{ty};3]=[{a},{b},{c}];{name}[({index})%3usize]}}");
        }
        if choice < 19 {
            let name = self.name("option");
            let condition = self.expression(random, "bool", environment, next);
            let value = self.expression(random, ty, environment, next);
            let otherwise = self.expression(random, ty, environment, next);
            return format!("{{let {name}:Option<{ty}>=if {condition}{{Some({value})}}else{{None}};match {name}{{Some(value)=>value,None=>{otherwise}}}}}");
        }
        if choice < 20 {
            let name = self.name("enumeration");
            let variant = match random.pick(3) {
                0 => format!(
                    "En::A({})",
                    self.expression(random, "i64", environment, next)
                ),
                1 => {
                    let byte = self.expression(random, "u8", environment, next);
                    let boolean = self.expression(random, "bool", environment, next);
                    format!("En::B({byte},{boolean})")
                }
                _ => "En::C".to_string(),
            };
            let otherwise = self.expression(random, ty, environment, next);
            let empty = self.expression(random, ty, environment, next);
            return format!("{{let {name}={variant};match {name}{{En::A(value)=>value as {ty},En::B(value,yes)=>if yes{{value as {ty}}}else{{{otherwise}}},En::C=>{empty}}}}}");
        }
        if ty == "i64" {
            let vectors = of_type(environment, "vec");
            if choice < 21 && !vectors.is_empty() {
                let name = vectors[random.pick(vectors.len())].name.clone();
                return match random.pick(4) {
                    0 => format!("sumr(&{name})"),
                    1 => format!("({name}.len() as i64)"),
                    2 => {
                        let index = self.expression(random, "usize", environment, next);
                        let otherwise = self.expression(random, ty, environment, next);
                        format!("(if {name}.len()>0usize{{{name}[({index})%{name}.len()]}}else{{{otherwise}}})")
                    }
                    _ => {
                        let otherwise = self.expression(random, ty, environment, next);
                        format!("(if {name}.len()>0usize{{*first(&{name})}}else{{{otherwise}}})")
                    }
                };
            }
            if choice < 22 {
                let name = self.name("aggregate");
                let first = self.expression(random, ty, environment, next);
                let second = self.expression(random, ty, environment, next);
                return format!("{{let mut {name}:Vec<i64>=Vec::new();{name}.push({first});let g=mv(Hs{{v:{name},n:{second}}});sumv(g.v)+g.n}}");
            }
            if choice < 23 {
                return format!(
                    "unbox(Box::new({}))",
                    self.expression(random, ty, environment, next)
                );
            }
            if choice < 24 {
                let condition = self.expression(random, "bool", environment, next);
                let value = self.expression(random, ty, environment, next);
                let otherwise = self.expression(random, ty, environment, next);
                return format!(
                    "pickopt(if {condition}{{Some(Box::new({value}))}}else{{None}},{otherwise})"
                );
            }
            if choice < 25 {
                let name = self.name("mutable");
                let value = self.expression(random, ty, environment, next);
                let other = self.expression(random, ty, environment, next);
                let operation = match random.pick(3) {
                    0 => "addto",
                    1 => "replace_mix",
                    _ => "projected_mix",
                };
                let argument = if operation == "addto" { "&q" } else { "q" };
                return format!("{{let mut {name}:i64={value};let q:i64={other};{operation}(&mut {name},{argument});{name}}}");
            }
        }
        literal(random, ty)
    }
    fn statements(
        &mut self,
        random: &mut Random,
        outer: &[Variable],
        depth: usize,
        count: usize,
        result: &str,
        in_loop: bool,
    ) -> (String, Vec<Variable>) {
        let mut source = String::new();
        let mut environment = outer.to_vec();
        for _ in 0..count {
            let choice = random.pick(16);
            if choice < 4 {
                let ty = scalar_type(random);
                let name = self.name("value");
                let mutable = random.pick(2) == 0;
                let value = self.expression(random, &ty, &environment, depth);
                source.push_str(&format!(
                    "let {}{name}:{ty}={value};\n",
                    if mutable { "mut " } else { "" }
                ));
                environment.push(Variable { name, ty, mutable });
            } else if choice < 6 {
                let mut mutable: Vec<Variable> = Vec::new();
                for variable in &environment {
                    if variable.mutable {
                        mutable.push(variable.clone());
                    }
                }
                if !mutable.is_empty() {
                    let variable = mutable[random.pick(mutable.len())].clone();
                    let value = self.expression(random, &variable.ty, &environment, depth);
                    let operation = if variable.ty == "bool" || random.pick(2) == 1 {
                        "="
                    } else {
                        match random.pick(3) {
                            0 => "+=",
                            1 => "-=",
                            _ => "*=",
                        }
                    };
                    source.push_str(&format!("{}{operation}{value};\n", variable.name));
                }
            } else if choice < 7 && depth > 0 {
                let condition = self.expression(random, "bool", &environment, depth);
                let (body, _) =
                    self.statements(random, &environment, depth - 1, 2, result, in_loop);
                source.push_str(&format!("if {condition}{{\n{body}}}\n"));
            } else if choice < 8 && depth > 0 {
                let condition = self.expression(random, "bool", &environment, depth);
                let (left, _) =
                    self.statements(random, &environment, depth - 1, 2, result, in_loop);
                let (right, _) =
                    self.statements(random, &environment, depth - 1, 2, result, in_loop);
                source.push_str(&format!("if {condition}{{\n{left}}}else{{\n{right}}}\n"));
            } else if choice < 9 && depth > 0 {
                let name = self.name("loop");
                let length = 1 + random.pick(4);
                let mut inner = environment.clone();
                inner.push(Variable {
                    name: name.clone(),
                    ty: "i64".to_string(),
                    mutable: false,
                });
                let (body, _) = self.statements(random, &inner, depth - 1, 3, result, true);
                source.push_str(&format!(
                    "let mut {name}=0i64;while {name}<{length}{{{name}+=1;\n{body}}}\n"
                ));
            } else if choice < 10 && depth > 0 {
                let name = self.name("index");
                let length = 1 + random.pick(4);
                let mut inner = environment.clone();
                inner.push(Variable {
                    name: name.clone(),
                    ty: "usize".to_string(),
                    mutable: false,
                });
                let (body, _) = self.statements(random, &inner, depth - 1, 3, result, true);
                source.push_str(&format!("for {name} in 0..{length}usize{{\n{body}}}\n"));
            } else if choice < 11 {
                let name = self.name("vector");
                source.push_str(&format!("let mut {name}:Vec<i64>=Vec::new();\n"));
                for _ in 0..random.pick(3) {
                    let value = self.expression(random, "i64", &environment, 1);
                    source.push_str(&format!("{name}.push({value});\n"));
                }
                environment.push(Variable {
                    name,
                    ty: "vec".to_string(),
                    mutable: false,
                });
            } else if choice < 13 {
                let vectors = of_type(&environment, "vec");
                if !vectors.is_empty() {
                    let name = vectors[random.pick(vectors.len())].name.clone();
                    let statement: String = match random.pick(3) {
                        0 => {
                            let value = self.expression(random, "i64", &environment, depth);
                            format!("{name}.push({value});\n")
                        }
                        1 => format!("bump(&mut {name});\n"),
                        _ => {
                            let value = self.expression(random, "i64", &environment, depth);
                            format!("if {name}.len()>0{{let last={name}.len()-1;{name}[last]={value};}}\n")
                        }
                    };
                    source.push_str(&statement);
                }
            } else if choice < 14 {
                let mut mutable: Vec<Variable> = Vec::new();
                let scalars = of_type(&environment, "i64");
                for variable in &scalars {
                    if variable.mutable {
                        mutable.push(variable.clone());
                    }
                }
                if !mutable.is_empty() && !scalars.is_empty() {
                    let p = &mutable[random.pick(mutable.len())].name;
                    let q = &scalars[random.pick(scalars.len())].name;
                    if p != q {
                        source.push_str(&format!("addto(&mut {p},&{q});\n"));
                    }
                }
            } else if choice < 15 && in_loop {
                let condition = self.expression(random, "bool", &environment, 1);
                source.push_str(&format!(
                    "if {condition}{{{};}}\n",
                    if random.pick(2) == 0 {
                        "break"
                    } else {
                        "continue"
                    }
                ));
            } else if !result.is_empty() && random.pick(3) == 0 {
                let condition = self.expression(random, "bool", &environment, 1);
                let value = self.expression(random, result, &environment, 1);
                source.push_str(&format!("if {condition}{{return {value};}}\n"));
            }
        }
        (source, environment)
    }
    fn tail(
        &mut self,
        random: &mut Random,
        ty: &str,
        expression_environment: &[Variable],
        live: &[Variable],
        depth: usize,
    ) -> String {
        let mut source="let mut checksum=0i64;let inferred=inferred_pair();checksum=mix(checksum,inferred.0);checksum=mix(checksum,inferred.1 as i64);\n".to_string();
        for variable in live {
            if variable.ty == "vec" {
                source.push_str(&format!("checksum=mixv(checksum,&{});\n", variable.name));
            } else {
                source.push_str(&format!(
                    "checksum=mix(checksum,{} as i64);\n",
                    variable.name
                ));
            }
        }
        let expression = self.expression(random, ty, expression_environment, depth);
        source.push_str(&format!(
            "let result:{ty}={expression};checksum=mix(checksum,result as i64);\n"
        ));
        if ty == "i64" {
            source.push_str("checksum")
        } else {
            source.push_str(&format!("((checksum%100) as {ty})"));
        }
        source
    }
}
pub fn generate(random: &mut Random) -> Case {
    let mut generator = Generator {
        uid: 0,
        functions: Vec::new(),
        structures: Vec::new(),
    };
    let mut source = prelude().to_string();
    for index in 0..2usize {
        let fields = vec![int_type(random), int_type(random), "bool".to_string()];
        let name = format!("S{index}");
        source.push_str(&format!(
            "struct {name}{{f0:{},f1:{},f2:bool}}\n",
            fields[0], fields[1]
        ));
        generator.structures.push(Structure { name, fields });
    }
    for index in 0..3usize {
        let mut parameters: Vec<String> = Vec::new();
        let mut environment: Vec<Variable> = Vec::new();
        let mut declaration = String::new();
        for parameter in 0..random.pick(3) {
            let ty = int_type(random);
            let name = format!("p{parameter}");
            declaration.push_str(&format!("{name}:{ty},"));
            parameters.push(ty.clone());
            environment.push(Variable {
                name,
                ty,
                mutable: false,
            });
        }
        let result = int_type(random);
        let name = format!("f{index}");
        let (body, live) = generator.statements(random, &environment, 2, 4, &result, false);
        let tail = generator.tail(random, &result, &environment, &live, 2);
        source.push_str(&format!(
            "fn {name}({declaration})->{result}{{\n{body}{tail}\n}}\n"
        ));
        generator.functions.push(Function {
            name,
            parameters,
            result,
        });
    }
    let empty: Vec<Variable> = Vec::new();
    let (body, live) = generator.statements(random, &empty, 2, 9, "i64", false);
    let tail = generator.tail(random, "i64", &empty, &live, 3);
    source.push_str(&format!("pub fn probe()->i64{{\n{body}{tail}\n}}\n"));
    Case {
        family: "typed-values".to_string(),
        source,
        reference: None,
        expectation: "compare".to_string(),
    }
}
