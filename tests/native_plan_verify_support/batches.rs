use crate::components::Native;
use lkjscript::platform::json::{JsonLimits, decode_strict};
use serde_json::{Value, json};
use std::error::Error;
use std::io::{self, Write};
use std::ops::Range;

const PACKING_BYTES: usize = 900_000;
type PackingResult<T> = Result<T, Box<dyn Error>>;

pub struct Batch {
    pub arguments: std::path::PathBuf,
    pub expected: Value,
    pub count: usize,
    pub bytes: usize,
}

#[derive(Clone, Copy)]
struct Place {
    index: usize,
    side: &'static str,
    scope: &'static str,
}
impl Place {
    fn sample(index: usize, side: &'static str) -> Self {
        Self {
            index,
            side,
            scope: "sample",
        }
    }
    fn batch(self) -> Self {
        Self {
            scope: "batch starting at sample",
            ..self
        }
    }
    fn error(self, message: impl std::fmt::Display) -> Box<dyn Error> {
        format!("{} {} {}: {message}", self.scope, self.index, self.side).into()
    }
    fn add(self, left: usize, right: usize) -> PackingResult<usize> {
        left.checked_add(right)
            .ok_or_else(|| self.error("JSON size accounting overflow"))
    }
}

#[derive(Clone, Copy)]
struct Cost {
    bytes: usize,
}
impl Cost {
    fn append(self, value: Self, comma: bool, place: Place) -> PackingResult<Self> {
        Ok(Self {
            bytes: place.add(self.bytes, place.add(value.bytes, usize::from(comma))?)?,
        })
    }
    fn fits(self, limits: JsonLimits) -> bool {
        self.bytes <= limits.maximum_bytes
    }
    fn require(self, place: Place, limits: JsonLimits) -> PackingResult<()> {
        if self.bytes > limits.maximum_bytes {
            return Err(place.error(format!(
                "wrapped JSON has {} bytes; limit {}",
                self.bytes, limits.maximum_bytes
            )));
        }
        Ok(())
    }
}

fn require_strings(value: &Value, place: Place, limits: JsonLimits) -> PackingResult<()> {
    match value {
        Value::Array(values) => {
            for value in values {
                require_strings(value, place, limits)?;
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                if key.len() > limits.maximum_string_bytes {
                    return Err(place.error("JSON key exceeds the string-byte limit"));
                }
                require_strings(value, place, limits)?;
            }
        }
        Value::String(value) if value.len() > limits.maximum_string_bytes => {
            return Err(place.error("JSON text exceeds the string-byte limit"));
        }
        _ => {}
    }
    Ok(())
}

// Use the real serializer for escaping and numeric spelling, stopping before
// oversized output is retained. The same writer counts without storing copies.
struct BoundedWriter<W> {
    inner: W,
    bytes: usize,
    maximum: usize,
}
impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .checked_add(bytes.len())
            .filter(|next| *next <= self.maximum)
            .ok_or_else(|| io::Error::other("serialized JSON exceeds the byte limit"))?;
        self.inner.write_all(bytes)?;
        self.bytes = next;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
fn serialized<W: Write>(
    value: &Value,
    inner: W,
    place: Place,
    limits: JsonLimits,
) -> PackingResult<BoundedWriter<W>> {
    let mut writer = BoundedWriter {
        inner,
        bytes: 0,
        maximum: limits.maximum_bytes,
    };
    serde_json::to_writer(&mut writer, value).map_err(|error| place.error(error))?;
    Ok(writer)
}
fn measure(value: &Value, place: Place, limits: JsonLimits) -> PackingResult<Cost> {
    require_strings(value, place, limits)?;
    let writer = serialized(value, io::sink(), place, limits)?;
    Ok(Cost {
        bytes: writer.bytes,
    })
}

