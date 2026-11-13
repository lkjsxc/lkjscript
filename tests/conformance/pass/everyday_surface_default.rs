struct Marker { number: i64 }
impl Marker { fn default() -> Marker { Marker { number: 99 } } }
impl Default for Marker { fn default() -> Marker { Marker { number: 42 } } }
#[derive(Default)] struct Holder { marker: Marker, text: String, values: Vec<i64>, count: usize }
#[derive(Default)] struct Pair<T> { left: T, right: T }
#[derive(Default)] struct Tuple(i64, bool);
#[derive(Default)] struct Unit;
fn make<T: Default>() -> T { T::default() }
pub fn probe() -> i64 {
    let holder = Holder::default();
    assert_eq!(Marker::default().number, 99);
    assert_eq!(make::<Marker>().number, 42);
    let pair: Pair<i64> = make();
    let tuple = Tuple::default();
    let unit = Unit::default();
    drop(unit);
    assert_eq!(holder.text.len(), 0usize);
    assert_eq!(holder.values.len(), 0usize);
    assert_eq!(holder.count, 0usize);
    assert_eq!(pair.left + pair.right, 0);
    assert_eq!(tuple.0, 0);
    assert!(!tuple.1);
    holder.marker.number
}
