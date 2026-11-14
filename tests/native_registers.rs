#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[path = "../bridge/library.rs"]
mod bootstrap_library;

use lkjscript::{bootstrap_runtime, compiler, interpreter, native, source};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

static EXECUTION: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn compare(name: &str, text: &str, address_space: Option<u64>) {
    run_case(name, text, address_space, None);
}
fn run_case(name: &str, text: &str, address_space: Option<u64>, expected: Option<u64>) {
    let _execution = EXECUTION.lock().unwrap();
    let mut files = bootstrap_library::standard_library();
    files.push(source::SourceInput {
        path: name.to_string(),
        text: text.to_string(),
    });
    let checked = compiler::check_bundle(&source::SourceBundle {
        root: name.to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    })
    .unwrap_or_else(|error| {
        panic!(
            "{name}: {}:{} {} {}",
            error.path, error.line, error.code, error.message
        )
    });
    let function = checked
        .program()
        .functions
        .iter()
        .position(|f| f.name == "probe")
        .unwrap();
    let expected = match expected {
        Some(value) => value,
        None => {
            let interpreted = interpreter::run(checked.program(), function, &[], &mut Vec::new())
                .unwrap_or_else(|error| panic!("{name}: {} {}", error.code(), error.message));
            match interpreted.value {
                interpreter::Scalar::Int { value, .. } => value,
                _ => panic!("{name}: expected integer result"),
            }
        }
    };
    let image = native::encode(
        &checked,
        bootstrap_runtime::get().unwrap(),
        native::Entry {
            function,
            arguments: vec![],
            result_fd: Some(2),
        },
    )
    .unwrap_or_else(|error| panic!("{name}: {} {}", error.code, error.message));
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/native-registers");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{}-{name}.elf", std::process::id()));
    std::fs::write(&path, image.bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = if let Some(bytes) = address_space {
        Command::new("prlimit")
            .arg(format!("--as={bytes}"))
            .arg(&path)
            .output()
            .unwrap()
    } else {
        Command::new(&path).output().unwrap()
    };
    std::fs::remove_file(path).unwrap();
    assert_eq!(output.stderr.len(), 24, "{name}: {output:?}");
    let packet: [u64; 3] = std::array::from_fn(|i| {
        u64::from_le_bytes(output.stderr[i * 8..i * 8 + 8].try_into().unwrap())
    });
    assert!(output.status.success(), "{name}: {packet:?}");
    assert_eq!(packet[0], 0, "{name}: {packet:?}");
    assert_eq!(
        packet[1], expected,
        "{name}: register selection differs from independent interpreter"
    );
}

#[test]
fn nested_loops_keep_carried_locals_through_breaks_and_branches() {
    compare(
        "loop-carried",
        r#"
        pub fn probe() -> i64 {
            let mut sum = 0i64;
            let mut outer = 0i64;
            while outer < 11 {
                let mut inner = 0i64;
                let carried = outer * 7;
                while inner < 9 {
                    inner += 1;
                    if inner % 3 == 0 { continue; }
                    sum += carried + inner;
                    if sum > 400 { break; }
                }
                outer += 1;
            }
            sum
        }
    "#,
        None,
    );
}

#[test]
fn recursive_calls_and_stack_arguments_preserve_live_registers() {
    compare(
        "call-preservation",
        r#"
        fn mix(a:i64,b:i64,c:i64,d:i64,e:i64,f:i64,g:i64,h:i64)->i64 {
            let x = a + b; let y = c * d; let z = e ^ f; x + y + z + g + h
        }
        fn recurse(n:i64)->i64 {
            if n == 0 { return 3; }
            let kept = n * 17;
            kept + recurse(n - 1) + kept
        }
        pub fn probe()->i64 {
            let kept = recurse(18);
            let other = mix(11,13,17,19,23,29,31,37);
            kept + other + mix(other,2,3,4,5,6,7,kept)
        }
    "#,
        None,
    );
}

#[test]
fn narrow_results_and_borrowed_aliases_keep_storage_semantics() {
    compare(
        "narrow-alias",
        r#"
        fn change(value:&mut i8) { *value = value.wrapping_add(17i8); }
        pub fn probe()->i64 {
            let a = 250u8;
            let b = a.wrapping_add(14u8);
            let mut c = b as i8 - 12i8;
            change(&mut c);
            let d = -c;
            let e = !d;
            let f = 65530u16.wrapping_add(20u16);
            let g = -300i16;
            let h = (g >> 2) as i8;
            c as i64 + d as i64 + e as i64 + f as i64 + h as i64
        }
    "#,
        None,
    );
}

#[test]
fn zero_sized_unit_values_survive_references_and_collections() {
    compare(
        "unit-values",
        r#"
        struct Pair { empty: (), number: i64 }
        fn unit(value:()) { let borrowed = &value; let copied = *borrowed; copied }
        pub fn probe()->i64 {
            let value = Pair { empty: (), number: 42 };
            unit(value.empty);
            let mut values:Vec<()> = Vec::new();
            values.push(());
            values.push(());
            let item = values.pop().unwrap();
            unit(item);
            value.number + values.len() as i64 - 1
        }
    "#,
        None,
    );
}

#[test]
fn startup_adapts_to_an_external_address_space_limit() {
    compare(
        "limited-startup",
        "pub fn probe()->i64{let value=Box::new(42i64);*value}",
        Some(4 * 1024 * 1024 * 1024),
    );
}

#[test]
fn scalar_owners_retain_addressable_cleanup_storage() {
    compare(
        "owner-cleanup",
        "pub fn probe()->i64{let unused=Box::new(7i64);drop(Box::new(9i64));42}",
        None,
    );
}

#[test]
fn parameters_are_live_across_a_loop_at_function_entry() {
    compare(
        "entry-loop",
        r#"
        fn touch(value:i64)->i64 { value + 1 }
        fn scan(mut left:i64, remembered:i64)->i64 {
            loop {
                if left == 0 { return remembered; }
                let before = left + 11;
                let after = touch(before);
                left -= 1;
                if after < 0 { return after; }
            }
        }
        pub fn probe()->i64 { scan(8,42) }
    "#,
        None,
    );
}

#[test]
fn heap_pressure_releases_reservation_and_stack_regrows_with_live_references() {
    // The program's actual heap and live frames fit in this kernel allowance.
    // Its first large Vec growth must release unused reservation; recursion
    // then grows the stack again while the heap, references and ABI args live.
    run_case(
        "trim-regrow",
        r#"
        fn grow(n:i64,parent:&i64,a:i64,b:i64,c:i64,d:i64,e:i64,f:i64)->i64 {
            let page = [n;1024];
            if n == 0 { return *parent+a+b+c+d+e+f; }
            let result = grow(n-1,&page[17],a,b,c,d,e,f);
            assert!(page[17] == n);
            result
        }
        pub fn probe()->i64 {
            let mut values = Vec::new();
            let mut index = 0usize;
            while index < 2097152 {
                values.push(7i64);
                index += 1;
            }
            let marker = 41i64;
            let result = grow(96,&marker,1,2,3,4,5,6);
            result + values[0] + values[2097151]
        }
    "#,
        Some(32 * 1024 * 1024),
        Some(36),
    );
}
