//! Generated aggregate operations preserve shared referents and their owners.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const SHARED: &str = r#"
fn text(value:&str)->String {
    let mut result=String::new();
    result.push_str(value);
    result
}
fn cloned<T:Clone>(value:&T)->T { value.clone() }
#[derive(Clone,PartialEq,Eq)]
struct View<'a> { value:&'a i64, text:String }
#[derive(Clone,PartialEq,Eq)]
struct Pair<'a> { left:&'a i64, right:&'a i64, text:String }
struct Numbers { left:i64, right:i64 }
#[derive(Clone,PartialEq,Eq)]
enum Maybe<'a> {
    Empty,
    Full { left:&'a i64, right:&'a i64, text:String },
}
fn direct<'a>(external:&'a i64)->View<'a> {
    let carrier=View{value:external,text:text("original")};
    carrier.clone()
}
fn generic<'a>(external:&'a i64)->View<'a> {
    let carrier=View{value:external,text:text("generic")};
    cloned(&carrier)
}
fn paired<'a>(values:&'a Numbers)->Pair<'a> {
    let carrier=Pair{left:&values.left,right:&values.right,text:text("pair")};
    cloned(&carrier)
}
fn optional<'a>(values:&'a Numbers,present:bool)->Maybe<'a> {
    if present {
        let carrier=Maybe::Full{left:&values.left,right:&values.right,text:text("enum")};
        return cloned(&carrier);
    }
    let carrier=Maybe::Empty;
    cloned(&carrier)
}
pub fn cloned_views_outlive_carriers_and_keep_owned_siblings()->i64 {
    let number:i64=42;
    let mut first=direct(&number);
    let second=generic(&number);
    if *first.value!=42 || *second.value!=42 {return 1;}
    let independent=first.clone();
    let equal_number:i64=42;
    let equal_view=direct(&equal_number);
    let other_number:i64=43;
    let other_view=direct(&other_number);
    if first!=independent || first!=equal_view || first==other_view {return 4;}
    if !first.eq(&equal_view) || first.ne(&independent) {return 5;}
    first.text.clear();
    first.text.push_str("changed");
    drop(first);
    if independent.text.len()!=8 || *independent.value!=42 {return 2;}
    drop(independent);
    if second.text.len()!=7 || *second.value!=42 {return 3;}
    0
}
pub fn cloned_multiple_leaves_keep_the_common_source()->i64 {
    let numbers=Numbers{left:4,right:2};
    let value=paired(&numbers);
    if *value.left*10+*value.right!=42 || value.text.len()!=4 {return 1;}
    let copied=value.clone();
    drop(value);
    if *copied.left!=4 || *copied.right!=2 {return 2;}
    0
}
pub fn cloned_enum_variants_keep_present_and_absent_leaves()->i64 {
    let numbers=Numbers{left:4,right:2};
    let empty=optional(&numbers,false);
    let copied_empty=empty.clone();
    drop(empty);
    match copied_empty {Maybe::Empty=>{},Maybe::Full{..}=>return 1}
    let full=optional(&numbers,true);
    let copied_full=cloned(&full);
    drop(full);
    match copied_full {
        Maybe::Full{left,right,text}=>{
            if *left*10+*right!=42 || text.len()!=4 {return 2;}
        },
        Maybe::Empty=>return 3,
    }
    0
}
pub fn clone_retains_exclusive_ancestors_until_the_last_view_dies()->i64 {
    let mut number:i64=42;
    let exclusive=&mut number;
    let carrier=View{value:&*exclusive,text:text("borrowed")};
    let copied=cloned(&carrier);
    drop(carrier);
    if *copied.value!=42 {return 1;}
    drop(copied);
    *exclusive=43;
    *exclusive-43
}
"#;

const OWNERS: &str = r#"
struct Token { phase:i64 }
impl Drop for Token {
    fn drop(&mut self) {
        assert!(self.phase==2);
        self.phase=3;
    }
}
struct Owner { value:i64, token:Box<Token> }
impl Clone for Owner {
    fn clone(&self)->Self {
        assert!(self.token.phase==1);
        Owner{value:self.value,token:Box::new(Token{phase:1})}
    }
}
impl PartialEq for Owner {
    fn eq(&self,other:&Self)->bool {
        assert!(self.token.phase==1);
        assert!(other.token.phase==1);
        self.value==other.value
    }
}
impl Eq for Owner {}
impl Drop for Owner {
    fn drop(&mut self) {
        assert!(self.token.phase==1);
        self.token.phase=2;
    }
}
fn owner(value:i64)->Owner {Owner{value,token:Box::new(Token{phase:1})}}
#[derive(Clone,PartialEq,Eq)]
struct Record { first:Owner, second:Owner }
#[derive(Clone,PartialEq,Eq)]
enum Packet { Empty, Record{value:Record}, Number(i64) }
fn same<T:PartialEq>(left:&T,right:&T)->bool {left.eq(right)}
pub fn equality_borrows_owned_fields_and_preserves_drop_order()->i64 {
    let mut left=Record{first:owner(4),second:owner(2)};
    let mut right=left.clone();
    if !(left==right) || left!=right {return 1;}
    if !left.eq(&right) || left.ne(&right) || !same(&left,&right) {return 2;}
    right.second.value=3;
    if left==right || !(left!=right) {return 3;}
    left.first.value=8;
    if left.first.token.phase!=1 || right.second.token.phase!=1 {return 4;}
    drop(left);
    if right.first.value!=4 || right.first.token.phase!=1 {return 5;}
    drop(right);
    0
}
pub fn enum_equality_preserves_payload_owners_across_mismatched_tags()->i64 {
    let first=Packet::Record{value:Record{first:owner(4),second:owner(2)}};
    let copied=first.clone();
    let empty=Packet::Empty;
    let scalar=Packet::Number(42);
    if first!=copied || !same(&first,&copied) {return 1;}
    if first==empty || first==scalar || !first.ne(&empty) {return 2;}
    drop(first);
    match copied {
        Packet::Record{value}=>{
            if value.first.token.phase!=1 || value.second.token.phase!=1 {return 3;}
            if value.first.value*10+value.second.value!=42 {return 4;}
            drop(value);
        },
        _=>return 5,
    }
    0
}
"#;

