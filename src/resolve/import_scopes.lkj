//! Lexical imports are indexed by source blocks, separately from module privacy.
use crate::ast::{BlockId, Diagnostic, Item, ModuleId, Program, Span, Stmt, Use};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct ImportScopes {
    owners: BTreeMap<(usize, usize), usize>,
    parents: BTreeMap<usize, usize>,
    segments: BTreeMap<(ModuleId, usize), Vec<(usize, usize)>>,
}
impl ImportScopes {
    pub fn new(
        program: &Program,
        modules: &BTreeMap<String, ModuleId>,
    ) -> Result<Self, Diagnostic> {
        let mut result = Self::default();
        let mut blocks: BTreeMap<(ModuleId, usize), BTreeSet<BlockId>> = BTreeMap::new();
        for (id, block) in program.blocks.iter().enumerate() {
            for statement in &block.statements {
                if let Stmt::Item(Item::Use(first)) = statement {
                    let import = program.uses.get(*first).ok_or_else(|| {
                        Diagnostic::new(block.span, "E_IMPORT", "block import has no declaration")
                    })?;
                    let module = *modules.get(&import.module).unwrap();
                    let owner = program.scopes.len().checked_add(id).ok_or_else(|| {
                        Diagnostic::new(block.span, "E_IMPORT", "lexical scope index overflow")
                    })?;
                    result
                        .owners
                        .insert((import.span.file, import.span.start), owner);
                    let key = (module, block.span.file);
                    if !blocks.contains_key(&key) {
                        blocks.insert(key, BTreeSet::new());
                    }
                    blocks.get_mut(&key).unwrap().insert(id);
                }
            }
        }
        for ((module, file), blocks) in blocks {
            let mut events = Vec::new();
            for id in blocks {
                let span = program.blocks[id].span;
                let owner = program.scopes.len() + id;
                // Ends precede starts at a shared boundary; outer starts precede
                // inner starts and inner ends precede outer ends.
                events.push((span.start, true, !span.end, owner));
                events.push((span.end, false, !span.start, owner));
            }
            events.sort();
            let mut stack = Vec::new();
            let mut segments = Vec::new();
            for (position, enter, _, owner) in events {
                if enter {
                    result
                        .parents
                        .insert(owner, stack.last().copied().unwrap_or(module));
                    stack.push(owner);
                } else {
                    let mut index = stack.len();
                    while index > 0 {
                        index -= 1;
                        if stack[index] == owner {
                            for next in index + 1..stack.len() {
                                stack[next - 1] = stack[next];
                            }
                            stack.pop();
                            break;
                        }
                    }
                }
                segments.push((position, stack.last().copied().unwrap_or(module)));
            }
            result.segments.insert((module, file), segments);
        }
        Ok(result)
    }
    pub fn import_owner(&self, import: &Use, module: ModuleId) -> usize {
        self.owners
            .get(&(import.span.file, import.span.start))
            .copied()
            .unwrap_or(module)
    }
    pub fn at(&self, module: ModuleId, span: Span) -> usize {
        let Some(segments) = self.segments.get(&(module, span.file)) else {
            return module;
        };
        let mut at = 0;
        let mut end = segments.len();
        while at < end {
            let middle = at + (end - at) / 2;
            if segments[middle].0 <= span.start {
                at = middle + 1;
            } else {
                end = middle;
            }
        }
        if at == 0 { module } else { segments[at - 1].1 }
    }
    pub fn parent(&self, owner: usize) -> Option<usize> {
        self.parents.get(&owner).copied()
    }
}
