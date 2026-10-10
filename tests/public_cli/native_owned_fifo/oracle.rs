use super::*;
use std::collections::VecDeque;

pub(super) fn action(kind: i64, value: i64) -> Value {
    json!({"kind":kind,"value":value})
}

pub(super) fn expected(actions: &[Value], mode: usize) -> Value {
    assert!(mode < 3, "unknown independent carrier mode");
    let encode = |value: i64| match mode {
        0 => value,
        1 => value * 255 + 255,
        _ => value * 255 + 286,
    };
    let mut queue = VecDeque::new();
    let events: Vec<Value> = actions.iter().map(|input| {
        let kind = input["kind"].as_i64().unwrap();
        assert!((0..=3).contains(&kind), "invalid independent model operation");
        let observed = match kind {
            0 => {
                let value = encode(input["value"].as_i64().unwrap());
                queue.push_back(value);
                Some(value)
            },
            1 => queue.pop_front(),
            2 => queue.front().copied(),
            _ => { queue.clear(); None },
        };
        json!({"kind":kind,"present":observed.is_some(),"value":observed.unwrap_or(0),"length":queue.len()})
    }).collect();
    json!({"events":events,"remaining":queue,"empty-length":0,"reused":[encode(37)]})
}

pub(super) fn exhaustive() -> Vec<Vec<Value>> {
    let alphabet = [
        action(0, 0),
        action(0, 255),
        action(1, 0),
        action(2, 0),
        action(3, 0),
    ];
    let mut all = vec![vec![]];
    let mut layer = vec![vec![]];
    for _ in 0..4 {
        let mut next = Vec::new();
        for prefix in &layer {
            for operation in &alphabet {
                let mut word = prefix.clone();
                word.push(operation.clone());
                next.push(word);
            }
        }
        all.extend(next.iter().cloned());
        layer = next;
    }
    assert_eq!(all.len(), 781);
    all
}

pub(super) fn burst(length: usize) -> Vec<Value> {
    let mut result: Vec<_> = (0..length)
        .map(|i| action(0, i64::try_from(i % 256).unwrap()))
        .collect();
    for i in 0..length {
        result.push(action(2, 0));
        result.push(action(1, 0));
        if i % 3 == 0 {
            result.push(action(0, 255 - i64::try_from(i % 256).unwrap()));
        }
    }
    result
}

pub(super) fn mixed() -> Vec<Value> {
    let mut state = 0xc39c_6592_u32;
    (0..512)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let kind = match state % 16 {
                0 => 3,
                1..=5 => 1,
                6..=8 => 2,
                _ => 0,
            };
            action(kind, i64::from((state >> 8) & 255))
        })
        .collect()
}

#[test]
fn native_owned_fifo_oracle_distinguishes_lifo_and_nonconsuming_peek() {
    assert_eq!(
        expected(&[action(0, 3), action(0, 5), action(2, 0), action(1, 0)], 0),
        json!({"events":[
            {"kind":0,"present":true,"value":3,"length":1},
            {"kind":0,"present":true,"value":5,"length":2},
            {"kind":2,"present":true,"value":3,"length":2},
            {"kind":1,"present":true,"value":3,"length":1}],
            "remaining":[5],"empty-length":0,"reused":[37]})
    );
    assert_eq!(expected(&[action(0, 255)], 2)["remaining"], json!([65_311]));
}
