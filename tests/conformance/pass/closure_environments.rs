// Independent source probes for static callable and capture contracts.
mod case_0 {
    fn identity<T>(value: T) -> T {
        value
    }
    fn twice<F: Fn() -> i64>(value: F) -> i64 {
        value() + value()
    }
    pub fn probe() -> i64 {
        let first: i64 = 20;
        let second: i64 = 22;
        let read = identity(|| first + second);
        twice(read) - 42
    }
}
mod case_1 {
    fn twice<F: FnMut() -> i64>(mut value: F) -> i64 {
        value();
        value()
    }
    pub fn probe() -> i64 {
        let mut first: i64 = 10;
        let mut second: i64 = 20;
        twice(|| {
            first = first + 2;
            second = second + 4;
            first + second
        })
    }
}
mod case_2 {
    fn once<F: FnOnce() -> i64>(value: F) -> i64 {
        value()
    }
    pub fn probe() -> i64 {
        let owned = Box::new(42i64);
        once(move || {
            let transferred = owned;
            *transferred
        })
    }
}
mod case_3 {
    fn apply<F: Fn(i64) -> i64>(f: F, x: i64) -> i64 {
        f(x)
    }
    pub fn probe() -> i64 {
        apply(|x| x + 1, 41)
    }
}
mod case_4 {
    pub fn probe() -> i64 {
        let f = |x: i64| x + 1;
        f(20) + f(20)
    }
}
mod case_5 {
    pub fn probe() -> i64 {
        let a = 20i64;
        let b = 22i64;
        let f = || {
            let g = || a + b;
            g()
        };
        f()
    }
}
mod case_6 {
    pub fn probe() -> i64 {
        let x = 20i64;
        let f = || {
            let first = x;
            let x = 22i64;
            let g = || x;
            first + g()
        };
        f()
    }
}
mod case_7 {
    pub fn probe() -> i64 {
        let mut pair = (20i64, 20i64);
        let mut f = || {
            pair.0 = pair.0 + 1;
            pair.0
        };
        pair.1 = 21;
        f() + pair.1
    }
}
mod case_8 {
    pub fn probe() -> i64 {
        let mut value = 40i64;
        let r = &mut value;
        let mut f = || {
            *r = *r + 1;
            *r
        };
        f();
        f()
    }
}
mod case_9 {
    pub fn probe() -> i64 {
        let x = 42i64;
        let r = &x;
        let f = move || r;
        let output = f();
        drop(f);
        *output
    }
}
mod case_10 {
    pub fn probe() -> i64 {
        let x = 42i64;
        let f = || &x;
        let output = f();
        drop(f);
        *output
    }
}
mod case_11 {
    pub fn probe() -> i64 {
        let x = 42i64;
        let r = &x;
        let f = || r;
        let out = f();
        drop(f);
        *out
    }
}
mod case_12 {
    pub fn probe() -> i64 {
        let mut x = 41i64;
        let r = &mut x;
        let mut f = || {
            let q = &r;
            let y = **q;
            *r = y + 1;
            *r
        };
        f()
    }
}
mod case_13 {
    fn apply<F: Fn((i64, i64)) -> i64>(f: F) -> i64 {
        f((20, 22))
    }
    pub fn probe() -> i64 {
        apply(|(a, b)| a + b)
    }
}
mod case_14 {
    fn apply<F: Fn(&i64) -> i64>(f: F) -> i64 {
        f(&42)
    }
    pub fn probe() -> i64 {
        apply(|&value| value)
    }
}
mod case_15 {
    struct Pair {
        a: i64,
        b: i64,
    }
    fn apply<F: Fn(Pair) -> i64>(f: F) -> i64 {
        f(Pair { a: 20, b: 22 })
    }
    pub fn probe() -> i64 {
        apply(|Pair { a, b }| a + b)
    }
}
mod case_16 {
    fn apply<A, B, F: FnOnce(A) -> B>(f: F, a: A) -> B {
        f(a)
    }
    pub fn probe() -> i64 {
        apply(|x: i64| x + 1, 41)
    }
}
mod case_17 {
    fn call<F: Fn() -> i64>(f: &F) -> i64 {
        f()
    }
    pub fn probe() -> i64 {
        let x = 42i64;
        let f = || x;
        call(&f)
    }
}
mod case_18 {
    fn call<F: FnMut() -> i64>(f: &mut F) -> i64 {
        f()
    }
    pub fn probe() -> i64 {
        let mut x = 41i64;
        let mut f = || {
            x = x + 1;
            x
        };
        call(&mut f)
    }
}
mod case_19 {
    fn invoke<F: FnOnce() -> i64>(f: F) -> i64 {
        f()
    }
    pub fn probe() -> i64 {
        let box_value = Box::new(42i64);
        let f = move || *box_value;
        invoke(&f)
    }
}
mod case_20 {
    fn invoke<F: FnMut() -> i64>(mut f: F) -> i64 {
        f();
        f()
    }
    pub fn probe() -> i64 {
        let mut value = 40i64;
        let mut f = || {
            value = value + 1;
            value
        };
        invoke(&mut f)
    }
}
mod case_21 {
    pub fn probe() -> i64 {
        let mut first = 20i64;
        let mut second = 20i64;
        let a = &mut first;
        let b = &mut second;
        let f = move || (a, b);
        let mut pair = f();
        *pair.0 = *pair.0 + 1;
        *pair.1 = *pair.1 + 1;
        *pair.0 + *pair.1
    }
}
mod slice_result {
    pub fn probe() -> i64 {
        let mut values: Vec<i64> = Vec::new();
        values.push(20);
        values.push(22);
        let view = || values.as_slice();
        let result = view();
        result[0] + result[1]
    }
}
pub fn probe() -> i64 {
    if slice_result::probe() != 42 {
        return 99;
    }
    if case_0::probe() != 42 {
        return 1;
    }
    if case_1::probe() != 42 {
        return 2;
    }
    if case_2::probe() != 42 {
        return 3;
    }
    if case_3::probe() != 42 {
        return 4;
    }
    if case_4::probe() != 42 {
        return 5;
    }
    if case_5::probe() != 42 {
        return 6;
    }
    if case_6::probe() != 42 {
        return 7;
    }
    if case_7::probe() != 42 {
        return 8;
    }
    if case_8::probe() != 42 {
        return 9;
    }
    if case_9::probe() != 42 {
        return 10;
    }
    if case_10::probe() != 42 {
        return 11;
    }
    if case_11::probe() != 42 {
        return 12;
    }
    if case_12::probe() != 42 {
        return 13;
    }
    if case_13::probe() != 42 {
        return 14;
    }
    if case_14::probe() != 42 {
        return 15;
    }
    if case_15::probe() != 42 {
        return 16;
    }
    if case_16::probe() != 42 {
        return 17;
    }
    if case_17::probe() != 42 {
        return 18;
    }
    if case_18::probe() != 42 {
        return 19;
    }
    if case_19::probe() != 42 {
        return 20;
    }
    if case_20::probe() != 42 {
        return 21;
    }
    if case_21::probe() != 42 {
        return 22;
    }
    42
}
