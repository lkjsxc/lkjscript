use std::fmt;
use std::fmt::Write;
struct P {
    x: i64,
}
impl fmt::Display for P {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "P({})", self.x)
    }
}
fn value(trace: &mut i64, n: i64) -> i64 {
    *trace = *trace * 10 + n;
    n
}
pub fn probe() -> i64 {
    assert!(format!("{:00}", Width).len() == 4usize);
    assert!(format!("{}", Width).len() == 6usize);
    assert!(format!("{:8}", Width).len() == 4usize);
    let name = "水";
    let p = P { x: 42 };
    let s = format!("{name}:{p}:{}:{:04}:{:04x}:{{}}", true, -7i64, 42u8);
    assert!(s.as_bytes().len() == 27usize);
    let mut trace = 0i64;
    let t = format!("{1}:{0}:{1}", value(&mut trace, 1), value(&mut trace, 2));
    assert!(trace == 12);
    assert!(t.len() == 5usize);
    let mut written = String::new();
    write!(&mut written, "{}", p).unwrap();
    assert!(written.len() == 5usize);
    42
}

struct Width;
impl fmt::Display for Width {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match f.width() {
            Some(0) => f.write_str("zero"),
            Some(_) => f.write_str("some"),
            None => f.write_str("absent"),
        }
    }
}
