//! Exact current physical layout, independent of source semantic admission.
use super::*;

type LayoutCheck<T> = Result<T, Box<dyn std::error::Error>>;

fn page_frames(bytes: &[u8]) -> LayoutCheck<Vec<(PageDigest, &[u8])>> {
    let mut at = 0_usize;
    let mut pages = Vec::new();
    while at < bytes.len() {
        let header_end = at.checked_add(8).ok_or("page header overflow")?;
        let length = usize::try_from(u64::from_le_bytes(
            bytes
                .get(at..header_end)
                .ok_or("truncated page header")?
                .try_into()?,
        ))?;
        assert!((1..=65_536).contains(&length));
        let end = header_end
            .checked_add(length)
            .ok_or("page extent overflow")?;
        let page = bytes.get(header_end..end).ok_or("truncated page")?;
        pages.push((PageDigest::of(page), page));
        at = end;
    }
    assert_eq!(at, bytes.len());
    Ok(pages)
}

#[test]
fn selfhost_artifact_interface_and_reference_maps_match_host_physical_pages() -> LayoutCheck<()> {
    for (width, probe) in [
        (32_usize, "probe-interface-map-pages"),
        (33, "probe-reference-map-pages"),
    ] {
        let boundary = (16_384 - 108) / (22 + width);
        for count in [0_usize, 1, boundary, boundary + 1] {
            let mut entries = (0..count)
                .map(|index| {
                    let mut key = vec![0_u8; 17];
                    key[0] = 2;
                    key[1] = (index % 256) as u8;
                    key[2..6].copy_from_slice(&((index / 256 + 1) as u32).to_be_bytes());
                    let value = (0..width)
                        .map(|at| ((index * 17 + at * 23) % 256) as u8)
                        .collect::<Vec<_>>();
                    (key, value)
                })
                .collect::<Vec<_>>();
            entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            let input = entries
                .iter()
                .flat_map(|(key, value)| key.iter().chain(value).copied())
                .collect::<Vec<_>>();
            let mut expected_pages = MemoryPageStore::default();
            let expected =
                PersistentMap::from_sorted(&mut expected_pages, entries, &mut MapWork::default())?;
            let output = image(probe, &input);
            let emitted = page_frames(&output)?;
            let expected_objects = expected_pages.objects().collect::<BTreeMap<_, _>>();
            assert_eq!(emitted.len(), expected_objects.len());
            let mut reconstructed = MemoryPageStore::default();
            let mut unique = BTreeSet::new();
            for (digest, bytes) in &emitted {
                assert!(unique.insert(*digest), "duplicate physical page");
                assert_eq!(*bytes, *expected_objects.get(digest).ok_or("foreign page")?);
                reconstructed.write_page(*digest, bytes)?;
            }
            let root_digest = emitted.last().ok_or("missing root page")?.0;
            assert_eq!(root_digest, expected.root().page());
            let report = PersistentMap::from_root(expected.root())
                .verify(&reconstructed, &mut MapWork::default())?;
            assert_eq!(report.entries, u64::try_from(count)?);
            assert_eq!(report.pages, u64::try_from(emitted.len())?);
            assert_eq!(emitted.len() > 1, count > boundary && count > 1);
            eprintln!(
                "selfhost artifact current map width={width} entries={count} pages={} root={root_digest}",
                emitted.len()
            );
        }
    }
    Ok(())
}

fn object_rows(objects: &BTreeMap<ObjectKey, Vec<u8>>) -> LayoutCheck<Vec<u8>> {
    let rows_length = objects.len().checked_mul(56).ok_or("row size overflow")?;
    let mut data = vec![0_u8; 8 + rows_length];
    data[..8].copy_from_slice(&u64::try_from(rows_length)?.to_le_bytes());
    for (index, (key, bytes)) in objects.iter().enumerate() {
        let at = 8 + index * 56;
        let offset = u64::try_from(data.len())?;
        data[at..at + 4].copy_from_slice(&u32::from(key.domain.tag()).to_le_bytes());
        data[at + 8..at + 16].copy_from_slice(&offset.to_le_bytes());
        data[at + 16..at + 24].copy_from_slice(&u64::try_from(bytes.len())?.to_le_bytes());
        data[at + 24..at + 56].copy_from_slice(&key.digest.bytes());
        data.extend_from_slice(bytes);
    }
    Ok(data)
}

