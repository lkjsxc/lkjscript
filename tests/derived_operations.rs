//! Executable derives use exact field contracts and declaration-level rules.
#[path = "../bridge/library.rs"]
mod bootstrap_library;
use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn bundle(source: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "derived_operations.rs".to_string(),
        text: source.to_string(),
    });
    SourceBundle {
        root: "derived_operations.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-derived-operations-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn same_source(source: &str, probes: &[&str]) {
    let directory = directory();
    let mut oracle = source.to_string();
    oracle.push_str("\nfn main(){\n");
    for name in probes {
        oracle.push_str(&format!("assert_eq!({name}(),0,\"{name}\");\n"));
    }
    oracle.push_str("}\n");
    std::fs::write(directory.join("main.rs"), oracle).unwrap();
    let binary = directory.join("oracle");
    let built = Command::new("rustc")
        .args([
            "--edition=2024",
            "--cap-lints=allow",
            "-C",
            "overflow-checks=yes",
        ])
        .arg(directory.join("main.rs"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let observed = Command::new(binary).output().unwrap();
    assert!(
        observed.status.success(),
        "{}",
        String::from_utf8_lossy(&observed.stderr)
    );
    let artifact =
        compiler::compile(&bundle(source)).unwrap_or_else(|problem| panic!("{problem:?}"));
    let mut script = String::from(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{try{const {instance}=await WebAssembly.instantiate(Buffer.concat(chunks),{lkjscript:{trap(c,l){throw Error('trap '+c+':'+l)}}});",
    );
    for name in probes {
        script.push_str(&format!(
            "if(instance.exports['fn:{name}']()!==0n)throw Error('{name} failed');"
        ));
    }
    script.push_str("}catch(e){console.error(e);process.exitCode=1}})");
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(&artifact).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn copy_derive_shortcut_matches_declaration_genericity() {
    let source = r#"
#[derive(Copy,PartialEq,Eq)] struct Leaf{value:i64}
impl Clone for Leaf{fn clone(&self)->Self{Leaf{value:42}}}
impl Leaf{fn clone(&self)->Self{Leaf{value:99}}}
#[derive(Clone,Copy)] struct Plain{value:Leaf}
#[derive(Clone,Copy)] struct Generic<T>{value:T}
#[derive(Clone,Copy)] struct Lifetime<'a>{value:Leaf,reference:&'a i64}
#[derive(Clone,Copy)] enum PlainEnum{Value(Leaf)}
#[derive(Clone,Copy)] enum GenericEnum<T>{Value(T)}
#[derive(Clone)] struct Affine{value:Leaf}
fn cloned<T:Clone>(value:&T)->T{value.clone()}
fn wrapped<T:Clone>(value:&Generic<T>)->Generic<T>{value.clone()}
pub fn copies()->i64{
 let number:i64=7;
 let plain=Plain{value:Leaf{value:1}}.clone();
 let generic=wrapped(&Generic{value:Leaf{value:1}});
 let lifetime=Lifetime{value:Leaf{value:1},reference:&number}.clone();
 let affine=Affine{value:Leaf{value:1}}.clone();
 let array=[Leaf{value:1},Leaf{value:2}].clone();
 let tuple=(Leaf{value:1},).clone();
 let selected=cloned(&Leaf{value:1});
 if plain.value.value!=1 || generic.value.value!=42 || lifetime.value.value!=1 {return 1;}
 if affine.value.value!=42 || array[0].value!=42 || array[1].value!=42 || tuple.0.value!=42 || selected.value!=42 {return 2;}
 if (Leaf{value:1}).clone().value!=99 {return 3;}
 let one=PlainEnum::Value(Leaf{value:1}).clone();
 let two=GenericEnum::Value(Leaf{value:1}).clone();
 match one {PlainEnum::Value(value)=>{if value.value!=1{return 4;}}}
 match two {GenericEnum::Value(value)=>{if value.value!=42{return 5;}}}
 0
}
"#;
    same_source(source, &["copies"]);
}

#[test]
fn derived_fields_use_trait_equality_and_short_circuit() {
    let source = r#"
#[derive(Clone)] struct Leaf{value:i64}
impl PartialEq for Leaf {
 fn eq(&self,other:&Self)->bool{assert!(self.value<100);self.value==other.value}
 fn ne(&self,other:&Self)->bool{false}
}
impl Eq for Leaf{}
impl Leaf{fn eq(&self,other:&Self)->bool{false}}
#[derive(Clone,PartialEq,Eq)] struct Pair{first:Leaf,second:Leaf}
#[derive(Clone,PartialEq,Eq)] enum Choice{Empty,Named{first:Leaf,second:Leaf},Tuple(Leaf)}
fn equal<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}
pub fn fields()->i64{
 let left=Pair{first:Leaf{value:1},second:Leaf{value:100}};
 let right=Pair{first:Leaf{value:2},second:Leaf{value:100}};
 if left==right || !left.ne(&right) || equal(&left,&right){return 1;}
 let left=Choice::Named{first:Leaf{value:1},second:Leaf{value:100}};
 let right=Choice::Named{first:Leaf{value:2},second:Leaf{value:100}};
 if left==right || left==Choice::Empty || left==Choice::Tuple(Leaf{value:1}) {return 2;}
 let first=Pair{first:Leaf{value:1},second:Leaf{value:2}};
 let second=first.clone();
 if first!=second || !first.eq(&second) || first.ne(&second){return 3;}
 let tuples=(Leaf{value:1},Leaf{value:2});
 if tuples!=tuples.clone(){return 4;}
 let array=[Leaf{value:1},Leaf{value:2}];
 if array!=array.clone(){return 5;}
 if ()!=(){return 6;}
 0
}
"#;
    same_source(source, &["fields"]);
}

#[test]
fn nested_collections_and_recursive_nominals_reuse_checked_helpers() {
    let source = r#"
#[derive(Clone,PartialEq,Eq)] enum List{End,Next(i64,Box<List>)}
#[derive(Clone,PartialEq,Eq)] struct Graph{lists:Vec<Vec<List>>,label:String}
fn cloned<T:Clone>(value:&T)->T{value.clone()}
pub fn recursive()->i64{
 let chain=List::Next(1,Box::new(List::Next(2,Box::new(List::End))));
 let mut lists:Vec<Vec<List>>=Vec::new(); lists.push(vec![chain]);
 let mut label=String::new();label.push_str("graph");
 let original=Graph{lists,label};
 let copied=cloned(&original);
 if original!=copied{return 1;}
 drop(original);
 let mut changed=Graph{lists:Vec::new(),label:String::new()};
 changed.clone_from(&copied);
 if changed!=copied || changed.label.len()!=5{return 2;}
 0
}
"#;
    same_source(source, &["recursive"]);
}

#[test]
fn loop_conditions_clean_up_projected_indexes_and_owned_receivers() {
    let source = r#"
struct Row{values:Vec<i64>}
fn at(calls:&mut i64)->usize{*calls=*calls+1;0}
pub fn projected_loop()->i64{
 let mut rows=vec![Row{values:vec![1i64,2,3]}];
 let mut index:usize=0;let mut calls:i64=0;
 while index<rows[at(&mut calls)].values.len(){
  rows[0].values[index]=rows[0].values[index]+1;
  index=index+1;
 }
 if calls!=4 || rows[0].values[2]!=4{return 1;}
 let mut count:usize=0;
 while rows[0].values.clone().len()>count {
  count=count+1;
  if count==2 {continue;}
 }
 if count!=3 || rows[0].values.len()!=3{return 2;}
 let mut count:usize=0;
 while rows[0].values.clone().len()>count {
  count=count+1;
  if count==2 {break;}
 }
 if count!=2{return 3;}
 0
}
"#;
    same_source(source, &["projected_loop"]);
}

#[test]
fn container_clone_from_remains_an_explicit_element_update_boundary() {
    for source in [
        "fn probe(){let mut target=Box::new(1i64);target.clone_from(&Box::new(2i64));}",
        "fn probe(){let mut target=[1i64];target.clone_from(&[2i64]);}",
        "fn probe(){let mut target=vec![1i64];target.clone_from(&vec![2i64]);}",
    ] {
        let problem = compiler::check_bundle(&bundle(source))
            .err()
            .expect("container update lowering remains pending");
        assert_eq!(problem.code, "E_UNSUPPORTED", "{problem:?}");
        assert!(
            problem.message.contains("container element update"),
            "{problem:?}"
        );
        let directory = directory();
        std::fs::write(directory.join("case.rs"), source).unwrap();
        let result = Command::new("rustc")
            .args([
                "--edition=2024",
                "--crate-type=lib",
                "--emit=metadata",
                "--cap-lints=allow",
            ])
            .arg(directory.join("case.rs"))
            .arg("-o")
            .arg(directory.join("case.rmeta"))
            .output()
            .unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}
