use crate::source::{SourceFile, SourceNode};

pub(crate) fn node_mut(files: &mut [SourceFile], index: u32) -> Option<&mut SourceNode> {
    let mut ordered: Vec<_> = (0..files.len()).collect();
    ordered.sort_by(|a, b| {
        files[*a]
            .origin
            .logical_path
            .cmp(&files[*b].origin.logical_path)
    });
    let mut remaining = index as usize;
    let mut selected = None;
    for file in ordered {
        let count: usize = files[file].syntax.iter().map(node_count).sum();
        if remaining < count {
            selected = Some(file);
            break;
        }
        remaining -= count;
    }
    let file = files.get_mut(selected?)?;
    for form in &mut file.syntax {
        let count = node_count(form);
        if remaining < count {
            return descendant_mut(form, remaining);
        }
        remaining -= count;
    }
    None
}

fn node_count(node: &SourceNode) -> usize {
    1 + node.children.iter().map(node_count).sum::<usize>()
}

fn descendant_mut(node: &mut SourceNode, mut index: usize) -> Option<&mut SourceNode> {
    if index == 0 {
        return Some(node);
    }
    index -= 1;
    for child in &mut node.children {
        let count = node_count(child);
        if index < count {
            return descendant_mut(child, index);
        }
        index -= count;
    }
    None
}
