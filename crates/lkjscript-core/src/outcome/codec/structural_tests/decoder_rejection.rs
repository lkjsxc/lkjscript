#![allow(clippy::expect_used)]

use super::*;

#[test]
fn structural_decoder_rejects_descriptor_payload_and_enum_corruption() {
    let string = value(
        20,
        StructuralKind::String,
        SemanticPayload::String(b"ok".to_vec()),
    );
    let bytes = encoded(string);
    for (offset, replacement) in [(18, 99), (19, 99)] {
        let mut malformed = bytes.clone();
        malformed[offset] = replacement;
        assert!(decode_execution_outcome(&malformed, 2 * 1024 * 1024).is_err());
    }
    let mut backreference = bytes.clone();
    backreference[19] = 8;
    assert!(decode_execution_outcome(&backreference, 2 * 1024 * 1024).is_err());
    let mut wrong_kind = bytes.clone();
    wrong_kind[18] = 5;
    assert!(decode_execution_outcome(&wrong_kind, 2 * 1024 * 1024).is_err());
    let mut zero_layout = bytes.clone();
    zero_layout[2..10].fill(0);
    assert!(decode_execution_outcome(&zero_layout, 2 * 1024 * 1024).is_err());
    let mut invalid_utf8 = bytes;
    invalid_utf8[24] = 255;
    assert!(decode_execution_outcome(&invalid_utf8, 2 * 1024 * 1024).is_err());
    let mut invalid_path = encoded(value(
        22,
        StructuralKind::Path,
        SemanticPayload::Path(b"/ok".to_vec()),
    ));
    invalid_path[24] = b'x';
    assert!(decode_execution_outcome(&invalid_path, 2 * 1024 * 1024).is_err());

    let enumeration = value(
        21,
        StructuralKind::Enum,
        SemanticPayload::Enum {
            tag: 0,
            active_payload: Vec::new(),
        },
    );
    let mut duplicate_active_section = encoded(enumeration);
    duplicate_active_section[20] = 2;
    assert!(decode_execution_outcome(&duplicate_active_section, 2 * 1024 * 1024).is_err());
}
