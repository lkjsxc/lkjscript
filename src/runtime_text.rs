//! Persistent runtime representation for exact bounded UTF-8 text.
//!
//! Tree shape, pieces, priorities, backing allocation, and derived indexes are deliberately
//! unobservable. Public values serialize as one canonical UTF-8 string. A flat materialization
//! remains the independent correctness oracle for every operation.

use crate::schema::{MAXIMUM_TEXT_BYTES, TextString, TextStringTooLarge};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use std::sync::{Arc, OnceLock};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const TARGET_CHUNK_BYTES: usize = 4 * 1024;
const MAXIMUM_TREE_DEPTH: usize = 64;
const MINIMUM_COMPACTION_PIECES: usize = 64;
const PIECE_MULTIPLIER: usize = 4;
const MAXIMUM_CACHED_GRAPHEME_BOUNDARIES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeTextError {
    TooLarge,
    ByteRange,
    ScalarRange,
    Utf8Boundary,
    LineRange,
}

impl fmt::Display for RuntimeTextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TooLarge => "text exceeds UTF-8 byte policy",
            Self::ByteRange => "text byte range is out of bounds",
            Self::ScalarRange => "Unicode scalar index is out of bounds",
            Self::Utf8Boundary => "text byte range does not use UTF-8 boundaries",
            Self::LineRange => "logical line index is out of bounds",
        })
    }
}

impl std::error::Error for RuntimeTextError {}

impl From<TextStringTooLarge> for RuntimeTextError {
    fn from(_: TextStringTooLarge) -> Self {
        Self::TooLarge
    }
}

#[derive(Clone, Debug)]
struct Piece {
    backing: Arc<str>,
    start: usize,
    end: usize,
}

impl Piece {
    fn new(backing: Arc<str>, start: usize, end: usize) -> Result<Self, RuntimeTextError> {
        if start > end || end > backing.len() {
            return Err(RuntimeTextError::ByteRange);
        }
        if !backing.is_char_boundary(start) || !backing.is_char_boundary(end) {
            return Err(RuntimeTextError::Utf8Boundary);
        }
        Ok(Self {
            backing,
            start,
            end,
        })
    }

    fn text(&self) -> &str {
        &self.backing[self.start..self.end]
    }

    fn len(&self) -> usize {
        self.end - self.start
    }

    fn prefix(&self, length: usize) -> Result<Self, RuntimeTextError> {
        Self::new(self.backing.clone(), self.start, self.start + length)
    }

    fn suffix(&self, offset: usize) -> Result<Self, RuntimeTextError> {
        Self::new(self.backing.clone(), self.start + offset, self.end)
    }
}

#[derive(Debug)]
struct Node {
    left: Option<Arc<Node>>,
    piece: Piece,
    right: Option<Arc<Node>>,
    priority: u64,
    bytes: usize,
    scalars: usize,
    newlines: usize,
    pieces: usize,
    depth: usize,
}

type NodeSplit = (Option<Arc<Node>>, Option<Arc<Node>>);

impl Node {
    fn new(
        left: Option<Arc<Self>>,
        piece: Piece,
        right: Option<Arc<Self>>,
        priority: u64,
    ) -> Arc<Self> {
        let text = piece.text();
        Arc::new(Self {
            bytes: total_bytes(&left) + piece.len() + total_bytes(&right),
            scalars: total_scalars(&left) + text.chars().count() + total_scalars(&right),
            newlines: total_newlines(&left)
                + text
                    .as_bytes()
                    .iter()
                    .filter(|byte| **byte == b'\n')
                    .count()
                + total_newlines(&right),
            pieces: total_pieces(&left) + 1 + total_pieces(&right),
            depth: 1 + tree_depth(&left).max(tree_depth(&right)),
            left,
            piece,
            right,
            priority,
        })
    }
}

#[derive(Debug)]
struct RuntimeTextInner {
    root: Option<Arc<Node>>,
    next_nonce: u64,
    flat: OnceLock<Arc<str>>,
    grapheme_boundaries: OnceLock<Option<Arc<[u32]>>>,
}