#[test]
fn selfhost_artifact_targeted_packs_match_host_with_oversized_singleton() -> LayoutCheck<()> {
    let target = 4 * 1024 * 1024;
    let objects = [vec![17_u8; target - 232], vec![29_u8; 31]]
        .into_iter()
        .map(|bytes| (ObjectKey::for_bytes(ObjectDomain::Blob, &bytes), bytes))
        .collect::<BTreeMap<_, _>>();
    let mut builder = PackBuilder::default();
    for (key, bytes) in &objects {
        builder.insert(*key, bytes)?;
    }
    let expected = builder.seal_targeted(target)?;
    assert_eq!(expected.len(), 2);
    assert!(
        expected
            .iter()
            .any(|pack| pack.bytes.len() == target + 1 && pack.metadata.entries.len() == 1)
    );
    let output = image("probe-target-packs", &object_rows(&objects)?);
    let mut at = 0_usize;
    let mut seen = BTreeMap::new();
    for pack in expected {
        let header_end = at.checked_add(40).ok_or("segment header overflow")?;
        let header = output
            .get(at..header_end)
            .ok_or("truncated segment header")?;
        let length = usize::try_from(u64::from_be_bytes(header[..8].try_into()?))?;
        assert_eq!(&header[8..40], &pack.id.bytes());
        let end = header_end
            .checked_add(length)
            .ok_or("segment extent overflow")?;
        let actual = output.get(header_end..end).ok_or("truncated pack")?;
        assert_eq!(actual, pack.bytes);
        let metadata = PackMetadata::decode(actual, true)?;
        metadata.verify_all(actual)?;
        for entry in metadata.entries {
            let start = usize::try_from(entry.offset)?;
            let end = start
                .checked_add(usize::try_from(entry.encoded_length)?)
                .ok_or("object extent overflow")?;
            assert!(
                seen.insert(
                    entry.key,
                    actual.get(start..end).ok_or("object extent")?.to_vec()
                )
                .is_none()
            );
        }
        at = end;
    }
    assert_eq!(at, output.len());
    assert_eq!(seen, objects);
    eprintln!(
        "selfhost artifact targeted packs objects={} segments=2 bytes={} output_sha256={}",
        objects.len(),
        output.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&output))
    );
    Ok(())
}

#[test]
fn selfhost_artifact_target_pack_plan_preserves_exact_threshold_and_extent() -> LayoutCheck<()> {
    let cases: &[(&[u64], &[[u64; 3]])] = &[
        (&[1], &[[0, 56, 234]]),
        (&[4_194_071], &[[0, 56, 4_194_304]]),
        (&[4_194_071, 1], &[[0, 56, 4_194_304], [56, 112, 234]]),
        (&[4_194_072], &[[0, 56, 4_194_305]]),
        (&[2_096_991, 2_096_991], &[[0, 112, 4_194_304]]),
        (
            &[2_096_992, 2_096_991],
            &[[0, 56, 2_097_225], [56, 112, 2_097_224]],
        ),
    ];
    for (sizes, parts) in cases {
        let mut rows = vec![0_u8; sizes.len() * 56];
        for (index, size) in sizes.iter().enumerate() {
            rows[index * 56 + 16..index * 56 + 24].copy_from_slice(&size.to_le_bytes());
        }
        let mut expected = u64::try_from(parts.len())?.to_le_bytes().to_vec();
        let total = parts
            .iter()
            .try_fold(0_u64, |sum, part| sum.checked_add(40)?.checked_add(part[2]))
            .ok_or("expected extent overflow")?;
        expected.extend_from_slice(&total.to_le_bytes());
        for part in *parts {
            for value in part {
                expected.extend_from_slice(&value.to_le_bytes());
            }
        }
        assert_eq!(image("probe-target-pack-plan", &rows), expected);
    }
    for length in [0_usize, 1, 55, 57] {
        assert_eq!(
            result("probe-target-pack-plan", &vec![0_u8; length]),
            (3, vec![])
        );
    }
    for payload in [i64::MAX as u64, i64::MAX as u64 - 233] {
        let mut row = vec![0_u8; 56];
        row[16..24].copy_from_slice(&payload.to_le_bytes());
        assert_eq!(result("probe-target-pack-plan", &row), (3, vec![]));
    }
    Ok(())
}
