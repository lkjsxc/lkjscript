//! The result protocol is ordinary source; encoding still uses Entry.result_fd.
use lkjscript::result_wire::{self, Descriptor, ScalarKind};

fn descriptor(kind: ScalarKind) -> Descriptor {
    Descriptor {
        kind,
        traps: vec!["first".to_string(), "custom\n\0π".to_string()],
    }
}

fn packet(code: u64, value: u64, last: u64, status: u8) -> Vec<u8> {
    let mut bytes = Vec::new();
    for word in [code, value, last] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(format!("\n{status}\n").bytes());
    bytes
}

#[test]
fn descriptor_preserves_scalar_type_and_exact_trap_bytes() {
    let kinds = [
        ScalarKind::Unit,
        ScalarKind::Bool,
        ScalarKind::Char,
        ScalarKind::F64,
        ScalarKind::Int {
            bits: 8,
            signed: true,
        },
        ScalarKind::Int {
            bits: 16,
            signed: false,
        },
        ScalarKind::Int {
            bits: 32,
            signed: true,
        },
        ScalarKind::Int {
            bits: 64,
            signed: false,
        },
    ];
    for kind in kinds {
        let original = descriptor(kind);
        let bytes = result_wire::encode(&original).unwrap();
        let decoded = result_wire::decode(&bytes).unwrap();
        assert!(decoded.kind == original.kind);
        assert_eq!(decoded.traps, original.traps);
        assert_eq!(result_wire::encode(&decoded).unwrap(), bytes);
    }
    let empty = Descriptor {
        kind: ScalarKind::Unit,
        traps: vec![],
    };
    assert_eq!(
        result_wire::encode(&empty).unwrap(),
        b"LKJRD1\n\0\0\0\0\0\0\0\0\0\0"
    );
}

#[test]
fn descriptor_rejects_invalid_framing_without_count_allocations() {
    let valid = result_wire::encode(&descriptor(ScalarKind::Bool)).unwrap();
    for end in 0..valid.len() {
        assert!(
            result_wire::decode(&valid[..end]).is_err(),
            "accepted prefix {end}"
        );
    }
    let mut invalid = Vec::new();
    let mut bytes = valid.clone();
    bytes[5] = b'2';
    invalid.push(bytes);
    let mut bytes = valid.clone();
    bytes[7] = 6;
    invalid.push(bytes);
    let mut bytes = valid.clone();
    bytes[8] = 8;
    invalid.push(bytes);
    let mut bytes = valid.clone();
    bytes[7] = 3;
    bytes[8] = 7;
    invalid.push(bytes);
    let mut bytes = valid.clone();
    bytes[9..17].fill(255);
    invalid.push(bytes);
    let mut bytes = valid.clone();
    bytes[17..25].fill(255);
    invalid.push(bytes);
    let mut bytes = valid.clone();
    bytes[25] = 255;
    invalid.push(bytes);
    let mut bytes = valid;
    bytes.push(0);
    invalid.push(bytes);
    for bytes in invalid {
        assert!(result_wire::decode(&bytes).is_err());
    }
    assert!(
        result_wire::encode(&descriptor(ScalarKind::Int {
            bits: 0,
            signed: false
        }))
        .is_err()
    );
}

#[test]
fn scalar_values_keep_width_signedness_unicode_and_float_bits() {
    let cases = [
        (ScalarKind::Unit, u64::MAX, ""),
        (ScalarKind::Bool, 0, "false\n"),
        (ScalarKind::Bool, 1, "true\n"),
        (ScalarKind::Char, 0x1f980, "🦀\n"),
        (
            ScalarKind::Int {
                bits: 8,
                signed: true,
            },
            255,
            "-1\n",
        ),
        (
            ScalarKind::Int {
                bits: 16,
                signed: true,
            },
            0x8000,
            "-32768\n",
        ),
        (
            ScalarKind::Int {
                bits: 32,
                signed: true,
            },
            0x8000_0000,
            "-2147483648\n",
        ),
        (
            ScalarKind::Int {
                bits: 64,
                signed: true,
            },
            1 << 63,
            "-9223372036854775808\n",
        ),
        (
            ScalarKind::Int {
                bits: 8,
                signed: false,
            },
            255,
            "255\n",
        ),
        (
            ScalarKind::Int {
                bits: 64,
                signed: false,
            },
            u64::MAX,
            "18446744073709551615\n",
        ),
        (ScalarKind::F64, 0, "f64:0x0000000000000000\n"),
        (
            ScalarKind::F64,
            0x8000_0000_0000_0000,
            "f64:0x8000000000000000\n",
        ),
        (
            ScalarKind::F64,
            0x7ff8_0000_0000_0042,
            "f64:0x7ff8000000000042\n",
        ),
    ];
    for (kind, value, expected) in cases {
        let result =
            result_wire::evaluate(&descriptor(kind), "probe.rs", &packet(0, value, 37, 37))
                .unwrap();
        assert_eq!(result.stdout, expected);
        assert_eq!(result.stderr, "");
        assert_eq!(result.status, 37);
    }
    for (kind, value) in [
        (ScalarKind::Bool, 2),
        (ScalarKind::Char, 0xd800),
        (ScalarKind::Char, 0x110000),
        (ScalarKind::Char, 1 << 32),
        (
            ScalarKind::Int {
                bits: 8,
                signed: false,
            },
            256,
        ),
    ] {
        assert!(result_wire::evaluate(&descriptor(kind), "p", &packet(0, value, 0, 0)).is_err());
    }
}

