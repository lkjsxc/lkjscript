//! Optional independent inventory generator. No Rust code enters execution.
//! Run with rustc tools/debug-unicode-witness.rs -o target/debug-unicode-witness.
fn emit(ranges: &[(u32,u32)], depth: usize) {
    let prefix="    ".repeat(depth);
    if ranges.is_empty() { println!("{prefix}false"); return; }
    let mid=ranges.len()/2;
    let (start,end)=ranges[mid];
    println!("{prefix}if code < {start}u32 {{");
    emit(&ranges[..mid], depth+1);
    println!("{prefix}}} else if code <= {end}u32 {{ true }} else {{");
    emit(&ranges[mid+1..],depth+1);
    println!("{prefix}}}");
}
fn main() {
    let mut ranges=Vec::new();
    let mut begin=None;
    for value in 0u32..=0x110000u32 {
        let escaped=char::from_u32(value).is_some_and(|ch| value >= 0x7f && ch.escape_debug().count()!=1);
        if escaped {
            if begin.is_none() { begin=Some(value); }
        } else if let Some(start)=begin.take() {
            ranges.push((start,value-1));
        }
    }
    println!("//! Unicode {}.{}.{} debug-escape scalar intervals.", std::char::UNICODE_VERSION.0, std::char::UNICODE_VERSION.1,std::char::UNICODE_VERSION.2);
    println!("//! Generated from scalar observations by tools/debug-unicode-witness.rs.");
    println!("//! Unicode Data Files and Software License: https://www.unicode.org/license.txt");
    println!("pub fn escaped(code: u32) -> bool {{");
    println!("    if code < 127u32 {{ return code < 32u32; }}");
    emit(&ranges,1);
    println!("}}");
}
