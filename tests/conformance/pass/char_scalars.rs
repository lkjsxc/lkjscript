#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct C { value: char }
fn all(value:char)->i64 {match value {'\0'..='\u{d7ff}'=>1,'\u{e000}'..='\u{10ffff}'=>2}}
pub fn probe()->i64 {
let c='水'; let d='😀';let chars=vec![c,d];assert!(chars[0]<chars[1]);assert!(all(c)==1 && all(d)==2);
assert!(C{value:'a'}==C{value:'a'});let mut text=String::new();text.push('A');text.push('é');text.push(c);text.push(d);
assert!(text.as_bytes().len()==10);assert!(c.len_utf8()==3 && d.len_utf16()==2);assert!('Z'.to_ascii_lowercase()=='z');assert!('\u{3000}'.is_whitespace());
(c as u32 == 27700u32) as i64 + (41u8 as char as u32) as i64
}
pub fn character()->char {'水'}
