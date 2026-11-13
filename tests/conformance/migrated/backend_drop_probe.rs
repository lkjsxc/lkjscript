struct Counter<'a> { count: &'a mut i64 }
        impl<'a> Drop for Counter<'a> {
            fn drop(&mut self) { *self.count = *self.count + 1; }
        }
        struct Parent<'a> { child: Counter<'a> }
        fn returning(count: &mut i64) -> i64 {
            let counter = Counter { count };
            return 7;
        }
        fn breaking(count: &mut i64) {
            loop {
                let counter = Counter { count };
                break;
            }
        }
        fn drop_probe() -> i64 {
            let mut count: i64 = 0;
            let value = returning(&mut count);
            breaking(&mut count);
            { let parent = Parent { child: Counter { count: &mut count } }; }
            count + value
        }
    
pub fn probe()->i64 { let mut i=0; while i<1000 { assert_eq!(drop_probe(),10); i+=1; } 10 }