/// An exact immutable UTF-8 value backed by an unobservable persistent piece tree.
#[derive(Clone)]
pub struct RuntimeText(Arc<RuntimeTextInner>);

impl Default for RuntimeText {
    fn default() -> Self {
        Self::from_shared(Arc::from(""))
    }
}

impl fmt::Debug for RuntimeText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeText")
            .field("bytes", &self.len_bytes())
            .field("scalars", &self.scalar_count())
            .field("lines", &self.line_count())
            .finish_non_exhaustive()
    }
}

impl PartialEq for RuntimeText {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.len_bytes() == other.len_bytes()
                && self.materialized().as_bytes() == other.materialized().as_bytes())
    }
}

impl Eq for RuntimeText {}

impl Serialize for RuntimeText {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.materialized())
    }
}

impl<'de> Deserialize<'de> for RuntimeText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;
        impl de::Visitor<'_> for Visitor {
            type Value = RuntimeText;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("bounded UTF-8 text")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                RuntimeText::try_from_str(value).map_err(E::custom)
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}

impl RuntimeText {
    pub fn try_from_str(value: &str) -> Result<Self, RuntimeTextError> {
        if value.len() > MAXIMUM_TEXT_BYTES {
            return Err(RuntimeTextError::TooLarge);
        }
        Ok(Self::from_shared(Arc::from(value)))
    }

    pub fn from_text(value: &TextString) -> Self {
        Self::from_shared(value.shared())
    }

    pub fn to_text_string(&self) -> Result<TextString, RuntimeTextError> {
        TextString::from_shared(self.materialized()).map_err(Into::into)
    }

    pub fn len_bytes(&self) -> usize {
        total_bytes(&self.0.root)
    }

    pub fn is_empty(&self) -> bool {
        self.0.root.is_none()
    }

    pub fn scalar_count(&self) -> usize {
        total_scalars(&self.0.root)
    }

    pub fn line_count(&self) -> usize {
        total_newlines(&self.0.root).saturating_add(1)
    }

    pub fn grapheme_count(&self) -> usize {
        match self.cached_grapheme_boundaries() {
            Some(boundaries) => boundaries.len().saturating_sub(1),
            None => self.materialized().graphemes(true).count(),
        }
    }

    pub fn is_char_boundary(&self, offset: usize) -> bool {
        if offset == self.len_bytes() {
            return true;
        }
        let Some((piece, local)) = piece_at(&self.0.root, offset) else {
            return false;
        };
        piece.text().is_char_boundary(local)
    }

    pub fn byte_at(&self, offset: usize) -> Option<u8> {
        let (piece, local) = piece_at(&self.0.root, offset)?;
        piece.text().as_bytes().get(local).copied()
    }

    pub fn scalar_at(&self, index: usize) -> Result<u32, RuntimeTextError> {
        scalar_at(&self.0.root, index).ok_or(RuntimeTextError::ScalarRange)
    }

    pub fn slice_bytes(&self, start: usize, end: usize) -> Result<Self, RuntimeTextError> {
        self.validate_range(start, end)?;
        let mut nonce = self.0.next_nonce;
        let (_, tail) = split(self.0.root.clone(), start, &mut nonce)?;
        let (selected, _) = split(tail, end - start, &mut nonce)?;
        Ok(Self::from_root(selected, nonce).normalized())
    }

    pub fn splice_bytes(
        &self,
        start: usize,
        end: usize,
        replacement: &Self,
    ) -> Result<Self, RuntimeTextError> {
        self.validate_range(start, end)?;
        let result_bytes = self
            .len_bytes()
            .checked_sub(end - start)
            .and_then(|value| value.checked_add(replacement.len_bytes()))
            .ok_or(RuntimeTextError::TooLarge)?;
        if result_bytes > MAXIMUM_TEXT_BYTES {
            return Err(RuntimeTextError::TooLarge);
        }
        let mut nonce = self.0.next_nonce;
        let (left, tail) = split(self.0.root.clone(), start, &mut nonce)?;
        let (_, right) = split(tail, end - start, &mut nonce)?;
        let middle = rekey(&replacement.0.root, &mut nonce);
        Ok(Self::from_root(merge(merge(left, middle), right), nonce).normalized())
    }