fn bundle(source: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "derived_provenance.rs".to_string(),
        text: source.to_string(),
    });
    SourceBundle {
        root: "derived_provenance.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-derived-provenance-{}-{}",
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
fn generated_clones_preserve_referents_without_retaining_local_carriers() {
    same_source(
        SHARED,
        &[
            "cloned_views_outlive_carriers_and_keep_owned_siblings",
            "cloned_multiple_leaves_keep_the_common_source",
            "cloned_enum_variants_keep_present_and_absent_leaves",
            "clone_retains_exclusive_ancestors_until_the_last_view_dies",
        ],
    );
}

#[test]
fn generated_equality_preserves_owner_lifetimes_and_cleanup() {
    same_source(
        OWNERS,
        &[
            "equality_borrows_owned_fields_and_preserves_drop_order",
            "enum_equality_preserves_payload_owners_across_mismatched_tags",
        ],
    );
}

fn rust_metadata(source: &str) -> std::process::Output {
    let directory = directory();
    std::fs::write(directory.join("case.rs"), source).unwrap();
    let output = Command::new("rustc")
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
    output
}

fn rejected_by_both(source: &str, code: &str, rust_code: &str) {
    let problem = compiler::check_bundle(&bundle(source))
        .err()
        .expect("invalid derived operation must be rejected");
    assert_eq!(problem.code, code, "{problem:?}");
    assert_eq!(problem.path, "derived_provenance.rs", "{problem:?}");
    let invalid_line = source
        .lines()
        .position(|line| line.contains("fn leak") || line.contains("fn invalid"))
        .map_or(1, |line| line + 1);
    assert_eq!(problem.line, invalid_line, "{problem:?}");
    let output = rust_metadata(source);
    assert!(!output.status.success(), "Rust accepted invalid source");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(rust_code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_cloning_cannot_hide_local_reference_escape() {
    for body in [
        "fn leak<'a>(external:&'a i64)->View<'a>{let local:i64=7;let carrier=View{value:&local,text:text(\"local\")};carrier.clone()}",
        "fn leak<'a>(external:&'a i64)->Maybe<'a>{let local:i64=7;let carrier=Maybe::Full{left:&local,right:&local,text:text(\"local\")};cloned(&carrier)}",
        "#[derive(Clone)] struct TextView<'a>{value:&'a str} fn leak<'a>(external:&'a str)->TextView<'a>{let local=text(\"local\");let carrier=TextView{value:local.as_str()};carrier.clone()}",
    ] {
        rejected_by_both(&format!("{SHARED}\n{body}"), "E_BORROW", "E0515");
    }
}

#[test]
fn generated_cloning_preserves_exclusive_parent_and_trait_requirements() {
    rejected_by_both(
        &format!(
            "{SHARED}\nfn invalid()->i64{{let mut number:i64=7;let exclusive=&mut number;let carrier=View{{value:&*exclusive,text:text(\"borrowed\")}};let copied=cloned(&carrier);drop(carrier);*exclusive=9;*copied.value}}"
        ),
        "E_BORROW",
        "E0506",
    );
    rejected_by_both(
        "#[derive(Clone)] struct Exclusive<'a>{value:&'a mut i64}",
        "E_TRAIT",
        "E0277",
    );
    // Returning the address of an owned field cannot impersonate the external
    // leaf selected by Clone's projected input contract.
    rejected_by_both(
        "struct Wrong<'a>{external:&'a i64,own:i64} #[derive(Clone)] struct Wrapped<'a>{value:Wrong<'a>} impl<'a> Clone for Wrong<'a>{fn clone(&self)->Self{Wrong{external:&self.own,own:self.own}}}",
        "E_BORROW",
        "lifetime may not live long enough",
    );
}

#[test]
fn conditional_view_joins_remain_an_explicit_conservative_boundary() {
    // This Rust-valid join fails without Clone or derives. Keep the precision
    // limit visible while generated operations use branch-local return probes.
    let source = r#"
struct Numbers { left:i64, right:i64 }
enum Maybe<'a> { Empty, Full{left:&'a i64,right:&'a i64} }
fn optional<'a>(values:&'a Numbers,present:bool)->Maybe<'a> {
    let carrier=if present {
        Maybe::Full{left:&values.left,right:&values.right}
    } else {Maybe::Empty};
    carrier
}
"#;
    let problem = compiler::check_bundle(&bundle(source))
        .err()
        .expect("view join precision has not been implemented");
    assert_eq!(problem.code, "E_MOVE", "{problem:?}");
    assert!(
        problem
            .message
            .contains("branches must leave the same owners and borrows"),
        "{problem:?}"
    );
    let rust = rust_metadata(source);
    assert!(
        rust.status.success(),
        "{}",
        String::from_utf8_lossy(&rust.stderr)
    );
}
