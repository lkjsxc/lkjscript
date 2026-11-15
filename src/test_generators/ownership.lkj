// Stateful ownership sequences with an independent scalar/owner model.
// Random valid statements are composed before a seeded, explicit rejection.
// Expected checksums and rejection codes never come from the compiler.
use super::source_cases::Case;
use super::Random;

#[derive(Clone)]
struct Owner {
    name: String,
    kind: u8,
    alive: bool,
    values: Vec<i64>,
    number: i64,
}
#[derive(Clone)]
struct State {
    owners: Vec<Owner>,
    checksum: i64,
}
struct Generator {
    next: usize,
}
fn mix(h: i64, value: i64) -> i64 {
    ((h % 1000003) * 1009 + (value % 1000003) + 2000006) % 1000003
}
fn sum(values: &[i64]) -> i64 {
    let mut result = 0i64;
    for value in values {
        result += *value;
    }
    result
}
fn available(state: &State, kind: u8, first: usize) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for (index, owner) in state.owners.iter().enumerate() {
        if owner.alive && (kind == 0 || owner.kind == kind) && index >= first {
            out.push(index);
        }
    }
    out
}
fn prelude() -> &'static str {
    r#"struct Capture<T>{value:T}
fn capture<T>(value:T)->T{value}
struct Hs{v:Vec<i64>,n:i64}
struct View<'a>{r:&'a i64,k:i64}
fn mix(h:i64,x:i64)->i64{((h%1000003)*1009+(x%1000003)+2000006)%1000003}
fn sumr(w:&Vec<i64>)->i64{let mut s=0i64;let mut i=0usize;while i<w.len(){s+=w[i];i+=1;}s}
fn sumv(w:Vec<i64>)->i64{sumr(&w)}
fn mv(h:Hs)->Hs{h}
fn first(w:&Vec<i64>)->&i64{&w[0]}
fn slot(w:&mut Vec<i64>)->&mut i64{&mut w[0]}
fn field(h:&Hs)->&i64{&h.n}
fn inner(h:&mut Hs)->&mut Vec<i64>{&mut h.v}
fn unbox(b:Box<i64>)->i64{*b}
fn look(v:View<'_>)->i64{*v.r+v.k}
"#
}
impl Generator {
    fn name(&mut self, prefix: &str) -> String {
        let name = format!("{prefix}{}", self.next);
        self.next += 1;
        name
    }
    fn create(&mut self, random: &mut Random, state: &mut State, kind: u8) -> String {
        let name = self.name("owner");
        let a = random.pick(9) as i64;
        let b = random.pick(9) as i64;
        let number = random.pick(9) as i64;
        let source = match kind {
            1 => format!("let mut {name}:Vec<i64>=vec![{a},{b}];\n"),
            2 => format!("let mut {name}:Box<i64>=Box::new({number});\n"),
            _ => format!("let mut {name}=Hs{{v:vec![{a},{b}],n:{number}}};\n"),
        };
        state.owners.push(Owner {
            name,
            kind,
            alive: true,
            values: vec![a, b],
            number,
        });
        source
    }
    fn read(&mut self, random: &mut Random, state: &mut State, index: usize) -> String {
        let owner = state.owners[index].clone();
        let name = owner.name;
        let mode = random.pick(4);
        let (expression, value) = match owner.kind {
            1 => match mode {
                0 => (format!("sumr(&{name})"), sum(&owner.values)),
                1 => (
                    format!("{{let r:&Vec<i64>=&{name};r.len() as i64}}"),
                    owner.values.len() as i64,
                ),
                2 => (format!("{{let r=first(&{name});*r}}"), owner.values[0]),
                _ => (
                    format!("{{let r=&{name}[0];let view=View{{r,k:2}};look(view)}}"),
                    owner.values[0] + 2,
                ),
            },
            2 => {
                if mode % 2 == 0 {
                    (format!("{{let r=&*{name};*r}}"), owner.number)
                } else {
                    (
                        format!("{{let view=View{{r:&*{name},k:3}};look(view)}}"),
                        owner.number + 3,
                    )
                }
            }
            _ => match mode {
                0 => (format!("{{let r=field(&{name});*r}}"), owner.number),
                1 => (format!("{{let r=&{name}.v[0];*r}}"), owner.values[0]),
                2 => (format!("{{let r=&{name}.v;sumr(r)}}"), sum(&owner.values)),
                _ => (
                    format!("{{let view=View{{r:&{name}.n,k:1}};look(view)}}"),
                    owner.number + 1,
                ),
            },
        };
        state.checksum = mix(state.checksum, value);
        format!("acc=mix(acc,{expression});\n")
    }
    fn mutate(
        &mut self,
        random: &mut Random,
        state: &mut State,
        index: usize,
        mode: usize,
    ) -> String {
        let owner = state.owners[index].clone();
        let name = owner.name;
        let value = 1 + random.pick(9) as i64;
        if owner.kind == 1 || (owner.kind == 3 && mode % 2 == 0) {
            let target = if owner.kind == 1 {
                name.clone()
            } else {
                format!("{name}.v")
            };
            let receiver = if owner.kind == 1 {
                format!("&mut {name}")
            } else {
                format!("inner(&mut {name})")
            };
            match mode % 5 {
                0 => {
                    state.owners[index].values.push(value);
                    format!("{{let loan=&mut {target};loan.push({value});}}\n")
                }
                1 => {
                    state.owners[index].values[0] += value;
                    format!("{{let loan=slot(&mut {target});*loan+={value};}}\n")
                }
                2 => {
                    state.owners[index].values.push(value);
                    format!("{{let loan={receiver};loan.push({value});}}\n")
                }
                3 => {
                    let old = state.owners[index].values[0];
                    state.owners[index].values[0] = value + 1;
                    state.checksum = mix(state.checksum, old);
                    format!("{{let loan=&mut {target}[0];let old=std::mem::replace(loan,{value});*loan+=1;acc=mix(acc,old);}}\n")
                }
                _ => {
                    state.owners[index].values = vec![value];
                    format!("{target}=Vec::new();{target}.push({value});\n")
                }
            }
        } else {
            let target = if owner.kind == 2 {
                format!("*{name}")
            } else {
                format!("{name}.n")
            };
            let old = state.owners[index].number;
            state.owners[index].number = value + 1;
            state.checksum = mix(state.checksum, old);
            if mode % 2 == 0 {
                format!("{{let loan=&mut {target};let held=Capture{{value:loan}};let old=std::mem::replace(held.value,{value});*held.value+=1;acc=mix(acc,old);}}\n")
            } else {
                format!("{{let loan=&mut {target};let held=capture(loan);let old=std::mem::replace(held,{value});*held+=1;acc=mix(acc,old);}}\n")
            }
        }
    }
    fn move_owner(&mut self, random: &mut Random, state: &mut State, index: usize) -> String {
        let owner = state.owners[index].clone();
        let name = owner.name.clone();
        state.owners[index].alive = false;
        if random.pick(2) == 0 {
            let next = self.name("moved");
            let initializer = if owner.kind == 3 {
                format!("mv({name})")
            } else {
                name
            };
            let mut moved = owner;
            moved.name = next.clone();
            state.owners.push(moved);
            return format!("let mut {next}={initializer};\n");
        }
        let (expression, value) = match owner.kind {
            1 => (format!("sumv({name})"), sum(&owner.values)),
            2 => (format!("unbox({name})"), owner.number),
            _ => (
                format!("{{let moved=mv({name});sumv(moved.v)+moved.n}}"),
                sum(&owner.values) + owner.number,
            ),
        };
        state.checksum = mix(state.checksum, value);
        format!("acc=mix(acc,{expression});\n")
    }
    fn repeated(&mut self, random: &mut Random, state: &mut State, index: usize) -> String {
        let owner = state.owners[index].clone();
        let count = 1 + random.pick(3);
        let value = 1 + random.pick(9) as i64;
        let target = match owner.kind {
            1 => format!("{}[0]", owner.name),
            2 => format!("*{}", owner.name),
            _ => format!("{}.n", owner.name),
        };
        for _ in 0..count {
            let old = if owner.kind == 1 {
                state.owners[index].values[0]
            } else {
                state.owners[index].number
            };
            state.checksum = mix(state.checksum, old);
            if owner.kind == 1 {
                state.owners[index].values[0] = old + value;
            } else {
                state.owners[index].number = old + value;
            }
        }
        let body = format!(
            "{{let loan=&mut {target};let before=*loan;*loan+={value};acc=mix(acc,before);}}"
        );
        if random.pick(2) == 0 {
            format!("for _ in 0..{count}usize{{{body}}}\n")
        } else {
            let counter = self.name("counter");
            format!("let mut {counter}=0usize;while {counter}<{count}{{{counter}+=1;{body}}}\n")
        }
    }
    fn sequence(
        &mut self,
        random: &mut Random,
        state: &mut State,
        depth: usize,
        count: usize,
        protected: usize,
    ) -> String {
        let mut source = String::new();
        for _ in 0..count {
            let choice = random.pick(24);
            if choice < 3 {
                source.push_str(&self.create(random, state, 1 + choice as u8));
                continue;
            }
            let candidates = available(state, 0, 0);
            if candidates.is_empty() {
                source.push_str(&self.create(random, state, 1));
                continue;
            }
            let index = candidates[random.pick(candidates.len())];
            if choice < 8 {
                source.push_str(&self.read(random, state, index));
            } else if choice < 13 {
                source.push_str(&self.mutate(random, state, index, choice - 8));
            } else if choice < 16 {
                let movable = available(state, 0, protected);
                if movable.is_empty() {
                    source.push_str(&self.read(random, state, index));
                } else {
                    let index = movable[random.pick(movable.len())];
                    source.push_str(&self.move_owner(random, state, index));
                }
            } else if choice == 16 && depth > 0 {
                let parity = random.pick(2) as i64;
                let selected = state.checksum % 2 == parity;
                let length = state.owners.len();
                let mut left = state.clone();
                let mut right = state.clone();
                let first = self.sequence(random, &mut left, depth - 1, 3, length);
                let second = self.sequence(random, &mut right, depth - 1, 3, length);
                source.push_str(&format!(
                    "if acc%2=={parity}{{\n{first}}}else{{\n{second}}}\n"
                ));
                *state = if selected { left } else { right };
                state.owners.truncate(length);
            } else if choice == 17 {
                source.push_str(&self.repeated(random, state, index));
            } else if choice == 18 {
                let structures = available(state, 3, protected);
                if structures.is_empty() {
                    source.push_str(&self.create(random, state, 3));
                } else {
                    let index = structures[random.pick(structures.len())];
                    let owner = state.owners[index].clone();
                    state.owners[index].alive = false;
                    state.checksum = mix(state.checksum, sum(&owner.values));
                    state.checksum = mix(state.checksum, owner.number);
                    source.push_str(&format!(
                        "acc=mix(acc,sumv({}.v));acc=mix(acc,{}.n);\n",
                        owner.name, owner.name
                    ));
                }
            } else if choice == 19 {
                let value = random.pick(9) as i64;
                let name = self.name("once");
                source.push_str(&format!("{{let owned=Box::new({value}i64);let {name}=move||{{let held=Capture{{value:owned}};unbox(held.value)}};acc=mix(acc,{name}());}}\n"));
                state.checksum = mix(state.checksum, value);
            } else if choice == 20 {
                let structures = available(state, 3, 0);
                if structures.is_empty() {
                    source.push_str(&self.create(random, state, 3));
                } else {
                    let index = structures[random.pick(structures.len())];
                    let name = state.owners[index].name.clone();
                    state.owners[index].number += 1;
                    state.owners[index].values[0] += 2;
                    source.push_str(&format!(
                        "{{let a=&mut {name}.n;let b=&mut {name}.v;*a+=1;b[0]+=2;}}\n"
                    ));
                }
            } else if choice == 21 {
                let owner = state.owners[index].clone();
                let (target, value) = match owner.kind {
                    1 => (format!("{}[0]", owner.name), owner.values[0]),
                    2 => (format!("*{}", owner.name), owner.number),
                    _ => (format!("{}.n", owner.name), owner.number),
                };
                let delta = random.pick(9) as i64;
                let next = value + delta;
                if owner.kind == 1 {
                    state.owners[index].values[0] = next;
                } else {
                    state.owners[index].number = next;
                }
                state.checksum = mix(state.checksum, next);
                source.push_str(&format!("{{let loan=&mut {target};let mut held=Some(loan);match held.as_mut(){{Some(value)=>**value+={delta},None=>()}}let result=match held{{Some(value)=>*value,None=>0}};acc=mix(acc,result);}}\n"));
            } else {
                source.push_str(&self.read(random, state, index));
            }
        }
        source
    }
}
fn rejection(index: usize) -> (&'static str, &'static str) {
    match index%14{
        0=>("let bad=Box::new(1i64);let moved=bad;acc+=*bad+*moved;","E_MOVE"),
        1=>("let mut bad=vec![1i64];let held=&bad[0];bad.push(2);acc+=*held;","E_BORROW"),
        2=>("let bad=vec![1i64];let held=&bad[0];drop(bad);acc+=*held;","E_BORROW"),
        3=>("let mut bad=1i64;let a=&mut bad;let b=&mut bad;acc+=*a+*b;","E_BORROW"),
        4=>("let bad=Hs{v:vec![1i64],n:2};let moved=bad.v;let whole=mv(bad);acc+=sumv(moved)+whole.n;","E_MOVE"),
        5=>("let mut bad=vec![1i64];let held=first(&bad);bad[0]=2;acc+=*held;","E_BORROW"),
        6=>("let mut bad=Hs{v:vec![1i64],n:2};let held=field(&bad);bad.n=3;acc+=*held;","E_BORROW"),
        7=>("let mut bad=Hs{v:vec![1i64],n:2};let held=inner(&mut bad);bad.v.push(3);held.push(4);","E_BORROW"),
        8=>("let mut bad=1i64;let held=&mut bad;let moved=Some(held);drop(moved);*held=2;","E_MOVE"),
        9=>("let bad=Box::new(1i64);let f=move||unbox(bad);acc+=f();acc+=f();","E_MOVE"),
        10=>("let mut bad=1i64;let held=&mut bad;let moved=Capture{value:held};drop(moved);*held=2;","E_MOVE"),
        11=>("let bad=Hs{v:vec![1i64],n:2};let held=&bad.v[0];acc+=sumv(bad.v);acc+=*held;","E_BORROW"),
        12=>("let bad=Box::new(1i64);let mut i=0usize;while i<2{acc+=unbox(bad);i+=1;}","E_MOVE"),
        _=>("let bad=Box::new(1i64);if acc%2==0{drop(bad);}acc+=*bad;","E_MOVE"),
    }
}
pub fn generate(random: &mut Random, index: usize) -> Case {
    let mut generator = Generator { next: 0 };
    let mut state = State {
        owners: Vec::new(),
        checksum: 0,
    };
    let mut source = prelude().to_string();
    source.push_str("pub fn probe()->i64{\nlet mut acc=0i64;\n");
    for kind in 1..4u8 {
        source.push_str(&generator.create(random, &mut state, kind));
    }
    let count = 9 + random.pick(8);
    source.push_str(&generator.sequence(random, &mut state, 2, count, 0));
    let mut expectation = String::new();
    if index % 4 == 0 {
        let (body, code) = rejection(index / 4);
        source.push_str(body);
        expectation = format!("reject {code}");
    }
    for owner in &state.owners {
        if !owner.alive {
            continue;
        }
        if owner.kind == 1 || owner.kind == 3 {
            let target = if owner.kind == 1 {
                owner.name.clone()
            } else {
                format!("{}.v", owner.name)
            };
            for value in &owner.values {
                state.checksum = mix(state.checksum, *value);
            }
            state.checksum = mix(state.checksum, owner.values.len() as i64);
            source.push_str(&format!("for value in &{target}{{acc=mix(acc,*value);}}acc=mix(acc,{target}.len() as i64);\n"));
        }
        if owner.kind == 2 || owner.kind == 3 {
            let target = if owner.kind == 2 {
                format!("*{}", owner.name)
            } else {
                format!("{}.n", owner.name)
            };
            state.checksum = mix(state.checksum, owner.number);
            source.push_str(&format!("acc=mix(acc,{target});\n"));
        }
    }
    source.push_str("acc\n}\n");
    if expectation.is_empty() {
        expectation = format!("value {}", state.checksum);
    }
    Case {
        family: "ownership-state-model".to_string(),
        source,
        reference: None,
        expectation,
    }
}
