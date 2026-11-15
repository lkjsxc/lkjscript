// expect: reject E_UNSUPPORTED
// pending: Explain that a static view inside a mixed owned output is unsupported and suggest owning the label as String.
struct Reply { kind: &'static str, text: String }
fn reply(kind: &'static str, text: String) -> Reply { Reply { kind, text } }
fn main() { let value = reply("text/plain", "hello".to_string()); assert!(value.text == "hello"); }