    pub fn concat(&self, rhs: &Self) -> Result<Self, RuntimeTextError> {
        self.splice_bytes(self.len_bytes(), self.len_bytes(), rhs)
    }

    pub fn previous_grapheme_boundary(&self, offset: usize) -> Result<usize, RuntimeTextError> {
        if offset > self.len_bytes() {
            return Err(RuntimeTextError::ByteRange);
        }
        if !self.is_char_boundary(offset) {
            return Err(RuntimeTextError::Utf8Boundary);
        }
        if let Some(boundaries) = self.cached_grapheme_boundaries() {
            let offset = u32::try_from(offset).map_err(|_| RuntimeTextError::TooLarge)?;
            return Ok(match boundaries.binary_search(&offset) {
                Ok(0) | Err(0) => 0,
                Ok(index) | Err(index) => boundaries[index - 1] as usize,
            });
        }
        let mut previous = 0;
        for (boundary, _) in self.materialized().grapheme_indices(true) {
            if boundary >= offset {
                break;
            }
            previous = boundary;
        }
        Ok(previous)
    }

    pub fn next_grapheme_boundary(&self, offset: usize) -> Result<usize, RuntimeTextError> {
        if offset > self.len_bytes() {
            return Err(RuntimeTextError::ByteRange);
        }
        if !self.is_char_boundary(offset) {
            return Err(RuntimeTextError::Utf8Boundary);
        }
        if let Some(boundaries) = self.cached_grapheme_boundaries() {
            let offset = u32::try_from(offset).map_err(|_| RuntimeTextError::TooLarge)?;
            let index = match boundaries.binary_search(&offset) {
                Ok(index) => index.saturating_add(1),
                Err(index) => index,
            };
            return Ok(boundaries
                .get(index)
                .copied()
                .map_or(self.len_bytes(), |value| value as usize));
        }
        for (boundary, _) in self.materialized().grapheme_indices(true) {
            if boundary > offset {
                return Ok(boundary);
            }
        }
        Ok(self.len_bytes())
    }

    pub fn line_start_byte(&self, line: usize) -> Result<usize, RuntimeTextError> {
        if line >= self.line_count() {
            return Err(RuntimeTextError::LineRange);
        }
        if line == 0 {
            return Ok(0);
        }
        nth_newline(&self.0.root, line - 1)
            .map(|offset| offset + 1)
            .ok_or(RuntimeTextError::LineRange)
    }

    pub fn line_end_byte(&self, line: usize) -> Result<usize, RuntimeTextError> {
        if line >= self.line_count() {
            return Err(RuntimeTextError::LineRange);
        }
        let mut end = nth_newline(&self.0.root, line).unwrap_or_else(|| self.len_bytes());
        if end > 0 && self.byte_at(end - 1) == Some(b'\r') {
            end -= 1;
        }
        Ok(end)
    }

    pub fn byte_to_line(&self, offset: usize) -> Result<usize, RuntimeTextError> {
        if offset > self.len_bytes() {
            return Err(RuntimeTextError::ByteRange);
        }
        Ok(newlines_before(&self.0.root, offset))
    }

    pub fn find_forward(
        &self,
        query: &Self,
        start: usize,
    ) -> Result<Option<usize>, RuntimeTextError> {
        if start > self.len_bytes() {
            return Err(RuntimeTextError::ByteRange);
        }
        if !self.is_char_boundary(start) {
            return Err(RuntimeTextError::Utf8Boundary);
        }
        if query.is_empty() {
            return Ok(Some(start));
        }
        Ok(self.materialized()[start..]
            .find(&*query.materialized())
            .map(|offset| start + offset))
    }

    pub fn find_backward(
        &self,
        query: &Self,
        end: usize,
    ) -> Result<Option<usize>, RuntimeTextError> {
        if end > self.len_bytes() {
            return Err(RuntimeTextError::ByteRange);
        }
        if !self.is_char_boundary(end) {
            return Err(RuntimeTextError::Utf8Boundary);
        }
        if query.is_empty() {
            return Ok(Some(end));
        }
        Ok(self.materialized()[..end].rfind(&*query.materialized()))
    }

