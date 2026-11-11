//! Bootstrap source-library byte provisioning. Owned compilation takes these as ordinary inputs.
use lkjscript::source::SourceInput;

/// Explicit bootstrap library inputs. Callers include these files in their bundle;
/// parse_bundle never substitutes bytes that are absent from that bundle.
pub fn standard_library() -> Vec<SourceInput> {
    [
        ("$std/lib.rs", include_str!("../library/lib.rs")),
        ("$std/prelude.rs", include_str!("../library/prelude.rs")),
        ("$std/sequence.rs", include_str!("../library/sequence.rs")),
        ("$std/bytes.rs", include_str!("../library/bytes.rs")),
        ("$std/string.rs", include_str!("../library/string.rs")),
        (
            "$std/ordered_map.rs",
            include_str!("../library/ordered_map.rs"),
        ),
    ]
    .into_iter()
    .map(|(path, text)| SourceInput {
        path: path.to_string(),
        text: text.to_string(),
    })
    .collect()
}
