#[derive(PartialEq, PartialOrd)]
struct Pair {
    first: f64,
    second: f64,
}

#[derive(PartialOrd)]
struct Contrary {
    value: i64,
}
impl PartialEq for Contrary {
    fn eq(&self, other: &Self) -> bool {
        self.value != other.value
    }
}
#[derive(PartialEq, PartialOrd)]
struct Nested {
    first: Contrary,
    last: i64,
}

pub fn probe() -> i64 {
    let zero = 0.0f64;
    let nan = zero / zero;
    let unordered = Pair {
        first: 1.0,
        second: nan,
    };
    let ordered = Pair {
        first: 1.0,
        second: 2.0,
    };
    assert!(!(unordered < ordered));
    assert!(!(unordered <= ordered));
    assert!(!(unordered > ordered));
    assert!(!(unordered >= ordered));
    assert!(
        Pair {
            first: 0.0,
            second: nan
        } < ordered
    );
    assert!(
        Pair {
            first: 2.0,
            second: nan
        } > ordered
    );
    let a = [1.0f64, nan];
    let b = [1.0f64, 2.0f64];
    assert!(!(a < b) && !(a <= b) && !(a > b) && !(a >= b));
    let first = vec![1.0f64, nan];
    let second = vec![1.0f64, 2.0f64];
    assert!(!(first < second) && !(first <= second));
    assert!(!(first > second) && !(first >= second));
    assert!(vec![-0.0f64, 1.0f64] <= vec![0.0f64, 1.0f64]);
    assert!(vec![0.0f64] < vec![-0.0f64, 1.0f64]);
    let left = Nested {
        first: Contrary { value: 1 },
        last: 100,
    };
    let right = Nested {
        first: Contrary { value: 2 },
        last: 0,
    };
    assert!(left < right);
    assert!(vec![Contrary { value: 1 }] < vec![Contrary { value: 2 }]);
    assert!(!(vec![Contrary { value: 2 }] < vec![Contrary { value: 2 }]));
    let a = [Contrary { value: 1 }];
    let b = [Contrary { value: 2 }];
    assert!(a < b);
    42
}
