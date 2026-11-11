fn copied<T: Clone>(value: &T) -> T { value.clone() }
fn refreshed<T: Clone>(target: &mut T, source: &T) { target.clone_from(source); }
fn same<T: PartialEq>(left: &T, right: &T) -> bool { left.eq(right) }
fn same_operator<T: PartialEq>(left: &T, right: &T) -> bool { *left == *right }

pub fn probe() -> i64 {
    let mut original = "Hi 🌍".to_string();
    let copy = copied(&original);
    original.clear();
    original.push_str("changed");
    let mut target = "a longer destination buffer".to_string();
    refreshed(&mut target, &copy);
    if same(&copy, &target)
        && same_operator(&copy, &target)
        && copy == target
        && !(copy != target)
        && original.ne(&copy)
        && copy.len() == 7usize
        && target.len() == 7usize
    {
        42
    } else {
        0
    }
}
