//! Named enum fields preserve declaration identity, source evaluation order and ownership.
use lkjscript::ast::Diagnostic;
use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{check, compiler, k};
use std::io::Write;
use std::process::{Command, Stdio};

const ROOT: &str = r#"
mod a;
mod b;
use crate::a::Packet as Imported;
type PacketAlias = a::Packet;

enum Basic { Empty, Pair { left: i64, right: i64 }, NamedEmpty {}, TupleEmpty() }
fn record(order: &mut i64, value: i64) -> i64 {
    *order = *order * 10 + value;
    value
}
pub fn evaluation_order() -> i64 {
    let mut order: i64 = 0;
    let value = Basic::Pair { right: record(&mut order, 2), left: record(&mut order, 1) };
    match value {
        Basic::Pair { right, left: renamed } => order * 100 + renamed * 10 + right,
        _ => 0
    }
}
pub fn rest_and_mutable_binding() -> i64 {
    let value = Basic::Pair { left: 1, right: 40 };
    match value { Basic::Pair { mut right, .. } => { right = right + 2; right }, _ => 0 }
}
pub fn named_empty() -> i64 {
    let value = Basic::NamedEmpty {};
    match value { Basic::NamedEmpty {} => 42, _ => 0 }
}
pub fn empty_brace_forms() -> i64 {
    let unit = Basic::Empty {};
    let tuple = Basic::TupleEmpty {};
    let first = match unit { Basic::Empty {} => 20i64, _ => 0 };
    let second = match tuple { Basic::TupleEmpty {} => 22i64, _ => 0 };
    first + second
}
enum Outcome<T> { Ready { value: T, count: i64 }, Waiting }
fn wrap<T>(value: T) -> Outcome<T> { Outcome::Ready { count: 2, value } }
pub fn inferred_generic() -> i64 {
    let inferred = Outcome::Ready { count: 1, value: Box::new(40i64) };
    let extra = wrap(Box::new(2i64));
    let left = match inferred { Outcome::Ready { value, .. } => *value, Outcome::Waiting => 0 };
    let right = match extra { Outcome::Ready { value, count: _ } => *value, Outcome::Waiting => 0 };
    left + right
}
pub fn scoped_aliases() -> i64 {
    let first: Imported = Imported::Data { payload: Box::new(5) };
    let second = b::Packet::Data { payload: Box::new(35i64) };
    let third = PacketAlias::Data { payload: Box::new(0) };
    let extra = match third { PacketAlias::Data { payload } => *payload + 1 };
    a::take(first) + b::take(second) + extra
}
enum Repeated<T> { Named { first: T, second: T } }
pub fn repeated_generic_field_inference() -> i64 {
    let value = Repeated::Named { first: 1, second: 41i64 };
    match value { Repeated::Named { first, second } => first + second }
}
enum Store { Empty, Data { left: Box<i64>, right: Box<i64> } }
fn read_store(value: &Store) -> i64 {
    match value { Store::Data { right, left } => **left + **right, Store::Empty => 0 }
}
fn change_store(value: &mut Store) {
    match value {
        Store::Data { right, left } => { **left = 7; **right = 35; },
        Store::Empty => {}
    }
}
pub fn shared_and_mutable_borrows() -> i64 {
    let mut value = Store::Data { right: Box::new(1), left: Box::new(2) };
    change_store(&mut value);
    read_store(&value)
}
struct Container { value: Store }
pub fn projected_mutable_field_match() -> i64 {
    let mut holder = Container { value: Store::Data { left: Box::new(1), right: Box::new(2) } };
    match holder.value {
        Store::Data { ref mut left, ref mut right } => { **left = 7; **right = 35; },
        Store::Empty => {}
    }
    read_store(&holder.value)
}
struct Measured {}
impl Measured { fn len(&self) -> i64 { 42 } }
pub fn inferred_method_result() -> i64 {
    let value = Outcome::Ready { value: Measured {}.len(), count: 0 };
    match value { Outcome::Ready { value, .. } => value, Outcome::Waiting => 0 }
}
struct Counter<'a> { count: &'a mut i64, payload: Box<i64> }
impl<'a> Drop for Counter<'a> {
    fn drop(&mut self) { *self.count = *self.count + 1; }
}
enum Owned<'a> { Empty, Named { kept: Box<i64>, discarded: Counter<'a> } }
fn extract(value: Owned<'_>) -> Box<i64> {
    match value { Owned::Named { kept, .. } => kept, Owned::Empty => Box::new(0) }
}
pub fn partial_move_cleanup() -> i64 {
    let mut count: i64 = 0;
    let kept = extract(Owned::Named {
        discarded: Counter { count: &mut count, payload: Box::new(0) },
        kept: Box::new(40)
    });
    *kept + count * 2
}
enum Observed<'a> { Named { value: i64, observed: &'a mut i64 } }
impl<'a> Drop for Observed<'a> {
    fn drop(&mut self) {
        match self { Observed::Named { value, observed } => { **observed = *value; } }
    }
}
pub fn explicit_mutable_field_binding() -> i64 {
    let mut observed: i64 = 0;
    {
        let mut value = Observed::Named { observed: &mut observed, value: 1 };
        match value { Observed::Named { ref mut value, .. } => { *value = 42; } }
    }
    observed
}
enum Guarded { Named { payload: Box<i64> } }
impl Drop for Guarded { fn drop(&mut self) {} }
pub fn explicit_shared_affine_field_binding() -> i64 {
    let value = Guarded::Named { payload: Box::new(21) };
    let first = match value { Guarded::Named { ref payload } => **payload };
    let second = match value { Guarded::Named { ref payload } => **payload };
    first + second
}
enum View<'a> { Empty, Named { label: i64, value: &'a i64 } }
fn view<'a>(value: &'a i64) -> View<'a> { View::Named { value, label: 2 } }
pub fn borrowed_view_transport() -> i64 {
    let number: i64 = 40;
    let wrapped = view(&number);
    match wrapped { View::Named { value, label } => *value + label, View::Empty => 0 }
}
"#;

