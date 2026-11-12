pub fn probe() -> i64 { let v = vec![1i64, 2, 3, 4]; v.iter().map(|x| x * 2).filter(|x| *x > 2).sum::<i64>() + 24 }
