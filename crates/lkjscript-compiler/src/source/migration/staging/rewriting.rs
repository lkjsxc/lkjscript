use std::collections::BTreeMap;

use crate::source::SourceFile;

use super::ConversionInsertion;
use crate::source::edition::EDITION_MARKER;

pub(super) const CONVERSION_OPEN: &str = "f64-from-i64-rounded/\n";
pub(super) const CONVERSION_CLOSE: &str = "\n/f64-from-i64-rounded";

pub(in crate::source::migration) fn replacement(
    file_index: usize,
    file: &SourceFile,
    conversions: &[ConversionInsertion],
) -> String {
    let (marker_offset, marker) = insertion(file);
    let mut before: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    let mut after: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    before.entry(marker_offset).or_default().push(marker);
    for site in conversions.iter().filter(|site| site.file == file_index) {
        before.entry(site.start).or_default().push(CONVERSION_OPEN);
        after.entry(site.end).or_default().push(CONVERSION_CLOSE);
    }
    let mut positions: Vec<_> = before.keys().chain(after.keys()).copied().collect();
    positions.sort_unstable();
    positions.dedup();
    let added = marker.len()
        + conversions
            .iter()
            .filter(|site| site.file == file_index)
            .count()
            * (CONVERSION_OPEN.len() + CONVERSION_CLOSE.len());
    let mut result = String::with_capacity(file.exact_source.len().saturating_add(added));
    let mut cursor = 0;
    for position in positions {
        result.push_str(&file.exact_source[cursor..position]);
        if let Some(values) = after.get(&position) {
            for value in values.iter().rev() {
                result.push_str(value);
            }
        }
        if let Some(values) = before.get(&position) {
            for value in values {
                result.push_str(value);
            }
        }
        cursor = position;
    }
    result.push_str(&file.exact_source[cursor..]);
    result
}

pub(in crate::source::migration) fn insertion(file: &SourceFile) -> (usize, &'static str) {
    let offset = file.syntax.first().map_or(file.exact_source.len(), |node| {
        node.span.start().byte() as usize
    });
    if offset > 0 && file.exact_source.as_bytes().get(offset - 1) != Some(&b'\n') {
        (offset, "\nedition/\n2\n/edition\n")
    } else {
        (offset, EDITION_MARKER)
    }
}
