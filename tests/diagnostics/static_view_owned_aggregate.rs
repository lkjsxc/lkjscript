// expect: reject E_UNSUPPORTED
// A closed private constructor receives an atomic edit set that owns its label as String.
struct Reply { kind: &'static str, text: String }
fn reply(kind: &'static str, text: String) -> Reply { Reply { kind, text } }
fn main() { let value = reply("text/plain", "hello".to_string()); assert!(value.text == "hello"); }
