struct Opaque {}
impl Opaque {
    fn unknown(&self) -> impl Iterator<Item = i64> { unreachable!() }
    fn known(&self) -> i64 { 42 }
}
struct Other {}
impl Other { fn known(&self) -> i64 { 42 } }
fn affected(value: Opaque) -> i64 { let _ = value.unknown(); 42 }
fn independent(value: Other) -> i64 { value.known() + true }