    /// Returns 0 for no terminators, 1 for LF, 2 for CRLF, and 3 for mixed or lone CR.
    pub fn line_ending_kind(&self) -> u8 {
        let bytes = self.materialized();
        let bytes = bytes.as_bytes();
        let mut lf = false;
        let mut crlf = false;
        let mut lone_cr = false;
        for (index, byte) in bytes.iter().copied().enumerate() {
            match byte {
                b'\n' if index > 0 && bytes[index - 1] == b'\r' => crlf = true,
                b'\n' => lf = true,
                b'\r' if bytes.get(index + 1) != Some(&b'\n') => lone_cr = true,
                _ => {}
            }
        }
        match (lf, crlf, lone_cr) {
            (false, false, false) => 0,
            (true, false, false) => 1,
            (false, true, false) => 2,
            _ => 3,
        }
    }

    pub fn display_width(
        &self,
        start: usize,
        end: usize,
        initial_column: usize,
        tab_width: usize,
    ) -> Result<usize, RuntimeTextError> {
        self.validate_range(start, end)?;
        if tab_width == 0 || tab_width > 64 {
            return Err(RuntimeTextError::ByteRange);
        }
        let selected = self.slice_bytes(start, end)?.materialized();
        let mut column = initial_column;
        for grapheme in selected.graphemes(true) {
            let width = grapheme_display_width(grapheme, column, tab_width);
            column = column
                .checked_add(width)
                .ok_or(RuntimeTextError::TooLarge)?;
        }
        column
            .checked_sub(initial_column)
            .ok_or(RuntimeTextError::ByteRange)
    }

    pub fn cell_prefix_boundary(
        &self,
        start: usize,
        end: usize,
        initial_column: usize,
        maximum_cells: usize,
        tab_width: usize,
    ) -> Result<(usize, usize), RuntimeTextError> {
        self.validate_range(start, end)?;
        if tab_width == 0 || tab_width > 64 {
            return Err(RuntimeTextError::ByteRange);
        }
        let limit = initial_column
            .checked_add(maximum_cells)
            .ok_or(RuntimeTextError::TooLarge)?;
        let materialized = self.materialized();
        let selected = &materialized[start..end];
        let mut column = initial_column;
        let mut boundary = start;
        let mut inspected = 0;
        for (offset, grapheme) in selected.grapheme_indices(true) {
            inspected = offset.saturating_add(grapheme.len());
            let width = grapheme_display_width(grapheme, column, tab_width);
            let next = column
                .checked_add(width)
                .ok_or(RuntimeTextError::TooLarge)?;
            if next > limit {
                break;
            }
            column = next;
            boundary = start.saturating_add(inspected);
        }
        Ok((boundary, inspected))
    }

    pub fn materialized(&self) -> Arc<str> {
        self.0
            .flat
            .get_or_init(|| {
                let mut output = String::with_capacity(self.len_bytes());
                append_text(&self.0.root, &mut output);
                Arc::from(output)
            })
            .clone()
    }

    fn from_shared(value: Arc<str>) -> Self {
        let (root, nonce) = canonical_root(value.clone());
        let flat = OnceLock::new();
        let _ = flat.set(value);
        Self(Arc::new(RuntimeTextInner {
            root,
            next_nonce: nonce,
            flat,
            grapheme_boundaries: OnceLock::new(),
        }))
    }

    fn from_root(root: Option<Arc<Node>>, next_nonce: u64) -> Self {
        Self(Arc::new(RuntimeTextInner {
            root,
            next_nonce,
            flat: OnceLock::new(),
            grapheme_boundaries: OnceLock::new(),
        }))
    }

    fn validate_range(&self, start: usize, end: usize) -> Result<(), RuntimeTextError> {
        if start > end || end > self.len_bytes() {
            return Err(RuntimeTextError::ByteRange);
        }
        if !self.is_char_boundary(start) || !self.is_char_boundary(end) {
            return Err(RuntimeTextError::Utf8Boundary);
        }
        Ok(())
    }

