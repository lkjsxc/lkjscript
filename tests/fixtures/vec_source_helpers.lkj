// This rustc oracle calls the exact source-library traits explicitly. Ordinary
// Vec/slice syntax elsewhere remains the independent language oracle.
pub fn source_vec_helpers_probe() -> i64 {
    struct Affine(Box<i64>);
    let empty: Vec<Affine> = Vec::new();
    assert!(sequence::VecAccess::is_empty(&empty));
    assert!(sequence::VecAccess::first(&empty).is_none());
    assert!(sequence::VecAccess::get(&empty, 0).is_none());
    let values = vec![Affine(Box::new(20)), Affine(Box::new(22))];
    assert!(!sequence::VecAccess::is_empty(&values));
    assert!(sequence::VecAccess::get(&values, values.len()).is_none());
    assert!(sequence::VecAccess::get(&values, usize::MAX).is_none());
    let first = sequence::VecAccess::first(&values).unwrap();
    let second = sequence::VecAccess::get(&values, 1).unwrap();
    assert_eq!(*first.0, *values.as_slice().first().unwrap().0);
    assert_eq!(*second.0, *values.as_slice().get(1).unwrap().0);

    use std::cell::RefCell;
    struct Compared<'a> {
        value: i64,
        visits: &'a RefCell<Vec<i64>>,
    }
    impl PartialEq for Compared<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.visits.borrow_mut().push(self.value);
            self.value == other.value
        }
        fn ne(&self, _other: &Self) -> bool {
            panic!("contains must use eq")
        }
    }
    let visits = RefCell::new(Vec::new());
    let compared = vec![
        Compared {
            value: 1,
            visits: &visits,
        },
        Compared {
            value: 2,
            visits: &visits,
        },
        Compared {
            value: 3,
            visits: &visits,
        },
    ];
    let needle = Compared {
        value: 2,
        visits: &visits,
    };
    assert!(sequence::VecContains::contains(&compared, &needle));
    assert_eq!(*visits.borrow(), vec![1, 2]);
    visits.borrow_mut().clear();
    assert!(compared.as_slice().contains(&needle));
    assert_eq!(*visits.borrow(), vec![1, 2]);
    visits.borrow_mut().clear();
    let absent = Compared {
        value: 9,
        visits: &visits,
    };
    assert!(!sequence::VecContains::contains(&compared, &absent));
    assert_eq!(*visits.borrow(), vec![1, 2, 3]);
    *first.0 + *second.0
}

pub fn source_vec_extend_probe() -> i64 {
    use std::cell::RefCell;
    struct Item<'a> { value: i64, visits: &'a RefCell<Vec<i64>> }
    impl Item<'_> { fn clone(&self) -> Self { panic!("inherent clone must not replace Clone") } }
    impl Clone for Item<'_> {
        fn clone(&self) -> Self {
            self.visits.borrow_mut().push(self.value);
            Item { value: self.value, visits: self.visits }
        }
    }
    let visits = RefCell::new(Vec::new());
    let source = [Item { value: 20, visits: &visits }, Item { value: 22, visits: &visits }];
    let mut target = Vec::new();
    sequence::VecExtend::extend_from_slice(&mut target, &source);
    assert_eq!(*visits.borrow(), vec![20, 22]);
    sequence::VecExtend::extend_from_slice(&mut target, &[]);
    assert_eq!(*visits.borrow(), vec![20, 22]);
    target[0].value = 19;
    assert_eq!(source[0].value, 20);
    assert_eq!(target[0].value + target[1].value, 41);
    source[0].value + source[1].value
}