const A: &str = r#"
pub enum Packet { Data { payload: Box<i64> } }
pub fn take(value: Packet) -> i64 {
    match value { Packet::Data { payload } => *payload + 1 }
}
"#;
const B: &str = r#"
pub enum Packet { Data { payload: Box<i64> } }
pub fn take(value: Packet) -> i64 {
    match value { Packet::Data { payload } => *payload }
}
"#;

fn bundle(files: &[(&str, &str)]) -> SourceBundle {
    SourceBundle {
        root: "main.rs".to_string(),
        library: None,
        files: files
            .iter()
            .map(|(path, text)| SourceInput {
                path: path.to_string(),
                text: text.to_string(),
            })
            .collect(),
    }
}

fn checked(text: &str) -> Result<k::CheckedProgram, Diagnostic> {
    compiler::check_bundle(&bundle(&[("main.rs", text)]))
}

fn rejects(text: &str, code: &str, reason: &str) {
    match checked(text) {
        Ok(_) => panic!("invalid named enum fixture was accepted: {text}"),
        Err(error) => {
            assert_eq!(error.code, code, "{text}: {error:?}");
            assert!(error.message.contains(reason), "{text}: {error:?}");
        }
    }
}

fn run_node(bytes: &[u8], observations: &str) {
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
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn named_construction_matching_and_cleanup_match_the_same_rust_sources() {
    let probes = [
        "evaluation_order",
        "rest_and_mutable_binding",
        "named_empty",
        "empty_brace_forms",
        "projected_mutable_field_match",
        "inferred_method_result",
        "inferred_generic",
        "repeated_generic_field_inference",
        "scoped_aliases",
        "shared_and_mutable_borrows",
        "partial_move_cleanup",
        "explicit_mutable_field_binding",
        "explicit_shared_affine_field_binding",
        "borrowed_view_transport",
    ];
    let directory =
        std::path::Path::new(".artifacts").join(format!("named-enum-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut root = ROOT.to_string();
    root.push_str("\nfn main(){\n");
    for name in probes {
        root.push_str(&format!("println!(\"{name} {{}}\",{name}());\n"));
    }
    root.push_str("}\n");
    std::fs::write(directory.join("main.rs"), root).unwrap();
    std::fs::write(directory.join("a.rs"), A).unwrap();
    std::fs::write(directory.join("b.rs"), B).unwrap();
    let binary = directory.join("oracle");
    let compiled = Command::new("rustc")
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
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let oracle = Command::new(&binary).output().unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let expected = String::from_utf8(oracle.stdout).unwrap();
    let mut script = String::new();
    let mut observed = 0;
    for row in expected.lines() {
        let parts: Vec<_> = row.split_whitespace().collect();
        assert_eq!(parts.len(), 2, "{row}");
        script.push_str(&format!(
            "if(instance.exports['fn:{}']()!=={}n)throw Error('{} result differs');",
            parts[0], parts[1], parts[0]
        ));
        observed += 1;
    }
    assert_eq!(observed, probes.len());
    let bytes = compiler::compile(&bundle(&[("main.rs", ROOT), ("a.rs", A), ("b.rs", B)]))
        .unwrap_or_else(|error| panic!("{error:?}"));
    run_node(&bytes, &script);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn named_constructor_field_sets_are_checked() {
    for fields in ["left:1", "left:1,right:2,extra:3", "left:1,left:2,right:3"] {
        rejects(
            &format!("enum E{{Named{{left:i64,right:i64}}}} fn bad()->E{{E::Named{{{fields}}}}}"),
            "E_TYPE",
            "field",
        );
    }
}

#[test]
fn named_pattern_field_sets_are_checked_even_after_a_default() {
    for fields in ["left", "left,right,extra", "left:a,left:b,right"] {
        rejects(
            &format!(
                "enum E{{Named{{left:i64,right:i64}}}} fn bad(value:E)->i64{{match value{{_=>0,E::Named{{{fields}}}=>1}}}}"
            ),
            "E_TYPE",
            "field",
        );
    }
}

#[test]
fn tuple_and_named_variant_syntax_are_distinct() {
    for source in [
        "enum E{Named{value:i64}} fn bad()->E{E::Named(1)}",
        "enum E{Tuple(i64)} fn bad()->E{E::Tuple{value:1}}",
        "enum E{Named{value:i64}} fn bad(value:E)->i64{match value{E::Named(item)=>item}}",
        "enum E{Tuple(i64)} fn bad(value:E)->i64{match value{E::Tuple{value}=>value}}",
        "enum E{Named{}} fn bad()->E{E::Named}",
        "enum E{Named{}} fn bad()->E{E::Named()}",
        "enum E{Tuple()} fn bad()->E{E::Tuple}",
        "enum E{Empty} fn bad()->E{E::Empty()}",
        "enum E{Tuple()} fn bad(value:E)->i64{match value{E::Tuple=>1}}",
        "enum E{Empty} fn bad(value:E)->i64{match value{E::Empty()=>1}}",
        "enum E{Named{}} fn bad(value:E)->i64{match value{E::Named=>1}}",
    ] {
        rejects(source, "E_TYPE", "variant");
    }
}

#[test]
fn sibling_named_variants_keep_their_nominal_identity() {
    let declarations = "mod a{pub enum E{Named{value:i64}}} mod b{pub enum E{Named{value:i64}}}";
    rejects(
        &format!(
            "{declarations} fn take(value:a::E){{}} fn bad(){{take(b::E::Named{{value:1}});}}"
        ),
        "E_TYPE",
        "declaration",
    );
    rejects(
        &format!(
            "{declarations} fn bad(value:a::E)->i64{{match value{{b::E::Named{{value}}=>value}}}}"
        ),
        "E_TYPE",
        "declaration",
    );
}

#[test]
fn named_payload_reference_provenance_cannot_escape_a_local() {
    rejects(
        "enum View<'a>{Named{label:i64,value:&'a i64}} fn bad<'a>(external:&'a i64)->View<'a>{let local:i64=7;View::Named{value:&local,label:42}}",
        "E_BORROW",
        "",
    );
}

#[test]
fn named_patterns_cannot_move_fields_through_borrowed_or_drop_owners() {
    rejects(
        "enum E{Named{payload:Box<i64>}} fn bad(value:&E)->Box<i64>{match value{E::Named{payload}=>*payload}}",
        "E_BORROW",
        "borrow",
    );
    rejects(
        "enum E{Named{payload:Box<i64>}} impl Drop for E{fn drop(&mut self){}} fn bad(value:E)->Box<i64>{match value{E::Named{payload}=>payload}}",
        "E_MOVE",
        "Drop",
    );
}

#[test]
fn unused_generic_named_constructors_are_checked() {
    rejects(
        "enum E<T>{Named{value:T,count:i64}} fn bad<T>(value:T)->E<T>{E::Named{value,count:true}}",
        "E_TYPE",
        "type",
    );
    rejects(
        "enum E<T>{Named{value:T,count:i64}} fn bad<T>(value:T)->i64{let item=E::Named{value,count:1};match item{_=>0,E::Named{unknown,..}=>1}}",
        "E_TYPE",
        "field",
    );
}

fn direct_borrows(second_variant: usize, second_field: usize, checked_tag: bool) -> k::Program {
    let mut types = k::primitives();
    let fields = vec![
        k::Field {
            name: "left".to_string(),
            ty: k::I64,
        },
        k::Field {
            name: "right".to_string(),
            ty: k::I64,
        },
    ];
    types.push(k::Type {
        name: "Pair".to_string(),
        kind: k::TypeKind::Enum {
            copy: false,
            variants: vec![
                k::Variant {
                    name: "Pair".to_string(),
                    fields: fields.clone(),
                },
                k::Variant {
                    name: "Other".to_string(),
                    fields,
                },
            ],
        },
    });
    types.push(k::Type {
        name: "&mut i64".to_string(),
        kind: k::TypeKind::Ref {
            inner: k::I64,
            mutable: true,
        },
    });
    let place = |variant, field| k::Place {
        local: 2,
        projections: vec![k::Projection::VariantField(variant, field)],
    };
    let drop_local = |local| k::Op::Drop {
        place: k::Place::local(local),
    };
    let mut body = if checked_tag {
        vec![
            k::Op::Int { dst: 0, value: 1 },
            k::Op::Int { dst: 1, value: 2 },
            k::Op::Variant {
                dst: 2,
                variant: 0,
                fields: vec![0, 1],
            },
        ]
    } else {
        vec![drop_local(0), drop_local(1)]
    };
    if checked_tag {
        body.extend([
            k::Op::TagTest {
                dst: 3,
                src: 2,
                variant: 0,
            },
            k::Op::If { condition: 3 },
        ]);
    }
    body.extend([
        k::Op::Borrow {
            dst: 4,
            place: place(0, 0),
            mutable: true,
        },
        k::Op::Borrow {
            dst: 5,
            place: place(second_variant, second_field),
            mutable: true,
        },
        drop_local(5),
        drop_local(4),
        drop_local(2),
        k::Op::Int { dst: 6, value: 42 },
        k::Op::Return { src: 6 },
    ]);
    if checked_tag {
        body.extend([
            k::Op::Else,
            k::Op::Trap {
                message: "unreachable variant".to_string(),
            },
            k::Op::End,
        ]);
    }
    let locals = [k::I64, k::I64, 7, k::BOOL, 8, 8, k::I64]
        .into_iter()
        .enumerate()
        .map(|(index, ty)| k::Local {
            name: format!("v{index}"),
            ty,
            mutable: index == 2,
        })
        .collect();
    k::Program {
        types,
        functions: vec![k::Function {
            file: 0,
            name: "borrow_fields".to_string(),
            params: if checked_tag { 0 } else { 3 },
            result: k::I64,
            return_source: None,
            locals,
            body: body
                .into_iter()
                .enumerate()
                .map(|(line, op)| k::Instruction { op, line: line + 1 })
                .collect(),
            test: false,
        }],
        destructors: Vec::new(),
    }
}

#[test]
fn direct_k_mutable_borrows_of_distinct_checked_variant_fields_are_disjoint() {
    check::check(direct_borrows(0, 1, true)).unwrap_or_else(|error| panic!("{error:?}"));
}

#[test]
fn direct_k_variant_disjointness_does_not_allow_aliases_or_unchecked_tags() {
    for (variant, field, tag, code) in [
        (0, 0, true, "E_BORROW"),
        (1, 1, true, "E_TYPE"),
        (0, 1, false, "E_TYPE"),
    ] {
        let error = match check::check(direct_borrows(variant, field, tag)) {
            Ok(_) => panic!("invalid variant field borrowing was accepted"),
            Err(error) => error,
        };
        assert_eq!(error.code, code, "{error:?}");
    }
}

#[test]
fn explicit_mutable_field_bindings_preserve_original_place_mutability() {
    rejects(
        "enum E{Named{value:i64}} fn bad(){let value=E::Named{value:1};match value{E::Named{ref mut value}=>{*value=2;}}}",
        "E_BORROW",
        "mutable",
    );
}

#[test]
fn mixed_affine_moves_and_explicit_references_are_named_unsupported() {
    let declaration = "enum E{Named{left:Box<i64>,right:Box<i64>}}";
    rejects(
        &format!(
            "{declaration} fn bad(value:E)->i64{{match value{{E::Named{{left,ref right}}=>*left+**right}}}}"
        ),
        "E_UNSUPPORTED",
        "mixed affine",
    );
    rejects(
        &format!(
            "{declaration} struct Holder{{value:E}} fn bad(holder:Holder)->i64{{match holder.value{{E::Named{{left,ref right}}=>*left+**right}}}}"
        ),
        "E_UNSUPPORTED",
        "mixed affine",
    );
}
