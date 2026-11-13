//! The actual source prelude derives conditional traits through the ordinary
//! compiler. Rust executes the identical prelude bytes and probe definitions.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::compiler;
use lkjscript::source::{SourceBundle, SourceInput};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const PRELUDE: &str = include_str!("../library/prelude.rs");
static NEXT: AtomicUsize = AtomicUsize::new(0);
const COMMON: &str = r#"
fn cloned<T:Clone>(value:&T)->T{value.clone()}
fn updated<T:Clone>(target:&mut T,source:&T){target.clone_from(source);}
fn same<T:PartialEq>(left:&T,right:&T)->bool{left.eq(right)}
fn different<T:PartialEq>(left:&T,right:&T)->bool{left.ne(right)}
fn eq_contract<T:Eq>(value:&T){}
fn copied_twice<T:Copy>(value:T)->T{let other=value;drop(other);value}

mod misleading {
    pub trait Clone { fn clone(&self)->Self; }
    pub trait PartialEq { fn eq(&self,other:&Self)->bool; }
}
struct Token { phase:i64 }
impl Drop for Token {
    fn drop(&mut self){assert!(self.phase==2);self.phase=3;}
}
struct Item { value:Box<i64>, generation:i64, token:Box<Token> }
impl Item {
    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:99,token:Box::new(Token{phase:1})}}
    fn eq(&self,other:&Self)->bool{false}
}
impl misleading::Clone for Item {
    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:77,token:Box::new(Token{phase:1})}}
}
impl misleading::PartialEq for Item {fn eq(&self,other:&Self)->bool{false}}
impl Clone for Item {
    fn clone(&self)->Self{Item{value:Box::new(*self.value),generation:self.generation+1,token:Box::new(Token{phase:1})}}
}
impl PartialEq for Item {
    fn eq(&self,other:&Self)->bool{*self.value==*other.value}
    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self,other:&Self)->bool{!(*self.value==*other.value)}
}
impl Eq for Item {}
impl Drop for Item {fn drop(&mut self){assert!(self.token.phase==1);self.token.phase=2;}}
fn item(value:i64)->Item{Item{value:Box::new(value),generation:0,token:Box::new(Token{phase:1})}}

