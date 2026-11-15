pub fn probe() -> i64 {
    let mut o: Option<Box<i64>> = Some(Box::new(3));
    let r: &i64 = match &o { Some(b) => &**b, None => return 0 };
    o = None;
    *r
}