fn ranges(samples: &[(Value, Value)]) -> PackingResult<Vec<Range<usize>>> {
    let limits = JsonLimits::default();
    let empty_input = Cost { bytes: 4 }; // [[]]
    let empty_output = Cost { bytes: 2 }; // []
    let (mut start, mut input, mut output) = (0, empty_input, empty_output);
    let mut selected = Vec::new();
    for (index, (argument, expected)) in samples.iter().enumerate() {
        let input_place = Place::sample(index, "input");
        let output_place = Place::sample(index, "output");
        let argument = measure(argument, input_place, limits)?;
        let expected = measure(expected, output_place, limits)?;
        let single_input = empty_input.append(argument, false, input_place)?;
        let single_output = empty_output.append(expected, false, output_place)?;
        single_input.require(input_place, limits)?;
        single_output.require(output_place, limits)?;
        let nonempty = index > start;
        let next_input = input.append(argument, nonempty, input_place)?;
        let next_output = output.append(expected, nonempty, output_place)?;
        if nonempty
            && (next_input.bytes > PACKING_BYTES
                || !next_input.fits(limits)
                || !next_output.fits(limits))
        {
            selected.push(start..index);
            start = index;
            (input, output) = (single_input, single_output);
        } else {
            (input, output) = (next_input, next_output);
        }
    }
    if start < samples.len() {
        selected.push(start..samples.len());
    }
    Ok(selected)
}

fn admitted(value: &Value, place: Place) -> PackingResult<(String, Cost)> {
    let place = place.batch();
    let limits = JsonLimits::default();
    require_strings(value, place, limits)?;
    let writer = serialized(value, Vec::new(), place, limits)?;
    let decoded = decode_strict(&writer.inner, limits).map_err(|error| {
        place.error(format!(
            "batch strict JSON {}: {}",
            error.code, error.message
        ))
    })?;
    if decoded != *value {
        return Err(place.error("batch strict JSON changed its value"));
    }
    let text = String::from_utf8(writer.inner).map_err(|error| place.error(error))?;
    Ok((
        text,
        Cost {
            bytes: writer.bytes,
        },
    ))
}

struct Materialized {
    arguments: String,
    expected: Value,
    input: Cost,
}
fn materialize(samples: &[(Value, Value)], start: usize) -> PackingResult<Materialized> {
    let arguments = Value::Array(vec![Value::Array(
        samples.iter().map(|(input, _)| input.clone()).collect(),
    )]);
    let expected = Value::Array(samples.iter().map(|(_, output)| output.clone()).collect());
    let (arguments, input) = admitted(&arguments, Place::sample(start, "input"))?;
    admitted(&expected, Place::sample(start, "output"))?;
    Ok(Materialized {
        arguments,
        expected,
        input,
    })
}

pub fn batches(
    public: &Native,
    prefix: &str,
    samples: &[(Value, Value)],
) -> Result<Vec<Batch>, Box<dyn Error>> {
    ranges(samples)?
        .into_iter()
        .enumerate()
        .map(|(index, range)| {
            let batch = materialize(&samples[range.clone()], range.start)?;
            Ok(Batch {
                arguments: public.input(&format!("{prefix}-{index:03}.json"), &batch.arguments),
                expected: batch.expected,
                count: range.len(),
                bytes: batch.input.bytes,
            })
        })
        .collect()
}

#[test]
fn batching_preserves_large_cardinality_and_order() -> PackingResult<()> {
    let samples = vec![
        (json!(vec![0; 131072]), json!(1)),
        (json!(vec![0; 131072]), json!(2)),
        (json!(null), json!(3)),
    ];
    let selected = ranges(&samples)?;
    assert_eq!(selected, [0..3]);
    let batch = materialize(&samples, 0)?;
    assert!(batch.arguments.len() < PACKING_BYTES);
    assert_eq!(batch.expected, json!([1, 2, 3]));
    assert_eq!(
        selected.into_iter().flatten().collect::<Vec<_>>(),
        [0, 1, 2]
    );
    Ok(())
}

#[test]
fn batching_retains_byte_case_and_empty_bounds_independently() -> PackingResult<()> {
    assert!(ranges(&[])?.is_empty());
    let empty = materialize(&[], 0)?;
    assert_eq!((empty.arguments.as_str(), empty.input.bytes), ("[[]]", 4));
    assert_eq!(empty.expected, json!([]));
    let samples = vec![(json!("x".repeat(600000)), json!(null)); 2];
    assert_eq!(ranges(&samples)?, [0..1, 1..2]);
    let samples: Vec<_> = (0..257)
        .map(|index| (json!(index), json!(3 * index + 5)))
        .collect();
    assert_eq!(
        ranges(&samples)?.as_slice(),
        std::slice::from_ref(&(0..257))
    );
    let batch = materialize(&samples, 0)?;
    let arguments = decode_strict(batch.arguments.as_bytes(), JsonLimits::default())?;
    assert_eq!(arguments[0], json!((0..257).collect::<Vec<_>>()));
    assert_eq!(
        batch.expected,
        json!((0..257).map(|index| 3 * index + 5).collect::<Vec<_>>())
    );
    assert_eq!(batch.expected[128], json!(389));
    Ok(())
}