pub fn affine_option_clone_keeps_independent_payloads()->i64{
    let mut original=Option::Some(item(42));
    let first=original.clone();
    let second=cloned(&first);
    match &mut original{Option::Some(value)=>{*value.value=7;},Option::None=>{}}
    let second_value=second.unwrap();
    if *second_value.value!=42 || second_value.generation!=2{return 1;}
    drop(second_value);
    let first_value=first.unwrap();
    if *first_value.value!=42 || first_value.generation!=1{return 2;}
    let direct=first_value.clone();
    if direct.generation!=99{return 3;}
    drop(direct);drop(first_value);
    let original_value=original.unwrap();
    if *original_value.value!=7 || original_value.generation!=0{return 4;}
    0
}
pub fn affine_result_clones_only_the_active_variant()->i64{
    let original_ok:Result<Item,Item>=Result::Ok(item(40));
    let original_err:Result<Item,Item>=Result::Err(item(2));
    let copied_ok=cloned(&original_ok);
    let copied_err=original_err.clone();
    drop(original_ok);drop(original_err);
    let ok=copied_ok.unwrap();let error=copied_err.unwrap_err();
    if ok.generation!=1 || error.generation!=1{return 1;}
    *ok.value+*error.value
}
pub fn option_clone_from_covers_all_variant_transitions()->i64{
    let mut target:Option<Item>=Option::None;
    let some=Option::Some(item(42));
    let none:Option<Item>=Option::None;
    updated(&mut target,&some);
    match &target{Option::Some(value)=>{if value.generation!=1{return 1;}},Option::None=>return 2}
    target.clone_from(&some);
    match &mut target{Option::Some(value)=>{*value.value=7;},Option::None=>return 3}
    match &some{Option::Some(value)=>{if *value.value!=42{return 4;}},Option::None=>return 5}
    target.clone_from(&none);
    if target.is_some(){return 6;}
    updated(&mut target,&none);
    if !target.is_none(){return 7;}
    0
}
pub fn result_clone_from_covers_both_payload_types_and_transitions()->i64{
    let mut target:Result<Item,Item>=Result::Ok(item(1));
    let ok:Result<Item,Item>=Result::Ok(item(40));
    let error:Result<Item,Item>=Result::Err(item(2));
    target.clone_from(&error);
    match &target{Result::Err(value)=>{if value.generation!=1 || *value.value!=2{return 1;}},Result::Ok(_)=>return 2}
    updated(&mut target,&error);
    updated(&mut target,&ok);
    match &mut target{Result::Ok(value)=>{if value.generation!=1{return 3;}*value.value=42;},Result::Err(_)=>return 4}
    match &ok{Result::Ok(value)=>{if *value.value!=40{return 5;}},Result::Err(_)=>return 6}
    target.clone_from(&ok);
    let final_value=target.unwrap();
    if *final_value.value!=40 || final_value.generation!=1{return 7;}
    0
}
pub fn option_equality_checks_tags_and_builtin_payload_evidence()->i64{
    let mut left=Option::Some(item(42));let right=Option::Some(item(42));
    let other=Option::Some(item(7));let none:Option<Item>=Option::None;
    eq_contract(&left);
    if !(left==right) || left!=right || !same(&left,&right) || different(&left,&right){return 1;}
    if left==other || !(left!=other){return 2;}
    if left==none || none==right || !(left!=none){return 3;}
    if !(none==none) || none!=none{return 4;}
    match &mut left{Option::Some(value)=>{*value.value=7;},Option::None=>return 5}
    if !(left==other) || left==right{return 6;}
    0
}
pub fn result_equality_checks_tags_and_both_payloads()->i64{
    let ok:Result<Item,Item>=Result::Ok(item(42));let ok_same:Result<Item,Item>=Result::Ok(item(42));
    let error:Result<Item,Item>=Result::Err(item(42));let error_same:Result<Item,Item>=Result::Err(item(42));
    let error_other:Result<Item,Item>=Result::Err(item(7));
    eq_contract(&ok);eq_contract(&error);
    if !same(&ok,&ok_same) || ok!=ok_same{return 1;}
    if !same(&error,&error_same) || error!=error_same{return 2;}
    if ok==error || !(ok!=error){return 3;}
    if error==error_other || !different(&error,&error_other){return 4;}
    0
}
#[derive(Clone,Copy)] struct NeOverride { value:i64 }
impl PartialEq for NeOverride {
    fn eq(&self,other:&Self)->bool{self.value==other.value}
    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self,other:&Self)->bool{false}
}
pub fn derived_ne_negates_aggregate_eq_not_payload_ne()->i64{
    let left=Option::Some(NeOverride{value:1});let right=Option::Some(NeOverride{value:2});
    if left==right || !(left!=right) || !left.ne(&right){return 1;}
    let first:Result<NeOverride,NeOverride>=Result::Err(NeOverride{value:1});
    let second:Result<NeOverride,NeOverride>=Result::Err(NeOverride{value:2});
    if first==second || !(first!=second) || !different(&first,&second){return 2;}
    0
}
#[derive(Copy)] struct CopyPayload { value:i64 }
impl Clone for CopyPayload {fn clone(&self)->Self{CopyPayload{value:self.value+1}}}
pub fn generic_enum_clone_calls_clone_even_for_a_copy_payload()->i64{
    let original=Option::Some(CopyPayload{value:41});
    let copy=original.clone();
    if original.unwrap().value!=41 || copy.unwrap().value!=42{return 1;}
    let ok:Result<CopyPayload,CopyPayload>=Result::Ok(CopyPayload{value:41});
    let error:Result<CopyPayload,CopyPayload>=Result::Err(CopyPayload{value:41});
    if cloned(&ok).unwrap().value!=42 || ok.unwrap().value!=41{return 2;}
    if error.clone().unwrap_err().value!=42 || error.unwrap_err().value!=41{return 3;}
    0
}
pub fn copy_is_conditional_and_keeps_original_values_usable()->i64{
    let option=Option::Some(21i64);let first=copied_twice(option);
    if first.unwrap()+option.unwrap()!=42{return 1;}
    let result:Result<i64,i64>=Result::Err(21);let second=copied_twice(result);
    if second.unwrap_err()+result.unwrap_err()!=42{return 2;}
    0
}
fn option_view<'a>(external:&'a i64)->Option<&'a i64>{let carrier=Option::Some(external);cloned(&carrier)}
fn result_ok_view<'a>(external:&'a i64)->Result<&'a i64,i64>{let carrier:Result<&i64,i64>=Result::Ok(external);carrier.clone()}
fn result_err_view<'a>(external:&'a i64)->Result<i64,&'a i64>{let carrier:Result<i64,&i64>=Result::Err(external);cloned(&carrier)}
fn nested_view<'a>(external:&'a i64)->Result<Option<&'a i64>,i64>{let carrier:Result<Option<&i64>,i64>=Result::Ok(Option::Some(external));carrier.clone()}
fn absent_view<'a>()->Option<&'a i64>{let carrier:Option<&i64>=Option::None;carrier.clone()}
pub fn shared_payload_clones_outlive_their_enum_carriers()->i64{
    let value:i64=42;
    if *option_view(&value).unwrap()!=42{return 1;}
    if *result_ok_view(&value).unwrap()!=42{return 2;}
    if *result_err_view(&value).unwrap_err()!=42{return 3;}
    if *nested_view(&value).unwrap().unwrap()!=42{return 4;}
    let absent=absent_view();if !absent.is_none(){return 5;}
    0
}
pub fn exclusive_payloads_can_be_compared_without_being_copied()->i64{
    let mut first:i64=42;let mut second:i64=42;
    {
        let left=Option::Some(&mut first);let right=Option::Some(&mut second);
        if !(left==right) || left!=right{return 1;}
    }
    first=40;second=2;
    first+second
}
pub fn mutation_clone_probe()->i64{Option::Some(item(42)).clone().unwrap().generation}
pub fn mutation_equality_probe()->i64{if Option::Some(item(1))==Option::Some(item(2)){1}else{0}}
pub fn mutation_prelude_probe()->i64{if Option::Some(42i64).is_some(){1}else{0}}
"#;

