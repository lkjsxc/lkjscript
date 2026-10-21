//! Independent ordered directories exercise raw byte windows on four engines.
use super::*;

pub(super) fn assert_probe(name: &str, bytes: &[u8], expected: &[u8]) {
    assert_eq!(result(name, bytes), (0, expected.to_vec()), "{name}");
    static IMAGES: OnceLock<std::sync::Mutex<BTreeMap<String, Vec<u8>>>> = OnceLock::new();
    let image = IMAGES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .entry(name.to_owned())
        .or_insert_with(|| {
            let module = "source-index-probe";
            let (_, program) = probes(module);
            let entry = program.function(function(module, name)).unwrap().0 as usize;
            let payload = input::export(program, entry).unwrap();
            let (source, compiler) = compiler();
            let (code, image) = compile_with_policy(
                source,
                compiler,
                &payload,
                NormalizedRunPolicy::foreground(),
            );
            assert_eq!(code, 0);
            image
        })
        .clone();
    let output = native_filter_tests::execute_filter(&image, bytes, &[], 127);
    assert_eq!(output.status.code(), Some(0), "{name}: {:?}", output.stderr);
    assert_eq!(output.stdout, expected, "{name}");
    assert!(output.stderr.is_empty());
}

fn word(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn packet(mode: u32, rows: &[Vec<u8>], stride: usize, key: &[u8], extra: u32) -> Vec<u8> {
    let mut bytes = vec![0; 27];
    for row in rows {
        assert!(row.len() <= stride);
        bytes.extend_from_slice(row);
        bytes.resize(bytes.len() + stride - row.len(), 0xe7);
    }
    bytes.extend_from_slice(&[0xa5; 5]);
    let key_at = bytes.len();
    bytes.extend_from_slice(key);
    if mode != 0 {
        bytes.extend_from_slice(&[0x31, 0x73, 0x19]);
    }
    for (at, value) in [
        (0, mode),
        (4, 27),
        (8, u32::try_from(rows.len()).unwrap()),
        (12, u32::try_from(key_at).unwrap()),
        (16, u32::try_from(rows.first().map_or(0, Vec::len)).unwrap()),
        (20, extra),
    ] {
        word(&mut bytes, at, value);
    }
    bytes
}

fn check(bytes: &[u8], expected: i64) {
    assert_probe("read-index", bytes, &encode(expected));
}

#[test]
fn selfhost_source_index_owner_windows_preserve_domain_and_byte_order() {
    let owner = |domain, id: [u8; 16]| [vec![domain], id.to_vec()].concat();
    let mut first = [0; 16];
    first[0] = 1;
    let mut second = [0; 16];
    second[1] = 1;
    let mut prefix = [7; 16];
    prefix[15] = 2;
    let mut later = prefix;
    later[15] = 3;
    let mut rows = vec![
        owner(1, first),
        owner(1, second),
        owner(1, prefix),
        owner(1, later),
        owner(1, [255; 16]),
        owner(2, [0; 16]),
        owner(255, [0; 16]),
    ];
    rows.sort();
    for (index, key) in rows.iter().enumerate() {
        // The reference marker is deliberately different from its owner domain.
        let reference = [vec![0xbb], key[1..].to_vec()].concat();
        let mut bytes = packet(1, &rows, 50, &reference, u32::from(key[0]));
        check(&bytes, index as i64);
        word(&mut bytes, 12, 27 + u32::try_from(index).unwrap() * 50);
        check(&bytes, index as i64); // Also permit a window into the directory.
        check(&packet(0, &rows, 50, key, 50), index as i64);
    }
    for missing in [
        owner(0, [0; 16]),
        owner(1, [0; 16]),
        owner(1, [6; 16]),
        owner(9, [0; 16]),
        owner(255, [255; 16]),
    ] {
        assert!(rows.binary_search(&missing).is_err());
        let reference = [vec![0], missing[1..].to_vec()].concat();
        check(&packet(1, &rows, 50, &reference, u32::from(missing[0])), -1);
    }
}

#[test]
fn selfhost_source_index_type_windows_preserve_long_prefixes_and_offset() {
    let mut a = vec![0; 32];
    a[0] = 1;
    let mut b = vec![0; 32];
    b[1] = 1;
    let mut c = vec![17; 32];
    c[31] = 2;
    let mut d = c.clone();
    d[31] = 3;
    let mut rows = vec![a, b, c, d, vec![255; 32]];
    rows.sort();
    for (index, key) in rows.iter().enumerate() {
        let reference = [vec![0xcc], key.clone()].concat();
        let mut bytes = packet(2, &rows, 32, &reference, 0);
        check(&bytes, index as i64);
        word(&mut bytes, 12, 26 + u32::try_from(index).unwrap() * 32);
        check(&bytes, index as i64);
        check(&packet(0, &rows, 32, key, 32), index as i64);
    }
    for missing in [vec![0; 32], vec![16; 32], vec![254; 32]] {
        assert!(rows.binary_search(&missing).is_err());
        check(&packet(2, &rows, 32, &[vec![0], missing].concat(), 0), -1);
    }
}

#[test]
fn selfhost_source_index_windows_reject_invalid_ranges_without_arithmetic_traps() {
    for (mode, width, stride, domain) in [(1, 17, 50, 3), (2, 32, 32, 0)] {
        let mut key = vec![11; width];
        if mode == 1 {
            key[0] = domain as u8;
        }
        let reference = if mode == 1 {
            [vec![0], key[1..].to_vec()].concat()
        } else {
            [vec![0], key.clone()].concat()
        };
        let base = packet(mode, &[key], stride, &reference, domain);
        for (at, value) in [
            (4, u32::MAX),
            (4, u32::MAX - 1),
            (4, u32::MAX - 2),
            (8, u32::MAX),
            (8, u32::MAX - 1),
            (8, 100_001),
            (12, u32::MAX),
            (12, u32::MAX - 1),
            (12, u32::MAX - 2),
            (12, u32::try_from(base.len() - 1).unwrap()),
        ] {
            let mut bytes = base.clone();
            word(&mut bytes, at, value);
            check(&bytes, -2);
        }
        let mut bytes = base.clone();
        word(
            &mut bytes,
            4,
            u32::try_from(base.len() - stride + 1).unwrap(),
        );
        check(&bytes, -2);
        let mut bytes = base.clone();
        word(&mut bytes, 4, u32::try_from(base.len()).unwrap());
        word(&mut bytes, 8, 0);
        check(&bytes, -1);
        if mode == 1 {
            for domain in [256, u32::MAX, u32::MAX - 1] {
                let mut bytes = base.clone();
                word(&mut bytes, 20, domain);
                check(&bytes, -2);
            }
        }
        let mut too_large = base;
        too_large.resize(8_388_609, 0);
        check(&too_large, -2);
    }
    let base = packet(0, &[vec![3; 17]], 50, &[3; 17], 50);
    for (at, value) in [
        (16, 0),
        (16, 33),
        (16, 16),
        (20, 16),
        (20, 129),
        (8, 100_001),
        (4, u32::MAX - 1),
    ] {
        let mut bytes = base.clone();
        word(&mut bytes, at, value);
        check(&bytes, -2);
    }
}
