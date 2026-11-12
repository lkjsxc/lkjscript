fn id<T>(x: T) -> T { x }
pub fn probe() -> i64 {
    let r: &i64;
    { let y: i64 = 5; r = id(&y); }
    *r
}