    fn normalized(self) -> Self {
        let bytes = self.len_bytes();
        let ideal = bytes.div_ceil(TARGET_CHUNK_BYTES).max(1);
        let piece_limit = ideal
            .saturating_mul(PIECE_MULTIPLIER)
            .max(MINIMUM_COMPACTION_PIECES);
        if total_pieces(&self.0.root) <= piece_limit
            && tree_depth(&self.0.root) <= MAXIMUM_TREE_DEPTH
        {
            return self;
        }
        Self::from_shared(self.materialized())
    }

    fn cached_grapheme_boundaries(&self) -> Option<&Arc<[u32]>> {
        self.0
            .grapheme_boundaries
            .get_or_init(|| {
                let text = self.materialized();
                let mut boundaries = Vec::new();
                for (offset, _) in text.grapheme_indices(true) {
                    if boundaries.len() == MAXIMUM_CACHED_GRAPHEME_BOUNDARIES {
                        return None;
                    }
                    boundaries.push(u32::try_from(offset).ok()?);
                }
                if boundaries.last().copied() != u32::try_from(text.len()).ok() {
                    if boundaries.len() == MAXIMUM_CACHED_GRAPHEME_BOUNDARIES {
                        return None;
                    }
                    boundaries.push(u32::try_from(text.len()).ok()?);
                }
                Some(Arc::from(boundaries))
            })
            .as_ref()
    }

    #[cfg(test)]
    fn piece_count(&self) -> usize {
        total_pieces(&self.0.root)
    }

    #[cfg(test)]
    fn depth(&self) -> usize {
        tree_depth(&self.0.root)
    }
}

fn grapheme_display_width(grapheme: &str, column: usize, tab_width: usize) -> usize {
    if grapheme == "\t" {
        tab_width - (column % tab_width)
    } else if grapheme.chars().all(|value| value.is_control()) {
        grapheme.chars().count().saturating_mul(2)
    } else {
        UnicodeWidthStr::width(grapheme)
    }
}

fn total_bytes(node: &Option<Arc<Node>>) -> usize {
    node.as_ref().map_or(0, |node| node.bytes)
}

fn total_scalars(node: &Option<Arc<Node>>) -> usize {
    node.as_ref().map_or(0, |node| node.scalars)
}

fn total_newlines(node: &Option<Arc<Node>>) -> usize {
    node.as_ref().map_or(0, |node| node.newlines)
}

fn total_pieces(node: &Option<Arc<Node>>) -> usize {
    node.as_ref().map_or(0, |node| node.pieces)
}

fn tree_depth(node: &Option<Arc<Node>>) -> usize {
    node.as_ref().map_or(0, |node| node.depth)
}

