trait Measure { fn measure(&self) -> i64; }
struct Item { value: i64 }
impl Measure for Item { fn measure(&self) -> i64 { self.value } }
fn sum<T: Measure>(values: &[T]) -> i64 {
    let mut result = 0i64;
    let mut i = 0usize;
    while i < values.len() { result += values[i].measure(); i += 1; }
    result
}
type Elements<T> = [T];
type Borrowed<'a, T> = &'a Elements<T>;
fn via_alias<'a, T: Measure>(values: Borrowed<'a, T>) -> i64 { sum(values) }
fn replace_first<T>(values: &mut [T], value: T) -> T {
    std::mem::replace(&mut values[0], value)
}
pub fn probe() -> i64 {
    let values = vec![Item { value: 19 }, Item { value: 23 }];
    assert!(sum(&values) == 42);
    assert!(via_alias(&values) == 42);
    let array = [Item { value: 20 }, Item { value: 22 }];
    assert!(sum(&array) == 42 && via_alias(&array) == 42);
    let mut numbers = vec![1i64, 2];
    assert!(replace_first(&mut numbers, 40i64) == 1);
    let mut array = [3i64, 2];
    assert!(replace_first(&mut array, 40i64) == 3);
    let reference = &mut numbers;
    assert!(replace_first(reference, 41i64) == 40);
    assert!(reference[0] == 41);
    numbers[0] + array[1] - 1
}
