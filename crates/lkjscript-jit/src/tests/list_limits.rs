use super::*;

#[test]
fn generated_list_equality_accepts_exact_limit_and_rejects_limit_plus_one() {
    let mut heap = GcHeap::default();
    let mut at_limit = Value::EMPTY_LIST;
    for _ in 0..MAX_LIST_EQUAL_STEPS {
        at_limit = heap
            .alloc(HeapObj::Pair {
                car: Value::UNIT,
                cdr: at_limit,
            })
            .expect("list allocation");
    }
    assert_eq!(
        super::list_values_equal(&heap, at_limit, at_limit, MAX_LIST_EQUAL_STEPS),
        Ok(true)
    );
    let over_limit = heap
        .alloc(HeapObj::Pair {
            car: Value::UNIT,
            cdr: at_limit,
        })
        .expect("over-limit list allocation");
    assert_eq!(
        super::list_values_equal(&heap, over_limit, over_limit, MAX_LIST_EQUAL_STEPS),
        Err("list-equal step limit exceeded".into())
    );
}
