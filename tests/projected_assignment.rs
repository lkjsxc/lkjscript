//! Assignment into initialized borrowed and indexed storage replaces its owner,
//! drops the old value once, and evaluates source operands in Rust order.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);
const SOURCE: &str = r#"
struct Token { phase:i64 }
impl Drop for Token {fn drop(&mut self){assert!(self.phase==2);self.phase=3;}}
struct Owner {value:Box<i64>,token:Box<Token>}
impl Drop for Owner {fn drop(&mut self){assert!(self.token.phase==1);self.token.phase=2;}}
fn owner(value:i64)->Owner{Owner{value:Box::new(value),token:Box::new(Token{phase:1})}}
fn identity_target<'a,T>(target:&'a mut T)->&'a mut T{target}
fn replace_generic<T>(target:&mut T,incoming:T){*identity_target(target)=incoming;}
fn replace_owner(target:&mut Owner){*target=owner(42);}
fn replace_box(target:&mut Box<Owner>){*target=Box::new(owner(42));}
fn replace_box_payload(target:&mut Box<Owner>){**target=owner(42);}
pub fn assignments_through_exclusive_owned_targets()->i64{
    let mut first=owner(1);replace_owner(&mut first);replace_generic(&mut first,owner(42));
    let mut second=Box::new(owner(2));replace_box(&mut second);
    let mut third=Box::new(owner(3));replace_box_payload(&mut third);
    if *first.value!=42 || *second.value!=42 || *third.value!=42{return 1;}
    *first.value=40;*second.value=1;*third.value=1;
    *first.value+*second.value+*third.value
}
pub fn indexed_vector_and_array_elements_keep_their_containers()->i64{
    let mut values:Vec<Owner>=Vec::new();values.push(owner(1));values.push(owner(2));
    values[0]=owner(40);values[1]=owner(2);
    let mut array=[Box::new(owner(1)),Box::new(owner(2))];
    array[0]=Box::new(owner(40));*array[1]=owner(2);
    if values.len()!=2 || *values[0].value+*values[1].value!=42{return 1;}
    if *array[0].value+*array[1].value!=42{return 2;}
    let mut boxed=Box::new([owner(1),owner(2)]);
    (*boxed)[0]=owner(40);(*boxed)[1]=owner(2);
    *(*boxed)[0].value+*(*boxed)[1].value
}
struct Row {values:Vec<Owner>}
struct Container {row:Box<Row>}
pub fn nested_vector_box_and_struct_places()->i64{
    let mut row:Vec<Owner>=Vec::new();row.push(owner(1));row.push(owner(2));
    let mut rows:Vec<Row>=Vec::new();rows.push(Row{values:row});
    rows[0].values[0]=owner(40);rows[0].values[1]=owner(2);
    let mut other:Vec<Owner>=Vec::new();other.push(owner(1));other.push(owner(2));
    let mut container=Container{row:Box::new(Row{values:other})};
    container.row.values[0]=owner(20);container.row.values[1]=owner(22);
    if *rows[0].values[0].value+*rows[0].values[1].value!=42{return 1;}
    *container.row.values[0].value+*container.row.values[1].value
}
fn replace_slice(values:&mut [Owner]){values[0]=owner(40);values[1]=owner(2);}
pub fn slices_replace_elements_without_moving_the_slice_owner()->i64{
    let mut values=[owner(1),owner(2)];
    replace_slice(&mut values);
    *values[0].value+*values[1].value
}
struct Counted<'a> {drops:&'a mut i64,payload:Box<i64>}
impl<'a> Drop for Counted<'a>{fn drop(&mut self){*self.drops=*self.drops+1;}}
pub fn replacement_drops_old_and_new_values_once()->i64{
    let mut old_drops:i64=0;let mut new_drops:i64=0;
    {
        let mut value=Counted{drops:&mut old_drops,payload:Box::new(1)};
        *(&mut value)=Counted{drops:&mut new_drops,payload:Box::new(42)};
        if *value.payload!=42{return 1;}
    }
    old_drops*10+new_drops
}
struct PlainView<'a>{value:&'a i64}
fn assigned_view<'a>(external:&'a i64)->PlainView<'a>{
    let mut view=PlainView{value:external};
    *(&mut view)=PlainView{value:external};
    view
}
pub fn replacement_preserves_the_installed_shared_view_origin()->i64{
    let first:i64=7;let second:i64=42;
    let mut view=PlainView{value:&first};
    *(&mut view)=PlainView{value:&second};
    if *view.value!=42{return 1;}
    *assigned_view(&second).value
}
fn scalar_target<'a>(trace:&mut i64,target:&'a mut i64)->&'a mut i64{*trace=*trace*10+2;target}
fn scalar_rhs(trace:&mut i64)->i64{*trace=*trace*10+1;42}
pub fn simple_assignment_evaluates_rhs_then_place_once()->i64{
    let mut trace:i64=0;let mut value:i64=0;
    *scalar_target(&mut trace,&mut value)=scalar_rhs(&mut trace);
    trace*100+value
}
pub fn primitive_compound_assignment_evaluates_rhs_before_reading_place()->i64{
    let mut trace:i64=0;let mut value:i64=0;
    *scalar_target(&mut trace,&mut value)+=scalar_rhs(&mut trace);
    value+={value=2;40};
    trace*100+value
}
fn owner_target<'a>(trace:&mut i64,target:&'a mut Owner)->&'a mut Owner{*trace=*trace*10+2;target}
fn owner_rhs(trace:&mut i64)->Owner{*trace=*trace*10+1;owner(42)}
pub fn affine_assignment_evaluates_rhs_then_borrowed_place_once()->i64{
    let mut trace:i64=0;let mut value=owner(1);
    *owner_target(&mut trace,&mut value)=owner_rhs(&mut trace);
    trace*100+*value.value
}
fn index(trace:&mut i64)->usize{*trace=*trace*10+2;0}
pub fn indexed_assignment_evaluates_rhs_before_the_index()->i64{
    let mut trace:i64=0;let mut values:Vec<Owner>=Vec::new();values.push(owner(1));
    values[index(&mut trace)]=owner_rhs(&mut trace);
    trace*100+*values[0].value
}
pub fn rhs_mutation_selects_the_new_index_and_preserves_container_growth()->i64{
    let mut values:Vec<Owner>=Vec::new();values.push(owner(1));
    let mut selected:usize=0;
    values[selected]={selected=1;values.push(owner(2));owner(42)};
    if values.len()!=2 || *values[0].value!=1{return 1;}
    *values[1].value
}
fn small_target<'a>(target:&'a mut u8)->&'a mut u8{target}
struct Factory{}
impl Factory{fn len<'a>(&self,target:&'a mut u8)->&'a mut u8{target}}
pub fn target_type_controls_unsuffixed_rhs_without_name_shortcuts()->i64{
    let mut value:u8=1;
    *small_target(&mut value)=40;
    *Factory{}.len(&mut value)+=2;
    value as i64
}
"#;

