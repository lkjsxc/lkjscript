// expect: value 42
// witness: {}
#[derive(Copy)]
struct Custom { value: i64 }
impl Clone for Custom {
    fn clone(&self) -> Self { Custom { value: self.value + 1i64 } }
}
#[derive(Copy)]
struct Override { value: i64 }
impl Clone for Override {
    fn clone(&self) -> Self { Override { value: self.value + 2i64 } }
    fn clone_from(&mut self, source: &Self) { self.value = source.value + 3i64; }
}
#[derive(Clone)]
struct Field<T> { value: T, byte: u8 }
#[derive(Clone, Copy)]
struct FixedCopy { value: Custom }
#[derive(Clone, Copy)]
struct GenericCopy<T> { value: T }
fn duplicate<T: Clone>(value: &T) -> T { value.clone() }
fn refresh<T: Clone>(target: &mut T, source: &T) { target.clone_from(source); }
pub fn probe() -> i64 {
    let original = Custom { value: 20i64 };
    let direct = original.clone();
    let generic = duplicate(&original);
    assert!(direct.value == 21i64 && generic.value == 21i64);
    let fields = Field { value: original, byte: 255u8 };
    let fields_copy = fields.clone();
    assert!(fields_copy.value.value == 21i64 && fields_copy.byte == 255u8);
    let fixed = FixedCopy { value: original };
    assert!(fixed.clone().value.value == 20i64);
    let conditional = GenericCopy { value: original };
    assert!(conditional.clone().value.value == 21i64);
    let mut default_target = Custom { value: 0i64 };
    refresh(&mut default_target, &original);
    assert!(default_target.value == 21i64);
    let original_override = Override { value: 20i64 };
    let mut explicit_target = Override { value: 0i64 };
    refresh(&mut explicit_target, &original_override);
    assert!(duplicate(&original_override).value == 22i64 && explicit_target.value == 23i64);
    direct.value + generic.value
}
