use super::*;

// Retained only as a disjoint test oracle, never a production fallback.
fn reference(prefix: &[u8]) -> bool {
    let prefix = &prefix[..prefix.len().min(512)];
    let line = prefix
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    line.starts_with(b"#!")
        && line
            .windows(6)
            .any(|word| word.eq_ignore_ascii_case(b"python"))
}

fn compare(prefixes: &[Vec<u8>]) {
    for batch in prefixes.chunks(64) {
        let expected = batch
            .iter()
            .map(|prefix| reference(prefix))
            .collect::<Vec<_>>();
        assert_eq!(shebangs(batch), expected);
    }
}

#[test]
fn extensions_preserve_ascii_matching_and_one_result_per_input() {
    assert_eq!(
        extensions(&[
            "py", "pY", "Py", "PY", "", "pyc", ".py", "py ", "ｐｙ", "rs", "épy"
        ]),
        [
            true, true, true, true, false, false, false, false, false, false, false
        ]
    );
    assert!(extensions(&[]).is_empty());
    assert!(shebangs(&[]).is_empty());
}

#[test]
fn every_ascii_case_and_every_single_byte_mutation_match_the_original_rule() {
    let mut prefixes = Vec::new();
    for mask in 0..64 {
        let mut prefix = b"#!/usr/bin/".to_vec();
        prefix.extend(b"python".iter().enumerate().map(|(index, byte)| {
            if mask & (1 << index) != 0 {
                byte.to_ascii_uppercase()
            } else {
                *byte
            }
        }));
        prefix.extend_from_slice(b"3\r\nnot inspected");
        prefixes.push(prefix);
    }
    for position in 0..6 {
        for value in 0..=255 {
            let mut prefix = b"#!python".to_vec();
            prefix[2 + position] = value;
            prefixes.push(prefix);
        }
    }
    assert_eq!(prefixes.len(), 1_600);
    compare(&prefixes);
}

#[test]
fn raw_bytes_first_line_and_adjacent_read_boundaries_are_preserved() {
    let mut prefixes = vec![
        vec![],
        b"#".to_vec(),
        b"#!".to_vec(),
        b"python".to_vec(),
        b" #!python".to_vec(),
        b"\xef\xbb\xbf#!python".to_vec(),
        b"#!\npython".to_vec(),
        b"#!py\nthon".to_vec(),
        b"#!sh\rpython\n".to_vec(),
        b"#!\xff\0PyThOn\xff".to_vec(),
        b"#!not-pythonic".to_vec(),
        b"#!PYTHO".to_vec(),
    ];
    for offset in [2, 3, 64, 255, 500, 505, 506, 507, 510, 511, 512] {
        let mut prefix = vec![b'x'; offset];
        prefix[..2].copy_from_slice(b"#!");
        prefix.extend_from_slice(b"python");
        prefixes.push(prefix.clone());
        prefix[2] = b'\n';
        prefixes.push(prefix);
    }
    for length in [0, 1, 2, 7, 8, 63, 255, 505, 506, 511, 512, 513, 1024] {
        let mut prefix = vec![b'X'; length];
        if length >= 2 {
            prefix[..2].copy_from_slice(b"#!");
        }
        prefixes.push(prefix);
    }
    compare(&prefixes);
    let mut last = vec![b'x'; 506];
    last[..2].copy_from_slice(b"#!");
    last.extend_from_slice(b"python");
    assert_eq!(shebangs(&[last.clone()]), [true]);
    last.insert(2, b'x');
    assert_eq!(shebangs(&[last]), [false]);
}

#[test]
fn rust_decisions_match_the_retained_language_predecessor() {
    use base64::Engine;
    use lkjscript::platform::{ExecutionControl, JsonLimits, native_tool::PureTool};
    let program = PureTool::load(
        include_bytes!("../../../../../native-policy/generated/policy.lkja"),
        &ExecutionControl::uncancelled(),
    )
    .expect("retained predecessor fixture");
    let values = [
        "py", "pY", "Py", "PY", "", "pyc", ".py", "py ", "ｐｙ", "rs", "épy",
    ];
    let input = serde_json::to_vec(&(values,)).unwrap();
    let output = program
        .run_json(
            "extensions",
            &input,
            JsonLimits::default(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(
        extensions(&values),
        serde_json::from_slice::<Vec<bool>>(&output).unwrap()
    );
    let mut prefixes = Vec::new();
    for mask in 0..64 {
        let mut prefix = b"#!/usr/bin/".to_vec();
        prefix.extend(b"python".iter().enumerate().map(|(index, byte)| {
            if mask & (1 << index) != 0 {
                byte.to_ascii_uppercase()
            } else {
                *byte
            }
        }));
        prefixes.push(prefix);
    }
    for position in 0..6 {
        for value in 0..=255 {
            let mut prefix = b"#!python".to_vec();
            prefix[2 + position] = value;
            prefixes.push(prefix);
        }
    }
    for offset in [500, 505, 506, 507, 510, 511, 512] {
        let mut prefix = vec![b'x'; offset];
        prefix[..2].copy_from_slice(b"#!");
        prefix.extend_from_slice(b"python");
        prefixes.push(prefix);
    }
    for batch in prefixes.chunks(64) {
        let values = batch
            .iter()
            .map(|prefix| {
                serde_json::json!({
                    "$bytes": base64::engine::general_purpose::STANDARD.encode(prefix)
                })
            })
            .collect::<Vec<_>>();
        let input = serde_json::to_vec(&(values,)).unwrap();
        let output = program
            .run_json(
                "shebangs",
                &input,
                JsonLimits::default(),
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
        assert_eq!(
            shebangs(batch),
            serde_json::from_slice::<Vec<bool>>(&output).unwrap()
        );
    }
}
