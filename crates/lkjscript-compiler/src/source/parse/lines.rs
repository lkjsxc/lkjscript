use crate::source::{SourcePosition, SourceSpan};

pub(super) struct Line<'a> {
    pub(super) text: &'a str,
    pub(super) start: usize,
    pub(super) content_end: usize,
    pub(super) line: u32,
}

pub(super) fn source_lines(source: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut number = 1_u32;
    for segment in source.split_inclusive('\n') {
        let segment_start = start;
        start += segment.len();
        let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
        let text = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        lines.push(Line {
            text,
            start: segment_start,
            content_end: segment_start + text.len(),
            line: number,
        });
        number = number.saturating_add(1);
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
            byte: line.start as u32,
            line: line.line,
            column: 1,
        },
        end: SourcePosition {
            byte: line.content_end as u32,
            line: line.line,
            column: line.text.chars().count() as u32 + 1,
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
            let column = line.text[..local_end.saturating_sub(line.start)]
                .chars()
                .count() as u32
                + 1;
            return SourcePosition {
                byte: byte as u32,
                line: line.line,
                column,
            };
        }
    }
    SourcePosition {
        byte: byte as u32,
        line: 1,
        column: 1,
    }
}
