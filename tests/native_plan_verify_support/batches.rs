use crate::components::Native;
use serde_json::{Value, json};
use std::ops::Range;

// The existing command reader separately bounds bytes and aggregate JSON entries.
// A container itself is counted by its parent; the document root is not an item.
const MAXIMUM_BYTES: usize = 1_048_576;
const MAXIMUM_ITEMS: usize = 100_000;

pub struct Batch {
    pub arguments: std::path::PathBuf,
    pub expected: Value,
    pub count: usize,
    pub bytes: usize,
    pub items: usize,
}
fn items(value: &Value) -> usize {
    match value {
        Value::Array(values) => values.len() + values.iter().map(items).sum::<usize>(),
        Value::Object(values) => values.len() + values.values().map(items).sum::<usize>(),
        _ => 0,
    }
}
fn ranges(samples: &[(Value, Value)]) -> Vec<Range<usize>> {
    let (mut start, mut bytes, mut count) = (0, 4, 1);
    let mut output = Vec::new();
    for (index, (input, _)) in samples.iter().enumerate() {
        let next_bytes = input.to_string().len() + 1;
        let next_items = 1 + items(input);
        assert!(
            next_bytes + 4 <= MAXIMUM_BYTES,
            "single input exceeds runner byte limit"
        );
        assert!(
            next_items < MAXIMUM_ITEMS,
            "single input exceeds runner JSON item limit"
        );
        if index > start
            && (index - start == 128
                || bytes + next_bytes > 900_000
                || count + next_items > MAXIMUM_ITEMS)
        {
            output.push(start..index);
            (start, bytes, count) = (index, 4, 1);
        }
        bytes += next_bytes;
        count += next_items;
    }
    if start < samples.len() {
        output.push(start..samples.len());
    }
    output
}

pub fn batches(public: &Native, prefix: &str, samples: &[(Value, Value)]) -> Vec<Batch> {
    ranges(samples)
        .into_iter()
        .enumerate()
        .map(|(index, range)| {
            let slice = &samples[range];
            let inputs: Vec<_> = slice.iter().map(|(input, _)| input).collect();
            let outputs: Vec<_> = slice.iter().map(|(_, output)| output).collect();
            let arguments = json!([inputs]);
            let text = arguments.to_string();
            let count = items(&arguments);
            assert!(text.len() <= MAXIMUM_BYTES);
            assert!(count <= MAXIMUM_ITEMS);
            Batch {
                arguments: public.input(&format!("{prefix}-{index:03}.json"), &text),
                expected: json!(outputs),
                count: slice.len(),
                bytes: text.len(),
                items: count,
            }
        })
        .collect()
}

#[test]
fn batching_honors_inclusive_json_items_without_losing_or_reordering_cases() {
    assert_eq!(items(&json!({"a":[1,{"b":null}],"z":[]})), 5);
    let samples = vec![
        (json!(vec![0; 49998]), json!(1)),
        (json!(vec![0; 49999]), json!(2)),
        (json!(null), json!(3)),
    ];
    let selected = ranges(&samples);
    assert_eq!(selected, [0..2, 2..3]);
    let arguments = json!([[samples[0].0, samples[1].0]]);
    assert_eq!(items(&arguments), MAXIMUM_ITEMS);
    assert!(arguments.to_string().len() < 900_000);
    assert_eq!(
        selected.into_iter().flatten().collect::<Vec<_>>(),
        [0, 1, 2]
    );
}

#[test]
fn batching_retains_byte_case_and_empty_bounds_independently() {
    assert!(ranges(&[]).is_empty());
    let samples = vec![(json!("x".repeat(600000)), json!(null)); 2];
    assert_eq!(ranges(&samples), [0..1, 1..2]);
    let samples = vec![(json!(null), json!(null)); 129];
    assert_eq!(ranges(&samples), [0..128, 128..129]);
}
