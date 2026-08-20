#![allow(clippy::expect_used)]

use std::cmp::{max, min};

const MAX_BUFFERS: usize = 100;
const MAX_SCALARS_PER_BUFFER: usize = 130_752;
const MAX_UNDO_COMMANDS: usize = 32;
const MAX_UNDO_SCALARS: usize = 2_097_152;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BufferId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Viewport {
    rows: usize,
    columns: usize,
    top_line: usize,
    left_column: usize,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            rows: 1,
            columns: 1,
            top_line: 0,
            left_column: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SearchState {
    pattern: Vec<char>,
    matches: Vec<usize>,
    selected: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EditSnapshot {
    content: Vec<char>,
    cursor: usize,
    anchor: Option<usize>,
    preferred_column: Option<usize>,
    dirty: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Buffer {
    id: BufferId,
    title: String,
    content: Vec<char>,
    cursor: usize,
    anchor: Option<usize>,
    preferred_column: Option<usize>,
    dirty: bool,
    undo: Vec<EditSnapshot>,
    redo: Vec<EditSnapshot>,
    search: Option<SearchState>,
    viewport: Viewport,
}

impl Buffer {
    fn new(id: BufferId, title: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into().chars().collect::<Vec<_>>();
        Self {
            id,
            title: title.into(),
            content,
            cursor: 0,
            anchor: None,
            preferred_column: None,
            dirty: false,
            undo: Vec::new(),
            redo: Vec::new(),
            search: None,
            viewport: Viewport::default(),
        }
    }

    fn text(&self) -> String {
        self.content.iter().collect()
    }

    fn selection(&self) -> Option<(usize, usize)> {
        self.anchor
            .filter(|anchor| *anchor != self.cursor)
            .map(|anchor| (min(anchor, self.cursor), max(anchor, self.cursor)))
    }

    fn snapshot(&self) -> EditSnapshot {
        EditSnapshot {
            content: self.content.clone(),
            cursor: self.cursor,
            anchor: self.anchor,
            preferred_column: self.preferred_column,
            dirty: self.dirty,
        }
    }

    fn restore(&mut self, snapshot: EditSnapshot) {
        self.content = snapshot.content;
        self.cursor = snapshot.cursor;
        self.anchor = snapshot.anchor;
        self.preferred_column = snapshot.preferred_column;
        self.dirty = snapshot.dirty;
        self.search = None;
        self.reveal_cursor();
    }

    fn prepare_edit(&self, replacement_length: usize) -> Result<(), EditorError> {
        let removed = self
            .selection()
            .map_or(0, |(start, end)| end.saturating_sub(start));
        let result_length = self
            .content
            .len()
            .saturating_sub(removed)
            .saturating_add(replacement_length);
        if result_length > MAX_SCALARS_PER_BUFFER {
            return Err(EditorError::ContentLimit);
        }
        let retained = self
            .undo
            .iter()
            .map(|snapshot| snapshot.content.len())
            .sum::<usize>()
            .saturating_add(self.content.len());
        if self.undo.len() >= MAX_UNDO_COMMANDS || retained > MAX_UNDO_SCALARS {
            return Err(EditorError::UndoLimit);
        }
        Ok(())
    }

    fn replace_selection(&mut self, replacement: &str) -> Result<(), EditorError> {
        let replacement = replacement.chars().collect::<Vec<_>>();
        self.prepare_edit(replacement.len())?;
        let prior = self.snapshot();
        let (start, end) = self.selection().unwrap_or((self.cursor, self.cursor));
        self.content.splice(start..end, replacement.iter().copied());
        self.cursor = start + replacement.len();
        self.anchor = None;
        self.preferred_column = None;
        self.dirty = true;
        self.undo.push(prior);
        self.redo.clear();
        self.search = None;
        self.reveal_cursor();
        Ok(())
    }

    fn backspace(&mut self) -> Result<bool, EditorError> {
        if self.selection().is_some() {
            self.replace_selection("")?;
            return Ok(true);
        }
        if self.cursor == 0 {
            self.anchor = None;
            return Ok(false);
        }
        self.anchor = Some(self.cursor - 1);
        self.replace_selection("")?;
        Ok(true)
    }

    fn delete_forward(&mut self) -> Result<bool, EditorError> {
        if self.selection().is_some() {
            self.replace_selection("")?;
            return Ok(true);
        }
        if self.cursor == self.content.len() {
            self.anchor = None;
            return Ok(false);
        }
        self.anchor = Some(self.cursor + 1);
        self.replace_selection("")?;
        Ok(true)
    }

    fn move_horizontal(&mut self, right: bool, select: bool) {
        if !select {
            if let Some((start, end)) = self.selection() {
                self.cursor = if right { end } else { start };
                self.anchor = None;
                self.preferred_column = None;
                self.reveal_cursor();
                return;
            }
            self.anchor = None;
        } else if self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        }
        self.cursor = if right {
            min(self.cursor.saturating_add(1), self.content.len())
        } else {
            self.cursor.saturating_sub(1)
        };
        self.preferred_column = None;
        self.reveal_cursor();
    }

    fn move_line_boundary(&mut self, end: bool, select: bool) {
        if select && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        } else if !select {
            self.anchor = None;
        }
        let (start, finish) = self.line_bounds(self.cursor);
        self.cursor = if end { finish } else { start };
        self.preferred_column = None;
        self.reveal_cursor();
    }

    fn move_vertical(&mut self, down: bool, select: bool) {
        if select && self.anchor.is_none() {
            self.anchor = Some(self.cursor);
        } else if !select {
            self.anchor = None;
        }
        let (line_start, _) = self.line_bounds(self.cursor);
        let column = self
            .preferred_column
            .unwrap_or_else(|| self.cursor.saturating_sub(line_start));
        self.preferred_column = Some(column);
        if down {
            let (_, line_end) = self.line_bounds(self.cursor);
            if line_end == self.content.len() {
                self.cursor = self.content.len();
            } else {
                let next_start = line_end + 1;
                let (_, next_end) = self.line_bounds(next_start);
                self.cursor = min(next_start.saturating_add(column), next_end);
            }
        } else if line_start == 0 {
            self.cursor = min(column, self.line_bounds(0).1);
        } else {
            let previous_end = line_start - 1;
            let (previous_start, _) = self.line_bounds(previous_end);
            self.cursor = min(previous_start.saturating_add(column), previous_end);
        }
        self.reveal_cursor();
    }

    fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.content.len();
        self.preferred_column = None;
        self.reveal_cursor();
    }

    fn undo(&mut self) -> bool {
        let Some(prior) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.restore(prior);
        true
    }

    fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snapshot());
        self.restore(next);
        true
    }

    fn start_search(&mut self, pattern: &str) -> Result<bool, EditorError> {
        let pattern = pattern.chars().collect::<Vec<_>>();
        if pattern.is_empty() {
            return Err(EditorError::EmptySearch);
        }
        let matches = literal_matches(&self.content, &pattern);
        let selected = matches
            .iter()
            .position(|start| *start >= self.cursor)
            .unwrap_or(0);
        self.search = Some(SearchState {
            pattern,
            matches,
            selected,
        });
        self.select_search_match();
        Ok(self
            .search
            .as_ref()
            .is_some_and(|search| !search.matches.is_empty()))
    }

    fn next_match(&mut self, previous: bool) -> bool {
        let Some(search) = &mut self.search else {
            return false;
        };
        if search.matches.is_empty() {
            return false;
        }
        search.selected = if previous {
            search
                .selected
                .checked_sub(1)
                .unwrap_or(search.matches.len() - 1)
        } else {
            (search.selected + 1) % search.matches.len()
        };
        self.select_search_match();
        true
    }

    fn select_search_match(&mut self) {
        let Some(search) = &self.search else {
            return;
        };
        let Some(start) = search.matches.get(search.selected).copied() else {
            return;
        };
        self.anchor = Some(start);
        self.cursor = start + search.pattern.len();
        self.preferred_column = None;
        self.reveal_cursor();
    }

    fn resize(&mut self, rows: usize, columns: usize) -> Result<(), EditorError> {
        if rows == 0 || columns == 0 || rows > 1_000 || columns > 1_000 {
            return Err(EditorError::ViewportLimit);
        }
        self.viewport.rows = rows;
        self.viewport.columns = columns;
        self.reveal_cursor();
        Ok(())
    }

    fn line_bounds(&self, position: usize) -> (usize, usize) {
        let position = min(position, self.content.len());
        let start = self.content[..position]
            .iter()
            .rposition(|scalar| *scalar == '\n')
            .map_or(0, |index| index + 1);
        let end = self.content[position..]
            .iter()
            .position(|scalar| *scalar == '\n')
            .map_or(self.content.len(), |offset| position + offset);
        (start, end)
    }

    fn cursor_line_column(&self) -> (usize, usize) {
        let line = self.content[..self.cursor]
            .iter()
            .filter(|scalar| **scalar == '\n')
            .count();
        let (start, _) = self.line_bounds(self.cursor);
        (line, self.cursor - start)
    }

    fn reveal_cursor(&mut self) {
        let (line, column) = self.cursor_line_column();
        if line < self.viewport.top_line {
            self.viewport.top_line = line;
        } else if line >= self.viewport.top_line.saturating_add(self.viewport.rows) {
            self.viewport.top_line = line.saturating_add(1).saturating_sub(self.viewport.rows);
        }
        if column < self.viewport.left_column {
            self.viewport.left_column = column;
        } else if column
            >= self
                .viewport
                .left_column
                .saturating_add(self.viewport.columns)
        {
            self.viewport.left_column = column
                .saturating_add(1)
                .saturating_sub(self.viewport.columns);
        }
    }
}

