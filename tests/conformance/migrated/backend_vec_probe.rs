#[derive(Copy, Clone)]
        struct Inner { x: i64 }
        #[derive(Copy, Clone)]
        struct Outer { inner: Inner }
        fn copy_probe() -> i64 {
            let mut a = Outer { inner: Inner { x: 7 } };
            let b = a;
            a.inner.x = 99;
            b.inner.x
        }
        fn vec_probe() -> i64 {
            let mut values: Vec<i64> = Vec::new();
            let mut i: i64 = 0;
            while i < 1000 {
                values.push(i);
                i = i + 1;
            }
            values[999] + values.len() as i64
        }
    
pub fn probe()->i64 { let mut i=0; while i<1000 { assert_eq!(vec_probe(),1999); i+=1; } 1999 }