#[test]
fn batching_splits_on_output_bytes_and_keeps_late_rejection_and_recovery() -> PackingResult<()> {
    let large = json!("x".repeat(8192));
    let samples: Vec<_> = (0..130)
        .map(|index| {
            (
                json!(index),
                match index {
                    0..=127 => large.clone(),
                    128 => json!("rejected"),
                    _ => json!("verified"),
                },
            )
        })
        .collect();
    let selected = ranges(&samples)?;
    assert_eq!(selected, [0..127, 127..130]);
    assert_eq!(
        selected.into_iter().flatten().collect::<Vec<_>>(),
        (0..130).collect::<Vec<_>>()
    );
    let batch = materialize(&samples[127..], 127)?;
    assert_eq!(batch.arguments, "[[127,128,129]]");
    assert_eq!(batch.expected, json!([large, "rejected", "verified"]));
    Ok(())
}

#[test]
fn batching_uses_exact_escaped_bytes_and_inclusive_single_pair_byte_bounds() -> PackingResult<()> {
    let escaped = json!("\"\\\0日本語");
    let samples = [
        (escaped.clone(), escaped),
        (json!(i64::MIN), json!(i64::MAX)),
        (json!(i64::MAX), json!(i64::MIN)),
    ];
    assert_eq!(ranges(&samples)?.as_slice(), std::slice::from_ref(&(0..3)));
    let batch = materialize(&samples, 0)?;
    assert_eq!(
        batch.arguments,
        r#"[["\"\\\u0000日本語",-9223372036854775808,9223372036854775807]]"#
    );
    assert_eq!(
        serde_json::to_string(&batch.expected)?,
        r#"["\"\\\u0000日本語",9223372036854775807,-9223372036854775808]"#
    );
    let maximum = JsonLimits::default().maximum_bytes;
    let exact = [(
        json!("x".repeat(maximum - 6)),
        json!("y".repeat(maximum - 4)),
    )];
    assert_eq!(ranges(&exact)?.as_slice(), std::slice::from_ref(&(0..1)));
    let batch = materialize(&exact, 0)?;
    assert_eq!(batch.input.bytes, maximum);
    assert_eq!(
        admitted(&batch.expected, Place::sample(0, "output"))?
            .1
            .bytes,
        maximum
    );
    for (side, pair) in [
        ("input", (json!("x".repeat(maximum - 5)), json!(null))),
        ("output", (json!(null), json!("y".repeat(maximum - 3)))),
    ] {
        let samples = [(json!(null), json!(null)), pair];
        let Err(error) = ranges(&samples) else {
            return Err(format!("oversized {side} pair was admitted").into());
        };
        assert!(error.to_string().starts_with(&format!("sample 1 {side}:")));
        assert!(error.to_string().contains("1048577 bytes"));
    }
    Ok(())
}

#[test]
fn batching_admits_deep_finite_containers_with_exact_wrapping() -> PackingResult<()> {
    fn nested(count: usize) -> Value {
        (0..count).fold(Value::Null, |value, _| Value::Array(vec![value]))
    }
    let exercise = || -> PackingResult<()> {
        for depth in [128, 512, 2048] {
            let samples = [(nested(depth), nested(depth + 1))];
            assert_eq!(ranges(&samples)?.as_slice(), std::slice::from_ref(&(0..1)));
            let batch = materialize(&samples, 7)?;
            let (output, cost) = admitted(&batch.expected, Place::sample(7, "output"))?;
            let exact = format!("{}null{}", "[".repeat(depth + 2), "]".repeat(depth + 2));
            assert_eq!(batch.arguments, exact);
            assert_eq!(output, exact);
            assert_eq!(batch.input.bytes, exact.len());
            assert_eq!(cost.bytes, exact.len());
            let truncated = &exact.as_bytes()[..exact.len() - 1];
            let error = decode_strict(truncated, JsonLimits::default())
                .expect_err("unterminated batch JSON must fail");
            assert_eq!(error.code, "json_decode");
        }
        Ok(())
    };
    lkjscript::platform::stack::run("batch-container-lifecycle", || {
        exercise().map_err(|error| error.to_string())
    })??;
    Ok(())
}