fn priority(nonce: u64) -> u64 {
    let mut value = nonce.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn singleton(piece: Piece, nonce: &mut u64) -> Option<Arc<Node>> {
    if piece.len() == 0 {
        return None;
    }
    *nonce = nonce.saturating_add(1);
    Some(Node::new(None, piece, None, priority(*nonce)))
}

fn merge(left: Option<Arc<Node>>, right: Option<Arc<Node>>) -> Option<Arc<Node>> {
    match (left, right) {
        (None, other) | (other, None) => other,
        (Some(left), Some(right)) if left.priority >= right.priority => Some(Node::new(
            left.left.clone(),
            left.piece.clone(),
            merge(left.right.clone(), Some(right)),
            left.priority,
        )),
        (Some(left), Some(right)) => Some(Node::new(
            merge(Some(left), right.left.clone()),
            right.piece.clone(),
            right.right.clone(),
            right.priority,
        )),
    }
}

fn split(
    root: Option<Arc<Node>>,
    offset: usize,
    nonce: &mut u64,
) -> Result<NodeSplit, RuntimeTextError> {
    let bytes = total_bytes(&root);
    if offset > bytes {
        return Err(RuntimeTextError::ByteRange);
    }
    if offset == 0 {
        return Ok((None, root));
    }
    if offset == bytes {
        return Ok((root, None));
    }
    let root = root.ok_or(RuntimeTextError::ByteRange)?;
    let left_bytes = total_bytes(&root.left);
    if offset < left_bytes {
        let (left, middle) = split(root.left.clone(), offset, nonce)?;
        return Ok((
            left,
            Some(Node::new(
                middle,
                root.piece.clone(),
                root.right.clone(),
                root.priority,
            )),
        ));
    }
    let piece_end = left_bytes + root.piece.len();
    if offset > piece_end {
        let (middle, right) = split(root.right.clone(), offset - piece_end, nonce)?;
        return Ok((
            Some(Node::new(
                root.left.clone(),
                root.piece.clone(),
                middle,
                root.priority,
            )),
            right,
        ));
    }
    let local = offset - left_bytes;
    if !root.piece.text().is_char_boundary(local) {
        return Err(RuntimeTextError::Utf8Boundary);
    }
    let left_piece = singleton(root.piece.prefix(local)?, nonce);
    let right_piece = singleton(root.piece.suffix(local)?, nonce);
    Ok((
        merge(root.left.clone(), left_piece),
        merge(right_piece, root.right.clone()),
    ))
}

fn rekey(root: &Option<Arc<Node>>, nonce: &mut u64) -> Option<Arc<Node>> {
    let mut pieces = Vec::with_capacity(total_pieces(root));
    collect_pieces(root, &mut pieces);
    pieces
        .into_iter()
        .fold(None, |tree, piece| merge(tree, singleton(piece, nonce)))
}

fn canonical_root(backing: Arc<str>) -> (Option<Arc<Node>>, u64) {
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < backing.len() {
        let mut end = (start + TARGET_CHUNK_BYTES).min(backing.len());
        while end > start && !backing.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            end = backing[start..]
                .char_indices()
                .nth(1)
                .map_or(backing.len(), |(offset, _)| start + offset);
        }
        ranges.push((start, end));
        start = end;
    }
    let mut nonce = 0;
    let root = ranges.into_iter().try_fold(None, |root, (start, end)| {
        let piece = Piece::new(backing.clone(), start, end).ok()?;
        Some(merge(root, singleton(piece, &mut nonce)))
    });
    (root.flatten(), nonce)
}

fn collect_pieces(root: &Option<Arc<Node>>, output: &mut Vec<Piece>) {
    if let Some(root) = root {
        collect_pieces(&root.left, output);
        output.push(root.piece.clone());
        collect_pieces(&root.right, output);
    }
}

fn append_text(root: &Option<Arc<Node>>, output: &mut String) {
    if let Some(root) = root {
        append_text(&root.left, output);
        output.push_str(root.piece.text());
        append_text(&root.right, output);
    }
}

fn piece_at(root: &Option<Arc<Node>>, offset: usize) -> Option<(Piece, usize)> {
    let root = root.as_ref()?;
    let left = total_bytes(&root.left);
    if offset < left {
        piece_at(&root.left, offset)
    } else if offset < left + root.piece.len() {
        Some((root.piece.clone(), offset - left))
    } else {
        piece_at(&root.right, offset - left - root.piece.len())
    }
}

fn scalar_at(root: &Option<Arc<Node>>, index: usize) -> Option<u32> {
    let root = root.as_ref()?;
    let left = total_scalars(&root.left);
    if index < left {
        return scalar_at(&root.left, index);
    }
    let own = root.piece.text().chars().count();
    if index < left + own {
        return root.piece.text().chars().nth(index - left).map(u32::from);
    }
    scalar_at(&root.right, index - left - own)
}

fn nth_newline(root: &Option<Arc<Node>>, ordinal: usize) -> Option<usize> {
    let root = root.as_ref()?;
    let left_newlines = total_newlines(&root.left);
    if ordinal < left_newlines {
        return nth_newline(&root.left, ordinal);
    }
    let piece_ordinal = ordinal - left_newlines;
    let mut seen = 0;
    for (offset, byte) in root.piece.text().as_bytes().iter().copied().enumerate() {
        if byte == b'\n' {
            if seen == piece_ordinal {
                return Some(total_bytes(&root.left) + offset);
            }
            seen += 1;
        }
    }
    nth_newline(&root.right, piece_ordinal - seen)
        .map(|offset| total_bytes(&root.left) + root.piece.len() + offset)
}

