// Rust accepts; fixed regions inside slice elements need a collection reference inventory.
type Items<'a> = &'a [&'a i64];
fn test<'a>(input: Items<'a>) {
    let values: Items<'a> = input;
    drop(values);
}
pub fn probe() -> i64 {
    42
}
