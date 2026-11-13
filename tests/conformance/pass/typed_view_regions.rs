// Typed views constrain their own paths; original shared descriptors retain their regions.
mod original {
    fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
        let pointer = input;
        let short: &&'a i64 = &pointer;
        drop(short);
        let long: &'b i64 = pointer;
        *long
    }
    pub fn probe() -> i64 {
        let input = 42i64;
        test(&input)
    }
}
mod mixed {
    fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
        let holder = (input, input);
        let short: &(&'a i64, &'b i64) = &holder;
        let long: &'b i64 = short.1;
        *long
    }
    pub fn probe() -> i64 {
        let input = 42i64;
        test(&input)
    }
}
mod shortened {
    fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
        let mut pointer = input;
        {
            let short: &mut &'a i64 = &mut pointer;
            drop(short);
        }
        let value: &'a i64 = pointer;
        *value
    }
    pub fn probe() -> i64 {
        let input = 42i64;
        test(&input)
    }
}
mod equivalent {
    fn test<'a: 'b, 'b: 'a>(mut input: &'b i64) -> i64 {
        let short: &mut &'a i64 = &mut input;
        drop(short);
        *input
    }
    pub fn probe() -> i64 {
        let value = 42i64;
        test(&value)
    }
}
mod loop_copy {
    fn test<'a, 'b: 'a>(input: &'b i64) -> i64 {
        let pointer = input;
        let short: &&'a i64 = &pointer;
        let mut slot = short;
        let mut n = 0usize;
        while n < 1 {
            let copied = slot;
            slot = copied;
            n += 1;
        }
        let allowed: &'a i64 = *slot;
        *allowed
    }
    pub fn probe() -> i64 {
        let input = 42i64;
        test(&input)
    }
}
mod receiver {
    struct Holder {
        value: i64,
    }
    impl Holder {
        fn get<'a, 'b: 'a>(&'b self, _other: &'a i64) -> &i64 {
            &self.value
        }
    }
    pub fn probe() -> i64 {
        let holder = Holder { value: 42 };
        let other = 0i64;
        *holder.get(&other)
    }
}
mod repeated {
    struct Pair<'a, 'b> {
        left: &'a i64,
        right: &'b i64,
    }
    fn test<'a>(input: Pair<'a, 'a>) -> &i64 {
        input.left
    }
    pub fn probe() -> i64 {
        let value = 42i64;
        *test(Pair {
            left: &value,
            right: &value,
        })
    }
}
mod anonymous {
    type Alias<'a> = &'a i64;
    fn identity<T>(value: T) -> T {
        value
    }
    fn test<'b>(input: &'b i64) -> i64 {
        let short = identity::<Alias<'_>>(input);
        let value: &'b i64 = short;
        *value
    }
    pub fn probe() -> i64 {
        let value = 42i64;
        test(&value)
    }
}
mod capture_store {
    fn test<'a>(input: &'a i64, other: &'a i64) -> i64 {
        let mut stored = input;
        let token = Box::new(0i64);
        let act = move || {
            drop(token);
            let first: &'a i64 = stored;
            let value = *first;
            drop(first);
            stored = other;
            let second: &'a i64 = stored;
            value + *second
        };
        act()
    }
    pub fn probe() -> i64 {
        let a = 21i64;
        let b = 21i64;
        test(&a, &b)
    }
}
pub fn probe() -> i64 {
    assert_eq!(original::probe(), 42);
    assert_eq!(mixed::probe(), 42);
    assert_eq!(shortened::probe(), 42);
    assert_eq!(equivalent::probe(), 42);
    assert_eq!(loop_copy::probe(), 42);
    assert_eq!(receiver::probe(), 42);
    assert_eq!(repeated::probe(), 42);
    assert_eq!(anonymous::probe(), 42);
    assert_eq!(capture_store::probe(), 42);
    42
}
