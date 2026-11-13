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
                    let module = modules[&import.module];
                    let owner = program.scopes.len().checked_add(id).ok_or_else(|| {
                        Diagnostic::new(block.span, "E_IMPORT", "lexical scope index overflow")
                    })?;
                    result
                        .owners
                        .insert((import.span.file, import.span.start), owner);
                    blocks
                        .entry((module, block.span.file))
                        .or_default()
                        .insert(id);
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
                events.push((span.start, true, usize::MAX - span.end, owner));
                events.push((span.end, false, usize::MAX - span.start, owner));
            }
            events.sort_unstable();
            let mut stack = Vec::new();
            let mut segments = Vec::new();
            for (position, enter, _, owner) in events {
                if enter {
                    result
                        .parents
                        .insert(owner, stack.last().copied().unwrap_or(module));
                    stack.push(owner);
                } else if let Some(index) = stack.iter().rposition(|active| *active == owner) {
                    stack.remove(index);
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
        let at = segments.partition_point(|(position, _)| *position <= span.start);
        if at == 0 { module } else { segments[at - 1].1 }
    }
    pub fn parent(&self, owner: usize) -> Option<usize> {
        self.parents.get(&owner).copied()
    }
}
