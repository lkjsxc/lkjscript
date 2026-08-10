// Every checked operation is bounded by one already-materialized source string; failures would
// violate Rust string/slice invariants rather than describe a semantic source limit.
#![allow(clippy::expect_used)]

use crate::source::{SourcePosition, SourceSpan};

pub(super) struct Line<'a> {
    pub(super) text: &'a str,
    pub(super) start: usize,
    pub(super) content_end: usize,
    pub(super) line: u64,
}

pub(super) fn source_lines(source: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut start = 0_usize;
    let mut number = 1_u64;
    for segment in source.split_inclusive('\n') {
        let segment_start = start;
        start = start
            .checked_add(segment.len())
            .expect("source segments cannot exceed their containing string");
        let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
        let text = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        lines.push(Line {
            text,
            start: segment_start,
            content_end: segment_start + text.len(),
            line: number,
        });
        number = number
            .checked_add(1)
            .expect("source line count cannot exceed its byte length plus one");
    }
    if !source.is_empty() && !source.ends_with('\n') && lines.is_empty() {
        lines.push(Line {
            text: source,
            start: 0,
            content_end: source.len(),
            line: 1,
        });
    }
    lines
}

pub(super) fn line_span(line: &Line<'_>) -> SourceSpan {
    SourceSpan {
        start: SourcePosition {
            byte: byte_offset(line.start),
            line: line.line,
            column: 1,
        },
        end: SourcePosition {
            byte: byte_offset(line.content_end),
            line: line.line,
            column: character_column(line.text),
        },
    }
}

pub(super) fn span_at(lines: &[Line<'_>], start: usize, end: usize) -> SourceSpan {
    SourceSpan {
        start: position_at(lines, start),
        end: position_at(lines, end),
    }
}

fn position_at(lines: &[Line<'_>], byte: usize) -> SourcePosition {
    for line in lines.iter().rev() {
        if byte >= line.start {
            let local_end = byte.min(line.content_end);
            let local_length = local_end
                .checked_sub(line.start)
                .expect("selected source line starts before its byte position");
            let column = character_column(&line.text[..local_length]);
            return SourcePosition {
                byte: byte_offset(byte),
                line: line.line,
                column,
            };
        }
    }
    SourcePosition {
        byte: byte_offset(byte),
        line: 1,
        column: 1,
    }
}

fn byte_offset(value: usize) -> u64 {
    u64::try_from(value).expect("validated source length guarantees u64 byte positions")
}

fn character_column(text: &str) -> u64 {
    u64::try_from(text.chars().count())
        .expect("character count cannot exceed validated source byte length")
        .checked_add(1)
        .expect("validated source length leaves room for the one-based column")
}

#[cfg(test)]
mod tests {
    use super::{line_span, source_lines, span_at, Line};

    #[test]
    fn source_columns_count_unicode_scalars_not_bytes() {
        let lines = source_lines("éx\n");
        let span = span_at(&lines, "é".len(), "éx".len());
        assert_eq!(span.start().line(), 1);
        assert_eq!(span.start().column(), 2);
        assert_eq!(span.end().column(), 3);
    }

    #[test]
    fn synthetic_positions_cross_the_former_u32_boundary_without_aliasing() {
        let start = usize::try_from(u64::from(u32::MAX) + 17).expect("64-bit test host");
        let line = Line {
            text: "x",
            start,
            content_end: start.checked_add(1).expect("test position"),
            line: u64::from(u32::MAX) + 9,
        };
        let span = line_span(&line);
        assert_eq!(span.start().byte(), u64::from(u32::MAX) + 17);
        assert_eq!(span.end().byte(), u64::from(u32::MAX) + 18);
        assert_eq!(span.start().line(), u64::from(u32::MAX) + 9);
        assert_ne!(span.start().byte(), span.end().byte());
    }
}
