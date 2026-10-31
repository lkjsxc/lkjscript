//! Canonical artifact row order, representative choice, and measured sorting growth.
use super::*;

const ROW_BYTES: usize = 56;

fn row(tag: u32, digest: [u8; 32], marker: u64) -> [u8; ROW_BYTES] {
    let mut row = [0; ROW_BYTES];
    row[..4].copy_from_slice(&tag.to_le_bytes());
    row[8..16].copy_from_slice(&marker.to_le_bytes());
    row[16..24].copy_from_slice(&(marker + 17).to_le_bytes());
    row[24..].copy_from_slice(&digest);
    row
}

fn rows(rows: &[[u8; ROW_BYTES]]) -> Vec<u8> {
    rows.iter().flat_map(|row| row.iter().copied()).collect()
}

fn ordinal(index: u16) -> [u8; 32] {
    let mut digest = [0; 32];
    digest[30..].copy_from_slice(&index.to_be_bytes());
    digest
}

fn selected(last: i64, prefix: &[u8], input: &[[u8; ROW_BYTES]]) -> Vec<u8> {
    let mut framed = last.to_le_bytes().to_vec();
    framed.extend_from_slice(&u64::try_from(prefix.len()).unwrap().to_le_bytes());
    framed.extend_from_slice(prefix);
    framed.extend_from_slice(&rows(input));
    image("probe-sort-selection", &framed)
}

#[test]
fn selfhost_artifact_order_preserves_first_representatives_and_original_thresholds() {
    let prefix = b"\0existing-output\xff";
    assert_eq!(selected(-1, prefix, &[]), prefix);

    let first = row(25, ordinal(4), 900);
    let later = row(25, ordinal(4), 100);
    assert_eq!(image("probe-sort-rows", &rows(&[first])), rows(&[first]));
    assert_eq!(
        image(
            "probe-sort-rows",
            &rows(&[first, later, later, first, later])
        ),
        rows(&[first]),
    );

    // The threshold is an offset into the unsorted input. Reinterpreting it as
    // an offset into the sorted buffer would select a different key here.
    let low = row(25, ordinal(1), 10);
    let middle = row(25, ordinal(3), 30);
    let high = row(25, ordinal(9), 90);
    let input = [high, first, low, later, middle, high, low];
    let mut expected = prefix.to_vec();
    expected.extend_from_slice(&rows(&[middle, first, high]));
    assert_eq!(selected(2 * 56, prefix, &input), expected);
    let mut expected = prefix.to_vec();
    expected.extend_from_slice(&rows(&[high]));
    assert_eq!(selected(3 * 56, prefix, &input), expected);
    assert_eq!(selected(0, prefix, &input), prefix);

    let sorted = [low, middle, first, high];
    assert_eq!(image("probe-sort-rows", &rows(&input)), rows(&sorted));
    assert_eq!(image("probe-sort-rows", &rows(&sorted)), rows(&sorted));
}

#[test]
fn selfhost_artifact_order_uses_canonical_domain_and_complete_digest_order() {
    // ObjectKey order places the three artifact domains after ordinary domains,
    // in CompilerUnit, MapPage, CompilationManifest order, not numeric tag order.
    let tags = [1_u32, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 25, 23, 24];
    let expected = tags
        .into_iter()
        .map(|tag| {
            row(
                tag,
                [255_u8 - u8::try_from(tag).unwrap(); 32],
                u64::from(tag),
            )
        })
        .collect::<Vec<_>>();
    let input = expected.iter().rev().copied().collect::<Vec<_>>();
    assert_eq!(image("probe-sort-rows", &rows(&input)), rows(&expected));

    // Non-power-of-two length, separated duplicates, and distinctions at the
    // final digest byte exercise partial runs and all 32 comparison bytes.
    let expected = (0_u16..17)
        .map(|index| row(25, ordinal(index), u64::from(index)))
        .collect::<Vec<_>>();
    let mut input = expected.iter().rev().copied().collect::<Vec<_>>();
    for index in [0_u16, 8, 16, 4] {
        input.push(row(25, ordinal(index), 1000 + u64::from(index)));
    }
    assert_eq!(image("probe-sort-rows", &rows(&input)), rows(&expected));

    let mut first_byte = [0; 32];
    first_byte[0] = 1;
    let mut middle_byte = [0; 32];
    middle_byte[16] = 1;
    let ordered = [
        row(25, ordinal(1), 1),
        row(25, middle_byte, 2),
        row(25, first_byte, 3),
    ];
    assert_eq!(
        image(
            "probe-sort-rows",
            &rows(&[ordered[2], ordered[0], ordered[1]])
        ),
        rows(&ordered),
    );
}

#[test]
fn selfhost_artifact_order_work_scales_below_quadratic_rescans() {
    let (source, program) = probes();
    let declaration = declaration_named(source, "probe-sort-rows");
    let mut observations = Vec::new();
    for count in [64_u16, 128, 256] {
        let expected = (0..count)
            .map(|index| row(25, ordinal(index), u64::from(index)))
            .collect::<Vec<_>>();
        let input = rows(&expected.iter().rev().copied().collect::<Vec<_>>());
        let started = std::time::Instant::now();
        let (value, report) = NormalizedVm::for_test(program, NormalizedRunPolicy::foreground())
            .invoke(
                declaration,
                vec![NormalizedValue::bytes(input.as_slice())],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
        assert!(!report.instructions_saturated);
        assert!(!report.allocated_bytes_saturated);
        assert_eq!(report.live_call_frames_after, 0);
        let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
            panic!("sort probe result")
        };
        let NormalizedValue::Bytes(actual) = &fields
            .iter()
            .find(|(name, _)| name.as_str() == "image")
            .unwrap()
            .1
        else {
            panic!("sort probe image")
        };
        assert_eq!(actual.as_ref(), rows(&expected));
        eprintln!(
            "selfhost artifact row sort rows={count} input_sha256={} output_sha256={} vm_instructions={} modeled_cumulative_allocation={} maximum_call_depth={} elapsed_ms={}",
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&input)),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(actual.as_ref())),
            report.instructions,
            report.allocated_bytes,
            report.maximum_call_depth,
            started.elapsed().as_millis(),
        );
        observations.push(report.instructions);
    }
    // This generous growth envelope rejects a return to repeated full scans;
    // elapsed time and modeled bytes are observations, not admission policies.
    for pair in observations.windows(2) {
        assert!(
            pair[1] < pair[0].checked_mul(3).unwrap(),
            "{observations:?}"
        );
    }
}