fn newlines_before(root: &Option<Arc<Node>>, offset: usize) -> usize {
    let Some(root) = root else {
        return 0;
    };
    let left_bytes = total_bytes(&root.left);
    if offset <= left_bytes {
        return newlines_before(&root.left, offset);
    }
    let within = (offset - left_bytes).min(root.piece.len());
    let own = root.piece.text().as_bytes()[..within]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count();
    if offset <= left_bytes + root.piece.len() {
        total_newlines(&root.left) + own
    } else {
        total_newlines(&root.left)
            + total_newlines(&Some(root.clone()))
                .saturating_sub(total_newlines(&root.left) + total_newlines(&root.right))
            + newlines_before(&root.right, offset - left_bytes - root.piece.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splice_slice_lines_and_graphemes_match_flat_utf8() {
        let source = RuntimeText::try_from_str("a\r\nCafe\u{301}\n界z").expect("source");
        assert_eq!(source.len_bytes(), source.materialized().len());
        assert_eq!(source.scalar_count(), source.materialized().chars().count());
        assert_eq!(source.line_count(), 3);
        assert_eq!(source.scalar_at(0), Ok(u32::from('a')));
        assert_eq!(source.scalar_at(3), Ok(u32::from('C')));
        assert_eq!(
            source.scalar_at(source.scalar_count()),
            Err(RuntimeTextError::ScalarRange)
        );
        assert_eq!(source.line_ending_kind(), 3);
        assert_eq!(source.line_start_byte(1), Ok(3));
        assert_eq!(source.line_end_byte(0), Ok(1));
        assert_eq!(source.byte_to_line(source.len_bytes()), Ok(2));

        let combining = source.materialized().find("e\u{301}").expect("combining");
        assert_eq!(source.next_grapheme_boundary(combining), Ok(combining + 3));
        assert_eq!(
            source.previous_grapheme_boundary(combining + 3),
            Ok(combining)
        );

        let replacement = RuntimeText::try_from_str("λ").expect("replacement");
        let edited = source
            .splice_bytes(combining, combining + 3, &replacement)
            .expect("splice");
        assert_eq!(&*edited.materialized(), "a\r\nCafλ\n界z");
        assert_eq!(
            &*edited.slice_bytes(3, 8).expect("slice").materialized(),
            "Cafλ"
        );
    }

    #[test]
    fn cell_prefix_boundary_stops_before_the_first_nonfitting_grapheme() {
        let source = RuntimeText::try_from_str("\u{200b}ab界\tz").expect("source");
        let through_b = source.materialized().find('界').expect("wide boundary");
        let (boundary, inspected) = source
            .cell_prefix_boundary(0, source.len_bytes(), 0, 2, 4)
            .expect("cell prefix");
        assert_eq!(boundary, through_b);
        assert!(inspected > boundary);

        let tab = RuntimeText::try_from_str("\ta").expect("tab text");
        assert_eq!(
            tab.cell_prefix_boundary(0, tab.len_bytes(), 1, 3, 4),
            Ok((1, 2))
        );
        assert_eq!(
            tab.cell_prefix_boundary(0, tab.len_bytes(), 0, 2, 4),
            Ok((0, 1))
        );
    }

    #[test]
    fn invalid_and_one_over_inputs_reject_without_partial_values() {
        let source = RuntimeText::try_from_str("aλb").expect("source");
        let empty = RuntimeText::default();
        assert_eq!(
            source.slice_bytes(2, 3),
            Err(RuntimeTextError::Utf8Boundary)
        );
        assert_eq!(
            source.splice_bytes(0, source.len_bytes() + 1, &empty),
            Err(RuntimeTextError::ByteRange)
        );
        let one_over = "x".repeat(MAXIMUM_TEXT_BYTES + 1);
        assert_eq!(
            RuntimeText::try_from_str(&one_over),
            Err(RuntimeTextError::TooLarge)
        );
    }

    #[test]
    fn concatenated_non_ascii_pieces_preserve_every_utf8_boundary() {
        let base = RuntimeText::try_from_str("A").expect("base");
        let lambda = RuntimeText::try_from_str("λ").expect("lambda");
        let combining = RuntimeText::try_from_str("\u{301}").expect("combining");
        let replacement = lambda.concat(&combining).expect("combined grapheme");
        assert_eq!(&*replacement.materialized(), "λ\u{301}");
        assert_eq!(replacement.len_bytes(), 4);
        for boundary in [0, 2, 4] {
            assert!(replacement.is_char_boundary(boundary));
        }
        let edited = base
            .splice_bytes(base.len_bytes(), base.len_bytes(), &replacement)
            .expect("splice combined grapheme");
        assert_eq!(&*edited.materialized(), "Aλ\u{301}");
        assert_eq!(edited.display_width(0, edited.len_bytes(), 0, 4), Ok(2));
    }

    #[test]
    fn repeated_local_edits_are_bounded_and_exact() {
        let mut text = RuntimeText::try_from_str(&"a".repeat(1024 * 1024)).expect("text");
        let replacement = RuntimeText::try_from_str("λ").expect("replacement");
        let middle = text.len_bytes() / 2;
        for _ in 0..1_000 {
            text = text
                .splice_bytes(middle, middle, &replacement)
                .expect("edit");
        }
        assert_eq!(text.len_bytes(), 1024 * 1024 + 2_000);
        assert!(text.depth() <= MAXIMUM_TREE_DEPTH);
        let ideal = text.len_bytes().div_ceil(TARGET_CHUNK_BYTES).max(1);
        assert!(text.piece_count() <= ideal * PIECE_MULTIPLIER.max(1));
        assert_eq!(text.materialized().chars().count(), 1024 * 1024 + 1_000);
    }

    #[test]
    fn randomized_splices_match_the_canonical_flat_oracle() {
        const SEED: u64 = 0x6c6b_6a65_6469_7434;
        const CASES: usize = 2_000;
        const INSERTIONS: [&str; 8] = ["", "a", "λ", "e\u{301}", "界", "\n", "\r\n", "\t"];

        fn next(state: &mut u64) -> u64 {
            *state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            *state
        }

        fn boundaries(value: &str) -> Vec<usize> {
            value
                .char_indices()
                .map(|(offset, _)| offset)
                .chain(std::iter::once(value.len()))
                .collect()
        }

        let mut state = SEED;
        let mut flat = String::from("seed\r\nλe\u{301}\n界");
        let mut persistent = RuntimeText::try_from_str(&flat).expect("initial text");
        for _ in 0..CASES {
            let points = boundaries(&flat);
            let left = usize::try_from(next(&mut state)).expect("rng") % points.len();
            let right = usize::try_from(next(&mut state)).expect("rng") % points.len();
            let (start_index, end_index) = if left <= right {
                (left, right)
            } else {
                (right, left)
            };
            let start = points[start_index];
            let end = points[end_index];
            let insertion =
                INSERTIONS[usize::try_from(next(&mut state)).expect("rng") % INSERTIONS.len()];
            let replacement = RuntimeText::try_from_str(insertion).expect("replacement");

            flat.replace_range(start..end, insertion);
            persistent = persistent
                .splice_bytes(start, end, &replacement)
                .expect("persistent splice");

            assert_eq!(&*persistent.materialized(), flat);
            assert_eq!(persistent.len_bytes(), flat.len());
            assert_eq!(persistent.scalar_count(), flat.chars().count());
            assert_eq!(persistent.grapheme_count(), flat.graphemes(true).count());
            assert_eq!(
                persistent.line_count(),
                flat.as_bytes()
                    .iter()
                    .filter(|byte| **byte == b'\n')
                    .count()
                    + 1
            );
            assert_eq!(
                persistent.find_forward(&RuntimeText::try_from_str("a").expect("query"), 0),
                Ok(flat.find('a'))
            );
            assert_eq!(
                persistent
                    .find_backward(&RuntimeText::try_from_str("a").expect("query"), flat.len()),
                Ok(flat.rfind('a'))
            );
        }
        eprintln!("runtime-text-differential seed={SEED:#018x} cases={CASES}");
    }
}