#[test]
fn traps_retain_diagnostic_label_line_and_custom_messages() {
    let descriptor = descriptor(ScalarKind::Unit);
    for (code, message) in [
        (1, "integer overflow"),
        (2, "division by zero"),
        (3, "index out of bounds"),
        (4, "explicit trap"),
        (5, "assertion failed"),
        (6, "heap allocation failed"),
        (7, "host output failure"),
        (8, "stack exhausted"),
        (16, "first"),
        (17, "custom\n\0π"),
    ] {
        let result =
            result_wire::evaluate(&descriptor, "with spaces/probe.rs", &packet(code, 0, 91, 1))
                .unwrap();
        assert_eq!(result.stdout, "");
        assert_eq!(
            result.stderr,
            format!("with spaces/probe.rs:91: R{code:04} {message}\n")
        );
        assert_eq!(result.status, 1);
    }
    for code in [9, 15, 18, u64::MAX] {
        assert!(result_wire::evaluate(&descriptor, "p", &packet(code, 0, 1, 1)).is_err());
    }
}

#[test]
fn parent_status_is_checked_and_control_stream_is_fully_consumed() {
    let descriptor = descriptor(ScalarKind::Unit);
    for bytes in [
        packet(0, 0, 7, 0),
        packet(0, 0, 256, 0),
        packet(3, 0, 9, 0),
        packet(3, 0, 9, 137),
    ] {
        assert!(result_wire::evaluate(&descriptor, "p", &bytes).is_err());
    }
    let base = packet(0, 0, 0, 0);
    for end in 0..base.len() {
        assert!(result_wire::evaluate(&descriptor, "p", &base[..end]).is_err());
    }
    for suffix in [
        b"\n\n".as_slice(),
        b"0\n",
        b"\n-1\n",
        b"\n256\n",
        b"\n0\n\n",
        b"\n0x0\n",
    ] {
        let mut bytes = base[..24].to_vec();
        bytes.extend(suffix);
        assert!(result_wire::evaluate(&descriptor, "p", &bytes).is_err());
    }
}

#[test]
fn backend_default_bytes_and_optional_descriptor_use_the_same_encoder() {
    use lkjscript::{backend, bootstrap_runtime, check, ir_text, native};
    let checked = check::check(
        ir_text::read(
            "lkjscript-ir 1\ntype \"()\" unit\ntype \"bool\" bool\ntype \"i32\" int 32 signed\n\
         type \"i64\" int 64 signed\ntype \"usize\" int 64 unsigned\n\
         type \"u8\" int 8 unsigned\ntype \"f64\" f64\ntype \"i16\" int 16 signed\n\
         fn \"probe\" 0 7\nlocal \"result\" 7 let\n@1 int 0 42\n@2 return 0\nendfn\n\
         fn \"custom_trap\" 0 0\n@3 trap \"custom message\"\nendfn\n",
        )
        .unwrap(),
    )
    .unwrap();
    let runtime = bootstrap_runtime::get().unwrap();
    let function = checked
        .program()
        .functions
        .iter()
        .position(|function| function.name == "probe")
        .unwrap();
    let ordinary = native::encode(
        &checked,
        runtime,
        native::Entry {
            function,
            arguments: vec![],
            result_fd: None,
        },
    )
    .unwrap();
    assert_eq!(
        backend::emit(&checked, runtime, "probe").unwrap(),
        ordinary.bytes
    );
    let (eval, bytes) = backend::emit_for_eval(&checked, runtime, "probe").unwrap();
    let descriptor = result_wire::decode(&bytes).unwrap();
    assert!(
        descriptor.kind
            == ScalarKind::Int {
                bits: 16,
                signed: true
            }
    );
    assert_eq!(descriptor.traps, ordinary.trap_messages);
    assert!(
        descriptor
            .traps
            .iter()
            .any(|message| message == "custom message")
    );
    let expected_eval = native::encode(
        &checked,
        runtime,
        native::Entry {
            function,
            arguments: vec![],
            result_fd: Some(3),
        },
    )
    .unwrap();
    assert_eq!(eval, expected_eval.bytes);
    assert_ne!(eval, ordinary.bytes);
    assert_eq!(
        backend::emit(&checked, runtime, "probe").unwrap(),
        ordinary.bytes
    );
}
