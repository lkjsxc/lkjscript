fn sum(values: &[i64]) -> i64 {
            let mut i: usize = 0;
            let mut total: i64 = 0;
            while i < values.len() {
                total = total + values[i];
                i = i + 1;
            }
            total
        }
        fn array_probe() -> i64 {
            let mut original: [i64; 3] = [1, 2, 3];
            let copied = original;
            {
                let view: &mut [i64] = &mut original;
                view[1] = 99;
            }
            assert_eq!(copied[1], 2);
            sum(&original) + sum(&copied)
        }

pub fn probe()->i64 { let mut i=0; while i<1000 { assert_eq!(array_probe(),109); i+=1; } 109 }