fn bundle(source: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "projected_assignment.rs".to_string(),
        text: source.to_string(),
    });
    SourceBundle {
        root: "projected_assignment.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}
fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-projected-assignment-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}
fn rust(source: &str, executable: bool) -> (std::path::PathBuf, std::process::Output) {
    let directory = directory();
    let input = directory.join("main.rs");
    let output = directory.join("oracle");
    std::fs::write(&input, source).unwrap();
    let mut command = Command::new("rustc");
    command.args([
        "--edition=2024",
        "--cap-lints=allow",
        "-C",
        "overflow-checks=yes",
    ]);
    if !executable {
        command.args(["--crate-type=lib", "--emit=metadata"]);
    }
    let result = command.arg(input).arg("-o").arg(output).output().unwrap();
    (directory, result)
}
fn run_node(artifact: &[u8], expected: &str) {
    let mut observations = String::new();
    for row in expected.lines() {
        let fields: Vec<_> = row.split_whitespace().collect();
        assert_eq!(fields.len(), 2);
        observations.push_str(&format!("{{const actual=instance.exports['fn:{}']();if(actual!=={}n)throw Error('{}: expected {}, got '+actual);}}",fields[0],fields[1],fields[0],fields[1]));
    }
    let script = format!(
        "const chunks=[];process.stdin.on('data',b=>chunks.push(b));process.stdin.on('end',async()=>{{try{{const {{instance}}=await WebAssembly.instantiate(Buffer.concat(chunks),{{lkjscript:{{trap(c,l){{throw Error('trap '+c+':'+l)}}}}}});{observations}}}catch(e){{console.error(e);process.exitCode=1}}}})"
    );
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(artifact).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[test]
fn projected_assignment_matches_rust_ownership_and_evaluation_order() {
    let probes = [
        "assignments_through_exclusive_owned_targets",
        "indexed_vector_and_array_elements_keep_their_containers",
        "nested_vector_box_and_struct_places",
        "slices_replace_elements_without_moving_the_slice_owner",
        "replacement_drops_old_and_new_values_once",
        "replacement_preserves_the_installed_shared_view_origin",
        "simple_assignment_evaluates_rhs_then_place_once",
        "primitive_compound_assignment_evaluates_rhs_before_reading_place",
        "affine_assignment_evaluates_rhs_then_borrowed_place_once",
        "indexed_assignment_evaluates_rhs_before_the_index",
        "rhs_mutation_selects_the_new_index_and_preserves_container_growth",
        "target_type_controls_unsuffixed_rhs_without_name_shortcuts",
    ];
    same_source(SOURCE, &probes);
}

fn same_source(source: &str, probes: &[&str]) {
    let mut executable = source.to_string();
    executable.push_str("\nfn main(){\n");
    for probe in probes {
        executable.push_str(&format!("println!(\"{probe} {{}}\",{probe}());\n"));
    }
    executable.push_str("}\n");
    let (directory, built) = rust(&executable, true);
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let oracle = Command::new(directory.join("oracle")).output().unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let expected = String::from_utf8(oracle.stdout).unwrap();
    assert_eq!(expected.lines().count(), probes.len());
    let artifact =
        compiler::compile(&bundle(source)).unwrap_or_else(|problem| panic!("{problem:?}"));
    run_node(&artifact, &expected);
    std::fs::remove_dir_all(directory).unwrap();
}
fn rejects(source: &str, reason: &str, rust_codes: &[&str]) {
    let problem = compiler::check_bundle(&bundle(source))
        .err()
        .expect("invalid assignment was accepted");
    assert_eq!(problem.code, "E_BORROW", "{problem:?}");
    assert!(problem.message.contains(reason), "{problem:?}");
    assert_eq!(problem.path, "projected_assignment.rs");
    let (directory, result) = rust(source, false);
    std::fs::remove_dir_all(directory).unwrap();
    assert!(!result.status.success(), "Rust accepted {source}");
    let message = String::from_utf8_lossy(&result.stderr);
    assert!(
        rust_codes.iter().any(|code| message.contains(code)),
        "{message}"
    );
}
#[test]
fn assignment_requires_exclusive_writable_storage() {
    for source in [
        "fn bad(value:&Box<i64>){*value=Box::new(42);}",
        "fn bad(value:&Vec<Box<i64>>){value[0]=Box::new(42);}",
        "struct Holder{value:Box<i64>} fn bad(){let holder=Holder{value:Box::new(1)};holder.value=Box::new(42);}",
    ] {
        rejects(source, "immutable", &["E0594", "E0596"]);
    }
}
#[test]
fn live_element_borrows_prevent_owner_replacement() {
    rejects(
        "fn bad()->i64{let mut values:Vec<Box<i64>>=Vec::new();values.push(Box::new(1i64));let view=&values[0];values[0]=Box::new(42);**view}",
        "borrow",
        &["E0502"],
    );
    rejects(
        "fn bad()->i64{let mut value=Box::new(1i64);let view=&*value;*(&mut value)=Box::new(42);*view}",
        "borrow",
        &["E0502"],
    );
}
#[test]
fn replacement_cannot_hide_local_or_conflicting_view_provenance() {
    rejects(
        "struct View<'a>{value:&'a i64} fn leak<'a>(external:&'a i64)->View<'a>{let local:i64=7;let mut view=View{value:external};*(&mut view)=View{value:&local};view}",
        "borrow",
        &["E0515"],
    );
    rejects(
        "struct View<'a>{value:&'a i64} fn bad()->i64{let first:i64=1;let mut second:i64=2;let mut view=View{value:&first};*(&mut view)=View{value:&second};second=42;*view.value}",
        "borrow",
        &["E0506"],
    );
}

#[test]
fn nested_effectful_targets_reuse_their_checked_type_context() {
    let mut source = "fn slot<'a>(value:&'a mut u8)->&'a mut u8{value}".to_string();
    let mut names = Vec::new();
    for depth in [2, 4, 8] {
        let name = format!("nested_{depth}");
        let mut target = "*slot(&mut value)".to_string();
        for _ in 0..depth {
            target = format!("*{{{target}=1;slot(&mut value)}}");
        }
        source.push_str(&format!(
            "pub fn {name}()->i64{{let mut value:u8=0;{target}=42;value as i64}}"
        ));
        names.push(name);
    }
    let probes: Vec<_> = names.iter().map(String::as_str).collect();
    same_source(&source, &probes);
}