fn literal_matches(content: &[char], pattern: &[char]) -> Vec<usize> {
    if pattern.is_empty() || pattern.len() > content.len() {
        return Vec::new();
    }
    (0..=content.len() - pattern.len())
        .filter(|start| content[*start..*start + pattern.len()] == *pattern)
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Editor {
    buffers: Vec<Buffer>,
    active: Option<usize>,
    next_buffer_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CloseDecision {
    Cancel,
    Discard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EditorError {
    BufferLimit,
    MissingBuffer,
    DirtyDecisionRequired,
    ContentLimit,
    UndoLimit,
    EmptySearch,
    ViewportLimit,
}

impl Editor {
    fn new() -> Self {
        Self {
            buffers: Vec::new(),
            active: None,
            next_buffer_id: 1,
        }
    }

    fn create_buffer(
        &mut self,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<BufferId, EditorError> {
        if self.buffers.len() >= MAX_BUFFERS {
            return Err(EditorError::BufferLimit);
        }
        let id = BufferId(self.next_buffer_id);
        self.next_buffer_id = self.next_buffer_id.saturating_add(1);
        self.buffers.push(Buffer::new(id, title, content));
        self.active = Some(self.buffers.len() - 1);
        Ok(id)
    }

    fn active(&self) -> Result<&Buffer, EditorError> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .ok_or(EditorError::MissingBuffer)
    }

    fn active_mut(&mut self) -> Result<&mut Buffer, EditorError> {
        self.active
            .and_then(|index| self.buffers.get_mut(index))
            .ok_or(EditorError::MissingBuffer)
    }

    fn switch(&mut self, id: BufferId) -> Result<(), EditorError> {
        self.active = self
            .buffers
            .iter()
            .position(|buffer| buffer.id == id)
            .ok_or(EditorError::MissingBuffer)
            .map(Some)?;
        Ok(())
    }

    fn close(
        &mut self,
        id: BufferId,
        decision: Option<CloseDecision>,
    ) -> Result<bool, EditorError> {
        let index = self
            .buffers
            .iter()
            .position(|buffer| buffer.id == id)
            .ok_or(EditorError::MissingBuffer)?;
        if self.buffers[index].dirty {
            match decision {
                None => return Err(EditorError::DirtyDecisionRequired),
                Some(CloseDecision::Cancel) => return Ok(false),
                Some(CloseDecision::Discard) => {}
            }
        }
        self.buffers.remove(index);
        self.active = match self.buffers.len() {
            0 => None,
            length => Some(min(index, length - 1)),
        };
        Ok(true)
    }
}

#[test]
fn unicode_scalar_selection_edit_and_boundaries_are_exact() {
    let mut editor = Editor::new();
    let id = editor
        .create_buffer("unicode", "aé🙂\nxy")
        .expect("create buffer");
    let buffer = editor.active_mut().expect("active buffer");
    buffer.move_horizontal(true, false);
    buffer.move_horizontal(true, false);
    assert_eq!(buffer.cursor, 2);
    buffer.move_horizontal(false, true);
    buffer.move_horizontal(false, true);
    assert_eq!(buffer.selection(), Some((0, 2)));
    buffer.replace_selection("界").expect("replace selection");
    assert_eq!(buffer.text(), "界🙂\nxy");
    assert_eq!(buffer.cursor, 1);
    assert!(buffer.backspace().expect("backspace"));
    assert_eq!(buffer.text(), "🙂\nxy");
    assert_eq!(buffer.cursor, 0);
    assert!(!buffer.backspace().expect("boundary backspace"));
    buffer.move_line_boundary(true, false);
    assert_eq!(buffer.cursor, 1);
    assert!(buffer.delete_forward().expect("delete newline"));
    assert_eq!(buffer.text(), "🙂xy");
    assert_eq!(editor.active().expect("same buffer").id, id);
}

#[test]
fn vertical_movement_selection_and_viewport_are_deterministic() {
    let mut buffer = Buffer::new(BufferId(1), "lines", "abcd\nx\nuvwxyz");
    buffer.resize(2, 3).expect("resize");
    buffer.cursor = 3;
    buffer.move_vertical(true, false);
    assert_eq!(buffer.cursor, 6);
    buffer.move_vertical(true, true);
    assert_eq!(buffer.cursor, 10);
    assert_eq!(buffer.selection(), Some((6, 10)));
    assert_eq!(buffer.viewport.top_line, 1);
    assert_eq!(buffer.viewport.left_column, 1);
    buffer.move_vertical(false, true);
    assert_eq!(buffer.cursor, 6);
    assert_eq!(buffer.anchor, Some(6));
    assert_eq!(buffer.selection(), None);
    buffer.move_line_boundary(false, false);
    assert_eq!(buffer.cursor, 5);
    assert_eq!(buffer.anchor, None);
    assert_eq!(
        buffer.resize(0, 10).expect_err("zero rows reject"),
        EditorError::ViewportLimit
    );
    assert_eq!(
        buffer.resize(1_001, 10).expect_err("excessive rows reject"),
        EditorError::ViewportLimit
    );
}

#[test]
fn undo_redo_branching_and_paste_grouping_are_exact() {
    let mut buffer = Buffer::new(BufferId(1), "history", "");
    buffer.replace_selection("a").expect("first edit");
    buffer.replace_selection("é🙂").expect("paste is one edit");
    assert_eq!(buffer.text(), "aé🙂");
    assert!(buffer.undo());
    assert_eq!(buffer.text(), "a");
    assert!(buffer.redo());
    assert_eq!(buffer.text(), "aé🙂");
    assert!(buffer.undo());
    buffer.replace_selection("z").expect("branching edit");
    assert_eq!(buffer.text(), "az");
    assert!(!buffer.redo());
    assert_eq!(buffer.undo.len(), 2);
}

#[test]
fn literal_search_overlaps_wraps_and_edits_invalidate_results() {
    let mut buffer = Buffer::new(BufferId(1), "search", "ababa");
    assert!(buffer.start_search("aba").expect("start search"));
    assert_eq!(
        buffer.search.as_ref().expect("search state").matches,
        vec![0, 2]
    );
    assert_eq!(buffer.selection(), Some((0, 3)));
    assert!(buffer.next_match(false));
    assert_eq!(buffer.selection(), Some((2, 5)));
    assert!(buffer.next_match(false));
    assert_eq!(buffer.selection(), Some((0, 3)));
    assert!(buffer.next_match(true));
    assert_eq!(buffer.selection(), Some((2, 5)));
    buffer.replace_selection("x").expect("replace match");
    assert!(buffer.search.is_none());
    assert_eq!(
        buffer.start_search("").expect_err("empty search rejects"),
        EditorError::EmptySearch
    );
}

#[test]
fn buffer_identity_nonreuse_switch_and_dirty_close_are_exact() {
    let mut editor = Editor::new();
    let first = editor.create_buffer("one", "").expect("first buffer");
    let second = editor.create_buffer("two", "").expect("second buffer");
    editor.switch(first).expect("switch first");
    editor
        .active_mut()
        .expect("active first")
        .replace_selection("dirty")
        .expect("edit first");
    assert_eq!(
        editor
            .close(first, None)
            .expect_err("dirty decision required"),
        EditorError::DirtyDecisionRequired
    );
    assert!(
        !editor
            .close(first, Some(CloseDecision::Cancel))
            .expect("cancel close")
    );
    assert!(
        editor
            .close(first, Some(CloseDecision::Discard))
            .expect("discard close")
    );
    assert_eq!(editor.active().expect("remaining buffer").id, second);
    let third = editor.create_buffer("three", "").expect("third buffer");
    assert_ne!(third, first);
    assert_eq!(third, BufferId(3));
    assert_eq!(
        editor.switch(first).expect_err("closed identity rejects"),
        EditorError::MissingBuffer
    );
}

#[test]
fn select_all_and_forward_delete_are_one_logical_edit() {
    let mut buffer = Buffer::new(BufferId(1), "selection", "one\ntwo");
    buffer.select_all();
    assert_eq!(buffer.selection(), Some((0, 7)));
    assert!(buffer.delete_forward().expect("delete selection"));
    assert_eq!(buffer.text(), "");
    assert_eq!(buffer.undo.len(), 1);
    assert!(buffer.undo());
    assert_eq!(buffer.text(), "one\ntwo");
}

#[test]
fn content_limit_accepts_exactly_and_rejects_one_over() {
    let mut exact = Buffer::new(BufferId(1), "exact", "");
    exact
        .replace_selection(&"x".repeat(MAX_SCALARS_PER_BUFFER))
        .expect("exact content limit");
    assert_eq!(exact.content.len(), MAX_SCALARS_PER_BUFFER);

    let mut excessive = Buffer::new(BufferId(2), "excessive", "");
    assert_eq!(
        excessive
            .replace_selection(&"x".repeat(MAX_SCALARS_PER_BUFFER + 1))
            .expect_err("one-over content limit"),
        EditorError::ContentLimit
    );
    assert!(excessive.content.is_empty());
}
