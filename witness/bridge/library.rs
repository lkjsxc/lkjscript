//! Bootstrap source-library byte provisioning. Owned compilation takes these as ordinary inputs.
use lkjscript::source::SourceInput;

/// Explicit bootstrap library inputs. Callers include these files in their bundle;
/// parse_bundle never substitutes bytes that are absent from that bundle.
pub fn standard_library() -> Vec<SourceInput> {
    [
        ("$std/lib.lkj", include_str!("../../library/lib.lkj")),
        ("$std/prelude.lkj", include_str!("../../library/prelude.lkj")),
        ("$std/sequence.lkj", include_str!("../../library/sequence.lkj")),
        ("$std/buffer.lkj", include_str!("../../library/buffer.lkj")),
        ("$std/timers.lkj", include_str!("../../library/timers.lkj")),
        ("$std/json.lkj", include_str!("../../library/json.lkj")),
        ("$std/hmac.lkj", include_str!("../../library/hmac.lkj")),
        ("$std/time.lkj", include_str!("../../library/time.lkj")),
        ("$std/text.lkj", include_str!("../../library/text.lkj")),
        ("$std/text_lower.lkj", include_str!("../../library/text_lower.lkj")),
        ("$std/bytes.lkj", include_str!("../../library/bytes.lkj")),
        ("$std/string.lkj", include_str!("../../library/string.lkj")),
        ("$std/str.lkj", include_str!("../../library/str.lkj")),
        ("$std/sha256.lkj", include_str!("../../library/sha256.lkj")),
        ("$std/char.lkj", include_str!("../../library/char.lkj")),
        ("$std/fmt.lkj", include_str!("../../library/fmt.lkj")),
        ("$std/iter.lkj", include_str!("../../library/iter.lkj")),
        ("$std/integer.lkj", include_str!("../../library/integer.lkj")),
        ("$std/io.lkj", include_str!("../../library/io.lkj")),
        (
            "$std/ordered_map.lkj",
            include_str!("../../library/ordered_map.lkj"),
        ),
        (
            "$std/ordered_set.lkj",
            include_str!("../../library/ordered_set.lkj"),
        ),
    ]
    .into_iter()
    .map(|(path, text)| SourceInput {
        path: path.to_string(),
        text: text.to_string(),
    })
    .collect()
}
