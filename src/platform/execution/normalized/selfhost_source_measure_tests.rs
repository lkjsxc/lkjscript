//! Opt-in measurement keeps the input carrier identical across compiler changes.
use super::*;

#[test]
#[ignore = "explicit same-carrier VM/reference measurement"]
fn selfhost_source_measure_nominal_on_pinned_carrier() {
    let bytes = if let Some(path) = std::env::var_os("LKJSCRIPT_SELFHOST_CARRIER") {
        std::fs::read(path).unwrap()
    } else {
        graph::packet(&compiler().0)
    };
    if let Some(path) = std::env::var_os("LKJSCRIPT_SELFHOST_CAPTURE") {
        use std::io::Write;
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        output.write_all(&bytes).unwrap();
        output.sync_all().unwrap();
    }
    assert!(bytes.len() >= 128);
    assert_eq!(&bytes[..8], b"LKJSG001");
    eprintln!(
        "selfhost pinned carrier bytes={} sha256={} state={}",
        bytes.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&bytes)),
        crate::platform::semantic_id::encode_hex(&bytes[48..80])
    );
    let module = "source-nominal-probe";
    let (_, program) = probes(module);
    let entry = program
        .function(function(module, "read-nominal"))
        .unwrap()
        .0 as usize;
    let payload = input::export(program, entry).unwrap();
    let word = |at| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
    eprintln!(
        "selfhost validator normalized capacity functions={} nodes={} type_rows={} input_bytes={}",
        word(92),
        word(100),
        word(84),
        payload.len()
    );
    assert_eq!(result("read-nominal", &bytes), (0, bytes[48..80].to_vec()));
}
