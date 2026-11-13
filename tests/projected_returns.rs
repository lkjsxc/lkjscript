//! Returning a stored reference follows its referent, not the temporary value
//! that carried the reference to the callee. K source projections are checked.
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{check, compiler, k, k_text, wasm};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const SOURCE: &str = r#"
fn stored<'a>(carrier: &&'a i64) -> &'a i64 { *carrier }
fn stored_twice<'a>(carrier: &&&'a i64) -> &'a i64 { **carrier }
fn through_local<'a>(external: &'a i64) -> &'a i64 {
    let carrier = external;
    stored(&carrier)
}
fn through_two_locals<'a>(external: &'a i64) -> &'a i64 {
    let first = external;
    let second = &first;
    stored_twice(&second)
}
fn cloned<T: Clone>(carrier: &T) -> T { carrier.clone() }
fn generic_through_local<'a>(external: &'a i64) -> &'a i64 {
    let carrier = external;
    cloned(&carrier)
}
#[derive(Clone, Copy)]
struct View<'a> { value: &'a i64 }
fn copied_view<'a>(carrier: &View<'a>) -> View<'a> { *carrier }
fn copied_field<'a>(carrier: &View<'a>) -> &'a i64 { carrier.value }
fn view_through_local<'a>(external: &'a i64) -> View<'a> {
    let carrier = View { value: external };
    copied_view(&carrier)
}
fn generic_view_through_local<'a>(external: &'a i64) -> View<'a> {
    let carrier = View { value: external };
    cloned(&carrier)
}
fn field_through_local<'a>(external: &'a i64) -> &'a i64 {
    let carrier = View { value: external };
    copied_field(&carrier)
}
fn stored_text<'a>(carrier: &&'a str) -> &'a str { *carrier }
fn text_through_local<'a>(external: &'a str) -> &'a str {
    let carrier = external;
    stored_text(&carrier)
}
pub fn direct_shared_reference() -> i64 {
    let value: i64 = 42;
    let returned = through_local(&value);
    *returned
}
pub fn twice_projected_shared_reference() -> i64 {
    let value: i64 = 42;
    let returned = through_two_locals(&value);
    *returned
}
pub fn generic_cloned_reference() -> i64 {
    let value: i64 = 42;
    let returned = generic_through_local(&value);
    *returned
}
pub fn copied_aggregate() -> i64 {
    let value: i64 = 42;
    let returned = view_through_local(&value);
    *returned.value
}
pub fn generic_cloned_aggregate() -> i64 {
    let value: i64 = 42;
    let returned = generic_view_through_local(&value);
    *returned.value
}
pub fn copied_aggregate_field() -> i64 {
    let value: i64 = 42;
    let returned = field_through_local(&value);
    *returned
}
#[derive(Clone, Copy)]
struct Pair<'a> { left: &'a i64, right: &'a i64 }
struct Numbers { left: i64, right: i64 }
fn swapped<'a>(carrier: &Pair<'a>) -> Pair<'a> {
    Pair { left: carrier.right, right: carrier.left }
}
fn swapped_through_local<'a>(values: &'a Numbers) -> Pair<'a> {
    let carrier = Pair { left: &values.left, right: &values.right };
    swapped(&carrier)
}
pub fn swapped_shared_fields_keep_their_real_targets() -> i64 {
    let values = Numbers { left: 4, right: 2 };
    let returned = swapped_through_local(&values);
    *returned.left * 10 + *returned.right
}
fn mutable_carrier<'a>(carrier: &mut &'a i64) -> &'a i64 { *carrier }
pub fn carrier_replacement_keeps_the_original_referent() -> i64 {
    let first: i64 = 7;
    let second: i64 = 9;
    let mut carrier = &first;
    let returned = mutable_carrier(&mut carrier);
    carrier = &second;
    *returned + *carrier
}
pub fn exclusive_parent_survives_carrier_replacement() -> i64 {
    let mut first: i64 = 7;
    let second: i64 = 9;
    let exclusive = &mut first;
    let mut carrier = &*exclusive;
    let returned = mutable_carrier(&mut carrier);
    carrier = &second;
    *returned + *carrier
}
fn nested_exclusive<'a>(carrier: &&'a &mut i64) -> &'a i64 { &***carrier }
pub fn nested_exclusive_descriptor_survives_until_the_returned_view_dies() -> i64 {
    let mut value: i64 = 7;
    let exclusive = &mut value;
    let inner = &exclusive;
    let carrier = &inner;
    let returned = nested_exclusive(carrier);
    *returned
}
pub fn copied_and_assigned_carrier_keeps_its_descriptor() -> i64 {
    let value: i64 = 42;
    let first = &value;
    let second = &first;
    let transferred = second;
    let mut assigned = second;
    assigned = transferred;
    **assigned
}
pub fn reassignment_releases_the_previous_carrier() -> i64 {
    let mut value: i64 = 7;
    let spare: i64 = 9;
    let first = &value;
    let second = &spare;
    let mut carrier = &first;
    let before = **carrier;
    carrier = &second;
    value = 33;
    before + **carrier + value
}
pub fn shared_and_exclusive_borrows_still_end_after_their_last_use() -> i64 {
    let mut value: i64 = 1;
    let shared = &value;
    let first = *shared;
    value = 20;
    let exclusive = &mut value;
    let reborrowed = &*exclusive;
    let second = *reborrowed;
    value = 21;
    first + second + value
}
pub fn branch_reassignment_does_not_retain_old_carrier_edges() -> i64 {
    let mut value: i64 = 7;
    let spare: i64 = 9;
    let first = &value;
    let second = &spare;
    let mut carrier = &first;
    let before = **carrier;
    if before == 7 { carrier = &second; } else { carrier = &second; }
    value = 33;
    before + **carrier + value
}
fn returned_from_loop<'a>(external: &'a i64, first_path: bool) -> &'a i64 {
    let first = external;
    let second = &first;
    let mut returned = external;
    loop {
        if first_path { returned = stored(&first); break; }
        returned = stored_twice(&second);
        break;
    }
    returned
}
pub fn loop_break_paths_preserve_carrier_dependencies() -> i64 {
    let value: i64 = 21;
    let first = returned_from_loop(&value, true);
    let second = returned_from_loop(&value, false);
    *first + *second
}
pub fn text_reference() -> usize {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.push(206); bytes.push(177); bytes.push(230); bytes.push(176); bytes.push(180);
    bytes.push(240); bytes.push(159); bytes.push(152); bytes.push(128);
    let text = std::str::from_utf8(bytes.as_slice()).unwrap();
    let returned = text_through_local(text);
    returned.len()
}
"#;

