// expect: value 42
use std::text::{trim, lowercase_chars};
pub fn probe() -> i64 {
    assert!(trim("") == "");
    assert!(trim("plain") == "plain");
    assert!(trim("\u{a0}\u{2003}\t cat \u{202f}\r\n") == "cat");
    assert!(trim("\u{85}\u{1680}\u{2000}\u{200a}\u{2028}\u{2029}\u{205f}\u{3000}") == "");
    assert!(trim("  cat\u{a0}dog  ") == "cat\u{a0}dog");
    assert!(trim("\u{200b}word\u{feff}") == "\u{200b}word\u{feff}");
    assert!(trim("\u{a0}🐈\u{a0}") == "🐈");
    assert!(lowercase_chars("ASCII read FILE") == "ascii read file");
    assert!(lowercase_chars("K") == "k");
    assert!(lowercase_chars("İ") == "i\u{307}");
    assert!(lowercase_chars("ẞ É") == "ß é");
    // This API is scalar mapping: unlike contextual string lowercase, no final sigma.
    assert!(lowercase_chars("ΟΣ Σ") == "οσ σ");
    assert!(lowercase_chars("\u{10400}\u{10427}") == "\u{10428}\u{1044f}");
    assert!(lowercase_chars("cat🐈 42") == "cat🐈 42");
    42
}
