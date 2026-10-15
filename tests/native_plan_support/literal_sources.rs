const PREFIX: &str = "declarations.begin\n(units\n  (use std builtin)\n";
const SUFFIX: &str = ")\ndeclarations.end\n";

/// Author these immutable fixtures in one supported collection, so all new module
/// names are visible together. Keep every inner byte, including bodies and comments.
pub fn grouped<'a>(sources: impl IntoIterator<Item = &'a str>) -> String {
    let mut output = PREFIX.to_owned();
    let mut count = 0;
    for source in sources {
        let Some(body) = source
            .strip_prefix(PREFIX)
            .and_then(|source| source.strip_suffix(SUFFIX))
        else {
            panic!("grouped fixture requires the exact single standard-import collection");
        };
        assert!(!body.contains("declarations.begin"));
        assert!(!body.contains("declarations.end"));
        assert!(!body.contains("\n  (use "));
        output.push_str(body);
        output.push('\n');
        count += 1;
    }
    assert!(count > 0);
    output.push_str(SUFFIX);
    // The ordinary public request decoder still enforces its unchanged complete
    // request, operation, record, identity and validation-work admissions.
    assert!(output.len() < lkjscript::platform::control::MAXIMUM_COMPACT_INPUT_BYTES);
    output
}