fn bundle(text: &str) -> SourceBundle {
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "projected_returns.rs".to_string(),
        text: text.to_string(),
    });
    SourceBundle {
        root: "projected_returns.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    }
}

fn directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-projected-returns-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
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
fn source_carriers_can_die_after_their_stored_references_are_returned() {
    let probes = [
        "direct_shared_reference",
        "twice_projected_shared_reference",
        "generic_cloned_reference",
        "copied_aggregate",
        "generic_cloned_aggregate",
        "copied_aggregate_field",
        "text_reference",
        "loop_break_paths_preserve_carrier_dependencies",
        "copied_and_assigned_carrier_keeps_its_descriptor",
        "reassignment_releases_the_previous_carrier",
        "shared_and_exclusive_borrows_still_end_after_their_last_use",
        "branch_reassignment_does_not_retain_old_carrier_edges",
        "carrier_replacement_keeps_the_original_referent",
        "exclusive_parent_survives_carrier_replacement",
        "nested_exclusive_descriptor_survives_until_the_returned_view_dies",
        "swapped_shared_fields_keep_their_real_targets",
    ];
    let directory = directory();
    let mut source = SOURCE.to_string();
    source.push_str("\nfn main(){\n");
    for name in probes {
        source.push_str(&format!("println!(\"{name} {{}}\",{name}());\n"));
    }
    source.push_str("}\n");
    std::fs::write(directory.join("main.rs"), source).unwrap();
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
    let oracle = Command::new(&binary).output().unwrap();
    assert!(
        oracle.status.success(),
        "{}",
        String::from_utf8_lossy(&oracle.stderr)
    );
    let expected = String::from_utf8(oracle.stdout).unwrap();
    let mut observations = String::new();
    let mut count = 0;
    for row in expected.lines() {
        let fields: Vec<_> = row.split_whitespace().collect();
        assert_eq!(fields.len(), 2);
        observations.push_str(&format!(
            "if(instance.exports['fn:{}']()!=={}n)throw Error('{} differs');",
            fields[0], fields[1], fields[0]
        ));
        count += 1;
    }
    assert_eq!(count, probes.len());
    let artifact =
        compiler::compile(&bundle(SOURCE)).unwrap_or_else(|problem| panic!("{problem:?}"));
    run_node(&artifact, &observations);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn source_local_referents_and_descriptors_still_cannot_escape() {
    let declarations = "fn stored<'a>(carrier:&&'a i64)->&'a i64{*carrier} fn cloned<T:Clone>(carrier:&T)->T{carrier.clone()} #[derive(Clone,Copy)] struct View<'a>{value:&'a i64,number:i64} fn copied_view<'a>(carrier:&View<'a>)->View<'a>{*carrier}";
    for body in [
        "fn leak<'a>(external:&'a i64)->&'a i64{let local:i64=7;let carrier=&local;stored(&carrier)}",
        "fn leak<'a>(external:&'a i64)->&'a i64{let local:i64=7;let carrier=&local;cloned(&carrier)}",
        "fn leak<'a>(external:&'a i64)->View<'a>{let local:i64=7;let carrier=View{value:&local,number:42};copied_view(&carrier)}",
        "fn leak<'a>(carrier:&View<'a>)->&'a i64{&carrier.number}",
        "fn borrow_carrier<'a>(value:&'a &'a i64)->&'a &'a i64{value} fn leak<'a>(external:&'a i64)->&'a &'a i64{let carrier=external;borrow_carrier(&carrier)}",
    ] {
        let text = format!("{declarations} {body}");
        match compiler::check_bundle(&bundle(&text)) {
            Ok(_) => panic!("local origin escaped through a reference carrier: {body}"),
            Err(problem) => {
                assert_eq!(problem.code, "E_BORROW", "{body}: {problem:?}");
                assert_eq!(problem.path, "projected_returns.rs");
            }
        }
        let directory = directory();
        std::fs::write(directory.join("case.rs"), &text).unwrap();
        let rust = Command::new("rustc")
            .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
            .arg(directory.join("case.rs"))
            .arg("-o")
            .arg(directory.join("case.rmeta"))
            .output()
            .unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        assert!(
            !rust.status.success(),
            "Rust accepted the local escape: {body}"
        );
        let diagnostics = String::from_utf8_lossy(&rust.stderr);
        assert!(
            diagnostics.contains("E0515") || diagnostics.contains("E0621"),
            "{diagnostics}"
        );
    }
}

fn local(ty: usize) -> k::Local {
    k::Local {
        name: String::new(),
        ty,
        mutable: false,
    }
}
fn place(local: usize, projections: Vec<k::Projection>) -> k::Place {
    k::Place { local, projections }
}
fn drop_local(local: usize) -> k::Op {
    k::Op::Drop {
        place: k::Place::local(local),
    }
}
fn function(
    name: &str,
    params: usize,
    locals: Vec<k::Local>,
    result: usize,
    depth: Option<usize>,
    operations: Vec<k::Op>,
) -> k::Function {
    k::Function {
        file: 0,
        name: name.to_string(),
        params,
        locals,
        result,
        return_source: depth.map(|dereferences| k::ReturnSource::Parameter {
            index: 0,
            projections: vec![k::Projection::Deref; dereferences],
        }),
        body: operations
            .into_iter()
            .enumerate()
            .map(|(line, op)| k::Instruction {
                column: 1,
                op,
                line: line + 1,
            })
            .collect(),
        test: false,
    }
}
fn types() -> Vec<k::Type> {
    let mut types = k::primitives();
    for inner in [k::I64, 7, 8] {
        let index = types.len();
        types.push(k::Type {
            name: format!("ref{index}"),
            kind: k::TypeKind::Ref {
                inner,
                mutable: false,
            },
        });
    }
    types.push(k::Type {
        name: "View".to_string(),
        kind: k::TypeKind::Struct {
            copy: true,
            fields: vec![
                k::Field {
                    name: "value".to_string(),
                    ty: 7,
                },
                k::Field {
                    name: "descriptor".to_string(),
                    ty: k::I64,
                },
            ],
        },
    });
    types.push(k::Type {
        name: "refView".to_string(),
        kind: k::TypeKind::Ref {
            inner: 10,
            mutable: false,
        },
    });
    types
}
fn program(functions: Vec<k::Function>) -> k::Program {
    k::Program {
        types: types(),
        functions,
        destructors: Vec::new(),
    }
}
fn copy(depth: usize) -> k::Function {
    function(
        "copy",
        1,
        vec![local(7 + depth), local(7)],
        7,
        Some(depth),
        vec![
            k::Op::Read {
                dst: 1,
                place: place(0, vec![k::Projection::Deref; depth]),
                moving: false,
            },
            drop_local(0),
            k::Op::Return { src: 1 },
        ],
    )
}
fn reject_k(program: k::Program, reason: &str) {
    match check::check(program) {
        Ok(_) => panic!("K accepted forged projected provenance"),
        Err(problem) => {
            assert_eq!(problem.code, "E_BORROW", "{problem:?}");
            assert!(problem.message.contains(reason), "{problem:?}");
        }
    }
}

#[test]
fn direct_k_nested_copy_contracts_check_and_round_trip() {
    for depth in [1, 2] {
        let original = program(vec![copy(depth)]);
        check::check(original.clone()).unwrap();
        let text = k_text::write(&original);
        let reparsed = k_text::read(&text).unwrap();
        assert!(
            reparsed == original,
            "K text roundtrip differs:\n{}",
            k_text::write(&reparsed)
        );
        check::check(reparsed).unwrap();
    }
    let field = function(
        "field",
        1,
        vec![local(11), local(7)],
        7,
        Some(1),
        vec![
            k::Op::Read {
                dst: 1,
                place: place(0, vec![k::Projection::Deref, k::Projection::Field(0)]),
                moving: false,
            },
            drop_local(0),
            k::Op::Return { src: 1 },
        ],
    );
    check::check(program(vec![field])).unwrap();
}

#[test]
fn direct_k_call_preserves_referent_after_carrier_cleanup() {
    let probe = function(
        "probe",
        0,
        vec![local(k::I64), local(7), local(8), local(7), local(k::I64)],
        k::I64,
        None,
        vec![
            k::Op::Int { dst: 0, value: 42 },
            k::Op::Borrow {
                dst: 1,
                place: k::Place::local(0),
                mutable: false,
            },
            k::Op::Borrow {
                dst: 2,
                place: k::Place::local(1),
                mutable: false,
            },
            k::Op::Call {
                dst: 3,
                function: 0,
                args: vec![2],
            },
            drop_local(1),
            k::Op::Read {
                dst: 4,
                place: place(3, vec![k::Projection::Deref]),
                moving: false,
            },
            drop_local(3),
            drop_local(0),
            k::Op::Return { src: 4 },
        ],
    );
    let checked = check::check(program(vec![copy(1), probe])).unwrap();
    let artifact = wasm::emit(&checked).unwrap();
    run_node(
        &artifact,
        "if(instance.exports['fn:probe']()!==42n)throw Error('projected K call differs');",
    );
}

#[test]
fn direct_k_rejects_invalid_projection_depth_and_nonview_sources() {
    for depth in [2, 3, 8] {
        let mut invalid = copy(1);
        invalid.return_source = Some(k::ReturnSource::Parameter {
            index: 0,
            projections: vec![k::Projection::Deref; depth],
        });
        reject_k(program(vec![invalid]), "source contract");
    }
    let mut invalid = copy(1);
    invalid.return_source = Some(k::ReturnSource::Parameter {
        index: 1,
        projections: vec![k::Projection::Deref],
    });
    reject_k(program(vec![invalid]), "source contract");
    let mut invalid = copy(1);
    invalid.result = k::I64;
    reject_k(program(vec![invalid]), "source contract");
}

#[test]
fn direct_k_rejects_returning_a_reference_descriptor_instead_of_its_referent() {
    let forged = function(
        "descriptor",
        1,
        vec![local(9), local(8)],
        8,
        Some(2),
        vec![
            k::Op::Borrow {
                dst: 1,
                place: place(0, vec![k::Projection::Deref, k::Projection::Deref]),
                mutable: false,
            },
            drop_local(0),
            k::Op::Return { src: 1 },
        ],
    );
    reject_k(program(vec![forged]), "declared source");
}

#[test]
fn direct_k_rejects_an_unrelated_scalar_field_of_the_same_carrier() {
    let forged = function(
        "wrong_field",
        1,
        vec![local(11), local(7)],
        7,
        Some(1),
        vec![
            k::Op::Borrow {
                dst: 1,
                place: place(0, vec![k::Projection::Deref, k::Projection::Field(1)]),
                mutable: false,
            },
            drop_local(0),
            k::Op::Return { src: 1 },
        ],
    );
    reject_k(program(vec![forged]), "declared source");
}

#[test]
fn direct_k_rejects_other_parameters_and_local_storage() {
    let other = function(
        "other",
        2,
        vec![local(8), local(8), local(7)],
        7,
        Some(1),
        vec![
            k::Op::Read {
                dst: 2,
                place: place(1, vec![k::Projection::Deref]),
                moving: false,
            },
            drop_local(0),
            drop_local(1),
            k::Op::Return { src: 2 },
        ],
    );
    reject_k(program(vec![other]), "declared source");
    let local = function(
        "local",
        1,
        vec![local(8), local(k::I64), local(7)],
        7,
        Some(1),
        vec![
            k::Op::Int { dst: 1, value: 7 },
            k::Op::Borrow {
                dst: 2,
                place: k::Place::local(1),
                mutable: false,
            },
            drop_local(0),
            k::Op::Return { src: 2 },
        ],
    );
    reject_k(program(vec![local]), "declared source");
}

#[test]
fn shared_projected_results_keep_the_underlying_owner_borrowed() {
    let source = r#"
#[derive(Clone,Copy)] struct Pair<'a>{left:&'a i64,right:&'a i64}
struct Numbers{left:i64,right:i64}
fn swapped<'a>(carrier:&Pair<'a>)->Pair<'a>{Pair{left:carrier.right,right:carrier.left}}
fn bad()->i64 {
    let mut values=Numbers{left:1,right:2};
    let returned={let carrier=Pair{left:&values.left,right:&values.right};swapped(&carrier)};
    values.left=9;
    *returned.right
}
"#;
    match compiler::check_bundle(&bundle(source)) {
        Ok(_) => panic!("projected call discarded the underlying owner's borrow"),
        Err(problem) => assert_eq!(problem.code, "E_BORROW", "{problem:?}"),
    }
    let directory = directory();
    std::fs::write(directory.join("case.rs"), source).unwrap();
    let rust = Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
        .arg(directory.join("case.rs"))
        .arg("-o")
        .arg(directory.join("case.rmeta"))
        .output()
        .unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    assert!(!rust.status.success());
    assert!(String::from_utf8_lossy(&rust.stderr).contains("E0506"));
}

