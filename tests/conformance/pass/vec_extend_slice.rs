struct Item { value: i64, generation: i64 }
impl Item { fn clone(&self) -> Self { Item { value: 0, generation: 99 } } }
impl Clone for Item { fn clone(&self) -> Self { Item { value: self.value, generation: self.generation + 1 } } }
fn append<T: Clone>(target: &mut Vec<T>, source: &[T]) { target.extend_from_slice(source); }
pub fn probe() -> i64 {
    let mut source = vec![Item { value: 20, generation: 0 }, Item { value: 21, generation: 0 }];
    let mut target = vec![Item { value: 1, generation: 0 }];
    append(&mut target, &source);
    let empty: [Item; 0] = [];
    target.extend_from_slice(&empty);
    assert!(target.len() == 3);
    assert!(target[0].generation == 0);
    assert!(target[1].generation == 1 && target[2].generation == 1);
    source[0].value = 90;
    assert!(target[1].value == 20);
    let integers = [20i64, 22i64];
    let mut copied: Vec<i64> = Vec::new();
    copied.extend_from_slice(&integers);
    copied[0] + copied[1]
}