fn bundle(source: &str, prelude: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files
        .iter_mut()
        .find(|file| file.path == "$std/prelude.rs")
        .unwrap()
        .text = prelude.to_string();
    files.push(SourceInput {
        path: "option_result_traits.rs".to_string(),
        text: source.to_string(),
    });
    SourceBundle {
        root: "option_result_traits.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-option-result-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn rust_library(directory: &std::path::Path, prelude: &str) -> String {
    std::fs::write(directory.join("prelude.rs"), prelude).unwrap();
    std::fs::write(
        directory.join("sequence.rs"),
        include_str!("../library/sequence.rs"),
    )
    .unwrap();
    std::fs::write(
        directory.join("iter.rs"),
        include_str!("../library/iter.rs"),
    )
    .unwrap();
    std::fs::write(
        directory.join("integer.rs"),
        include_str!("../library/integer.rs"),
    )
    .unwrap();
    "#[path=\"prelude.rs\"]mod prelude;#[path=\"iter.rs\"]mod iter;#[path=\"integer.rs\"]mod integer;#[path=\"sequence.rs\"]mod sequence;use prelude::{Option,Result};\n"
        .to_string()
}

fn rust_oracle(source: &str, prelude: &str, probes: &[&str]) -> String {
    let directory = directory();
    let mut main = rust_library(&directory, prelude);
    main.push_str(source);
    main.push_str("\nfn main(){\n");
    for probe in probes {
        main.push_str(&format!("println!(\"{probe} {{}}\",{probe}());\n"));
    }
    main.push_str("}\n");
    std::fs::write(directory.join("main.rs"), main).unwrap();
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
    let output = Command::new(&binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(directory).unwrap();
    String::from_utf8(output.stdout).unwrap()
}

fn run_node(artifact: &[u8], expected: &str) {
    let mut observations = String::new();
    for row in expected.lines() {
        let fields: Vec<_> = row.split_whitespace().collect();
        assert_eq!(fields.len(), 2);
        observations.push_str(&format!(
            "if(instance.exports['fn:{}']()!=={}n)throw Error('{} differs');",
            fields[0], fields[1], fields[0]
        ));
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
fn prelude_traits_execute_with_the_same_production_source_under_rust() {
    let probes = [
        "affine_option_clone_keeps_independent_payloads",
        "affine_result_clones_only_the_active_variant",
        "option_clone_from_covers_all_variant_transitions",
        "result_clone_from_covers_both_payload_types_and_transitions",
        "option_equality_checks_tags_and_builtin_payload_evidence",
        "result_equality_checks_tags_and_both_payloads",
        "derived_ne_negates_aggregate_eq_not_payload_ne",
        "generic_enum_clone_calls_clone_even_for_a_copy_payload",
        "copy_is_conditional_and_keeps_original_values_usable",
        "shared_payload_clones_outlive_their_enum_carriers",
        "exclusive_payloads_can_be_compared_without_being_copied",
    ];
    let expected = rust_oracle(COMMON, PRELUDE, &probes);
    assert_eq!(expected.lines().count(), probes.len());
    let artifact =
        compiler::compile(&bundle(COMMON, PRELUDE)).unwrap_or_else(|error| panic!("{error:?}"));
    run_node(&artifact, &expected);
}

#[test]
fn selected_source_clone_and_equality_bodies_change_derived_behavior() {
    let probes = ["mutation_clone_probe", "mutation_equality_probe"];
    let baseline =
        compiler::compile(&bundle(COMMON, PRELUDE)).unwrap_or_else(|error| panic!("{error:?}"));
    let expected = rust_oracle(COMMON, PRELUDE, &probes);
    run_node(&baseline, &expected);
    let clone_mutation = COMMON.replace(
        "generation:self.generation+1",
        "generation:self.generation+7",
    );
    let clone_artifact = compiler::compile(&bundle(&clone_mutation, PRELUDE))
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert_ne!(clone_artifact, baseline);
    let clone_expected = rust_oracle(&clone_mutation, PRELUDE, &probes);
    assert_ne!(clone_expected, expected);
    run_node(&clone_artifact, &clone_expected);
    let equality_mutation = COMMON.replace(
        "fn eq(&self,other:&Self)->bool{*self.value==*other.value}",
        "fn eq(&self,other:&Self)->bool{true}",
    );
    let equality_artifact = compiler::compile(&bundle(&equality_mutation, PRELUDE))
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert_ne!(equality_artifact, baseline);
    let equality_expected = rust_oracle(&equality_mutation, PRELUDE, &probes);
    assert_ne!(equality_expected, expected);
    run_node(&equality_artifact, &equality_expected);
}

#[test]
fn supplied_prelude_bytes_control_behavior_and_diagnostics() {
    let source =
        "pub fn probe()->i64{let value=Option::Some(42i64).clone();if value.is_some(){1}else{0}}";
    let baseline =
        compiler::compile(&bundle(source, PRELUDE)).unwrap_or_else(|error| panic!("{error:?}"));
    let mutation = PRELUDE.replace("Option::Some(_) => true,", "Option::Some(_) => false,");
    let altered =
        compiler::compile(&bundle(source, &mutation)).unwrap_or_else(|error| panic!("{error:?}"));
    assert_ne!(baseline, altered);
    run_node(&baseline, &rust_oracle(source, PRELUDE, &["probe"]));
    run_node(&altered, &rust_oracle(source, &mutation, &["probe"]));
    let invalid = PRELUDE.replace("Option::Some(_) => true,", "Option::Some(_) => 1i64,");
    match compiler::check_bundle(&bundle(source, &invalid)) {
        Ok(_) => panic!("ill-typed production prelude was accepted"),
        Err(error) => {
            assert_eq!(error.code, "E_TYPE", "{error:?}");
            assert_eq!(error.path, "$std/prelude.rs");
        }
    }
}

fn rust_checks(source: &str, prelude: &str, accepted: bool) {
    let directory = directory();
    let modules = rust_library(&directory, prelude);
    std::fs::write(directory.join("case.rs"), format!("{modules}{source}")).unwrap();
    let output = Command::new("rustc")
        .args([
            "--edition=2024",
            "--crate-type=lib",
            "--cap-lints=allow",
            "--emit=metadata",
        ])
        .arg(directory.join("case.rs"))
        .arg("-o")
        .arg(directory.join("case.rmeta"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    assert_eq!(
        output.status.success(),
        accepted,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    if !accepted {
        let diagnostic = String::from_utf8_lossy(&output.stderr);
        assert!(
            diagnostic.contains("E0277") || diagnostic.contains("E0515"),
            "{diagnostic}"
        );
    }
}

fn rejects(source: &str, prelude: &str, code: &str, reason: &str) {
    match compiler::check_bundle(&bundle(source, prelude)) {
        Ok(_) => panic!("invalid Option/Result trait use was accepted: {source}"),
        Err(error) => {
            assert_eq!(error.code, code, "{source}: {error:?}");
            assert!(error.message.contains(reason), "{source}: {error:?}");
        }
    }
    rust_checks(source, prelude, false);
}

#[test]
fn conditional_traits_require_every_payload_even_when_that_variant_is_absent() {
    for source in [
        "struct Owned{value:Box<i64>}fn require<T:Copy>(value:T){}fn bad(){let value:Option<Owned>=Option::None;require(value);}",
        "struct Owned{value:Box<i64>}fn require<T:Copy>(value:T){}fn bad(){let value:Result<i64,Owned>=Result::Ok(1);require(value);}",
    ] {
        rejects(source, PRELUDE, "E_TRAIT", "Copy");
    }
    for source in [
        "struct NoClone{}fn require<T:Clone>(value:&T){}fn bad(){let value:Option<NoClone>=Option::None;require(&value);}",
        "struct NoClone{}fn require<T:Clone>(value:&T){}fn bad(){let value:Result<i64,NoClone>=Result::Ok(1);require(&value);}",
    ] {
        rejects(source, PRELUDE, "E_TRAIT", "Clone");
    }
    for source in [
        "fn require<T:Eq>(value:&T){}fn bad(){let value:Option<f64>=Option::None;require(&value);}",
        "fn require<T:Eq>(value:&T){}fn bad(){let value:Result<i64,f64>=Result::Ok(1);require(&value);}",
    ] {
        rejects(source, PRELUDE, "E_TRAIT", "Eq");
    }
    rejects(
        "struct NoEq{}impl NoEq{fn eq(&self,other:&Self)->bool{true}}fn require<T:PartialEq>(value:&T){}fn bad(){let value:Option<NoEq>=Option::None;require(&value);}",
        PRELUDE,
        "E_TRAIT",
        "PartialEq",
    );
}

#[test]
fn removing_source_derives_removes_their_contracts() {
    let without_clone = PRELUDE.replace(
        "#[derive(Clone, Copy, PartialEq, Eq)]",
        "#[derive(PartialEq, Eq)]",
    );
    rejects(
        "fn require<T:Clone>(value:&T){}fn bad(){require(&Option::Some(42i64));}",
        &without_clone,
        "E_TRAIT",
        "Clone",
    );
    rejects(
        "fn require<T:Clone>(value:&T){}fn bad(){let value:Result<i64,i64>=Result::Ok(42);require(&value);}",
        &without_clone,
        "E_TRAIT",
        "Clone",
    );
    let without_eq = PRELUDE.replace(
        "#[derive(Clone, Copy, PartialEq, Eq)]",
        "#[derive(Clone, Copy, PartialEq)]",
    );
    rejects(
        "fn require<T:Eq>(value:&T){}fn bad(){require(&Option::Some(42i64));}",
        &without_eq,
        "E_TRAIT",
        "Eq",
    );
}

#[test]
fn cloned_shared_payloads_cannot_escape_their_actual_owners() {
    rejects(
        "fn bad<'a>(external:&'a i64)->Option<&'a i64>{let local:i64=7;let carrier=Option::Some(&local);carrier.clone()}",
        PRELUDE,
        "E_BORROW",
        "",
    );
    rejects(
        "fn bad<'a>(external:&'a i64)->Result<i64,&'a i64>{let local:i64=7;let carrier:Result<i64,&i64>=Result::Err(&local);carrier.clone()}",
        PRELUDE,
        "E_BORROW",
        "",
    );
    rejects(
        "fn require<T:Clone>(value:&T){}fn bad(){let mut value:i64=7;let carrier=Option::Some(&mut value);require(&carrier);}",
        PRELUDE,
        "E_TRAIT",
        "Clone",
    );
}

#[test]
fn replacing_view_payloads_keeps_its_named_mutation_contract_boundary() {
    for source in [
        "fn update<'a>(target:&mut Option<&'a i64>,source:&Option<&'a i64>){target.clone_from(source);}",
        "fn update<'a>(target:&mut Result<&'a i64,&'a i64>,source:&Result<&'a i64,&'a i64>){target.clone_from(source);}",
    ] {
        match compiler::check_bundle(&bundle(source, PRELUDE)) {
            Ok(_) => {
                panic!("default clone_from changed external view provenance without a contract")
            }
            Err(error) => {
                assert_eq!(error.code, "E_UNSUPPORTED", "{error:?}");
                assert!(
                    error
                        .message
                        .contains("provenance-preserving mutation contract"),
                    "{error:?}"
                );
            }
        }
        // Rust accepts the relationship; this test records the explicit current
        // lkjscript boundary rather than counting unsupported behavior as support.
        rust_checks(source, PRELUDE, true);
    }
}
