struct Pair { a: Box<i64>, b: i64 }
fn add(value: Result<i64, i64>) -> Result<i64, i64> { let n = value?; Ok(n+2) }
fn option(value: Option<i64>) -> Option<i64> { let n = value?; Some(n+3) }
fn from(value: Option<i64>) -> i64 { let Some(n) = value else { return 4; }; n }
pub fn probe() -> i64 {
    let a = if let Some(n) = Some(10) { n } else { 0 };
    let b = add(Ok(5)).unwrap();
    let c = add(Err(8)).unwrap_err();
    let d = option(Some(2)).unwrap();
    let e = from(None);
    let f = loop { break 9i64; };
    let p = Pair { a: Box::new(7), b: 1 };
    let q = Pair { b: 2, ..p };
    let mut v = vec![1i64,2,3];
    let mut total = 0;
    while let Some(x) = v.pop() { total += x; }
    a+b+c+d+e+f+*q.a+q.b+total
}
