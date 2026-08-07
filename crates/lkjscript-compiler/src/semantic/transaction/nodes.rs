use crate::source::{SourceFile, SourceNode};

pub(crate) fn node_mut(files: &mut [SourceFile], index: u64) -> Option<&mut SourceNode> {
    let mut ordered: Vec<_> = (0..files.len()).collect();
    ordered.sort_by(|a, b| {
        files[*a]
            .origin
            .logical_path
            .cmp(&files[*b].origin.logical_path)
    });
    let mut remaining = usize::try_from(index).ok()?;
    let mut selected = None;
    for file_index in ordered {
        for (form_index, form) in files[file_index].syntax.iter().enumerate() {
            if let Some(path) = path_at_preorder_index(form, &mut remaining) {
                selected = Some((file_index, form_index, path));
                break;
            }
        }
        if selected.is_some() {
            break;
        }
    }
    let (file_index, form_index, path) = selected?;
    let mut node = files.get_mut(file_index)?.syntax.get_mut(form_index)?;
    for child_index in path {
        node = node.children.get_mut(child_index)?;
    }
    Some(node)
}

fn path_at_preorder_index(node: &SourceNode, remaining: &mut usize) -> Option<Vec<usize>> {
    if *remaining == 0 {
        return Some(Vec::new());
    }
    *remaining -= 1;
    let mut frames = vec![(node, 0_usize)];
    let mut path = Vec::new();
    while !frames.is_empty() {
        let next = {
            let (parent, next_child) = frames.last_mut()?;
            if *next_child == parent.children.len() {
                None
            } else {
                let index = *next_child;
                *next_child += 1;
                Some((index, &parent.children[index]))
            }
        };
        let Some((child_index, child)) = next else {
            frames.pop();
            if !frames.is_empty() {
                path.pop();
            }
            continue;
        };
        path.push(child_index);
        if *remaining == 0 {
            return Some(path);
        }
        *remaining -= 1;
        frames.push((child, 0));
    }
    None
}