#[test]
fn direct_k_cannot_detach_an_exclusive_leaf_from_its_carrier() {
    let mut types = k::primitives();
    types.push(k::Type {
        name: "exclusive".to_string(),
        kind: k::TypeKind::Ref {
            inner: k::I64,
            mutable: true,
        },
    });
    types.push(k::Type {
        name: "carrier".to_string(),
        kind: k::TypeKind::Ref {
            inner: 7,
            mutable: false,
        },
    });
    types.push(k::Type {
        name: "shared".to_string(),
        kind: k::TypeKind::Ref {
            inner: k::I64,
            mutable: false,
        },
    });
    let borrowed = function(
        "exclusive",
        1,
        vec![local(8), local(9)],
        9,
        Some(1),
        vec![
            k::Op::Borrow {
                dst: 1,
                place: place(0, vec![k::Projection::Deref, k::Projection::Deref]),
                mutable: false,
            },
            drop_local(0),
            k::Op::Return { src: 1 },
        ],
    );
    reject_k(
        k::Program {
            types,
            functions: vec![borrowed],
            destructors: Vec::new(),
        },
        "source contract",
    );
}

#[test]
fn projected_shared_results_do_not_thaw_an_exclusive_ancestor() {
    for source in [
        "fn inner<'a>(r:&mut &'a i64)->&'a i64{*r} fn bad()->i64{let mut x:i64=7;let y:i64=9;let exclusive=&mut x;let mut r=&*exclusive;let out=inner(&mut r);r=&y;*exclusive=9;*out+*r}",
        "fn returned<'a>(r:&&'a &mut i64)->&'a i64{&***r} fn bad()->i64{let mut value:i64=7;let exclusive=&mut value;let inner=&exclusive;let carrier=&inner;let out=returned(carrier);*exclusive=9;*out}",
    ] {
        match compiler::check_bundle(&bundle(source)) {
            Ok(_) => panic!("exclusive ancestor became writable while a returned view was live"),
            Err(problem) => assert_eq!(problem.code, "E_BORROW", "{problem:?}"),
        }
        let directory = directory();
        std::fs::write(directory.join("case.rs"), source).unwrap();
        let rust = Command::new("rustc")
            .args(["--edition=2024", "--crate-type=lib", "--emit=metadata"])
            .arg(directory.join("case.rs"))
            .arg("-o")
            .arg(directory.join("case.rmeta"))
            .output()
            .unwrap();
        std::fs::remove_dir_all(directory).unwrap();
        assert!(!rust.status.success());
        assert!(String::from_utf8_lossy(&rust.stderr).contains("E0506"));
    }
}

#[test]
fn source_exclusive_leaf_projection_is_a_named_unsupported_boundary() {
    let source = "fn bad<'a>(carrier:&&'a mut i64)->&'a i64{&**carrier}";
    match compiler::check_bundle(&bundle(source)) {
        Ok(_) => panic!("exclusive projected return contract was accepted"),
        Err(problem) => {
            assert_eq!(problem.code, "E_UNSUPPORTED", "{problem:?}");
            assert!(
                problem.message.contains("shared reference leaves"),
                "{problem:?}"
            );
        }
    }
}

#[test]
fn per_leaf_contract_text_preserves_exact_and_static_sources() {
    for fixture in [
        include_str!("conformance/k/per_leaf_transport.k"),
        include_str!("conformance/k/per_leaf_static_mix.k"),
    ] {
        let original = k_text::read(fixture).unwrap();
        check::check(original.clone()).unwrap();
        let reparsed = k_text::read(&k_text::write(&original)).unwrap();
        assert!(
            original == reparsed,
            "per-leaf contract text changed meaning"
        );
        check::check(reparsed).unwrap();
    }
}
