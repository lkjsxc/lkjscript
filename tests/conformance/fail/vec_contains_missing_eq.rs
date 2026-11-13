struct NoEq {
    value: Box<i64>,
}
pub fn probe() -> bool {
    let values = vec![NoEq {
        value: Box::new(42i64),
    }];
    values.contains(&NoEq {
        value: Box::new(42i64),
    })
}
