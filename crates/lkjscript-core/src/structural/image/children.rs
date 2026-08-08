use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use super::super::value_runtime::{SemanticPayload, SemanticValue, StructuralType};

/// Owned child edges for an acyclic semantic value tree.
///
/// Every edge moves a `SemanticValue` into private vector storage; this representation has no
/// references, shared owners, or unsafe constructor through which a child could point to an
/// ancestor. Graph-shaped semantic snapshots use the separate `SemanticDagSnapshot` type.
#[derive(Default)]
pub struct SemanticChildren(Vec<SemanticValue>);

impl fmt::Debug for SemanticChildren {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SemanticChildren")
            .field("field_count", &self.len())
            .finish_non_exhaustive()
    }
}

impl PartialEq for SemanticChildren {
    fn eq(&self, other: &Self) -> bool {
        SemanticValue::slices_equal(&self.0, &other.0)
    }
}

impl Eq for SemanticChildren {}

impl SemanticChildren {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn as_slice(&self) -> &[SemanticValue] {
        &self.0
    }

    pub fn as_mut_slice(&mut self) -> &mut [SemanticValue] {
        &mut self.0
    }

    pub fn push(&mut self, value: SemanticValue) {
        self.0.push(value);
    }

    pub fn pop(&mut self) -> Option<SemanticValue> {
        self.0.pop()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, SemanticValue> {
        self.0.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, SemanticValue> {
        self.0.iter_mut()
    }

    pub fn into_vec(mut self) -> Vec<SemanticValue> {
        std::mem::take(&mut self.0)
    }
}

impl Clone for SemanticChildren {
    fn clone(&self) -> Self {
        let mut pending = Vec::new();
        let mut built = Vec::new();
        pending.extend(self.0.iter().rev().map(CloneTask::Visit));
        while let Some(task) = pending.pop() {
            match task {
                CloneTask::Visit(value) => match &value.payload {
                    SemanticPayload::Product(fields) => {
                        schedule(&mut pending, value.value_type, Aggregate::Product, fields)
                    }
                    SemanticPayload::Enum {
                        tag,
                        active_payload,
                    } => schedule(
                        &mut pending,
                        value.value_type,
                        Aggregate::Enum(*tag),
                        active_payload,
                    ),
                    payload => {
                        built.push(SemanticValue::new(value.value_type, clone_leaf(payload)))
                    }
                },
                CloneTask::Finish {
                    value_type,
                    aggregate,
                    fields,
                } => {
                    let start = built.len() - fields;
                    let children = Self(built.split_off(start));
                    let payload = match aggregate {
                        Aggregate::Product => SemanticPayload::Product(children),
                        Aggregate::Enum(tag) => SemanticPayload::Enum {
                            tag,
                            active_payload: children,
                        },
                    };
                    built.push(SemanticValue::new(value_type, payload));
                }
            }
        }
        Self(built)
    }
}

impl Drop for SemanticChildren {
    fn drop(&mut self) {
        let mut pending = std::mem::take(&mut self.0);
        while let Some(mut value) = pending.pop() {
            match &mut value.payload {
                SemanticPayload::Product(fields)
                | SemanticPayload::Enum {
                    active_payload: fields,
                    ..
                } => pending.append(&mut fields.0),
                _ => {}
            }
        }
    }
}

impl Deref for SemanticChildren {
    type Target = [SemanticValue];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for SemanticChildren {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl From<Vec<SemanticValue>> for SemanticChildren {
    fn from(values: Vec<SemanticValue>) -> Self {
        Self(values)
    }
}

impl IntoIterator for SemanticChildren {
    type Item = SemanticValue;
    type IntoIter = std::vec::IntoIter<SemanticValue>;

    fn into_iter(mut self) -> Self::IntoIter {
        std::mem::take(&mut self.0).into_iter()
    }
}

impl<'a> IntoIterator for &'a SemanticChildren {
    type Item = &'a SemanticValue;
    type IntoIter = std::slice::Iter<'a, SemanticValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut SemanticChildren {
    type Item = &'a mut SemanticValue;
    type IntoIter = std::slice::IterMut<'a, SemanticValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Clone, Copy)]
enum Aggregate {
    Product,
    Enum(u64),
}

enum CloneTask<'a> {
    Visit(&'a SemanticValue),
    Finish {
        value_type: StructuralType,
        aggregate: Aggregate,
        fields: usize,
    },
}

fn schedule<'a>(
    pending: &mut Vec<CloneTask<'a>>,
    value_type: StructuralType,
    aggregate: Aggregate,
    fields: &'a SemanticChildren,
) {
    pending.push(CloneTask::Finish {
        value_type,
        aggregate,
        fields: fields.len(),
    });
    pending.extend(fields.iter().rev().map(CloneTask::Visit));
}

fn clone_leaf(payload: &SemanticPayload) -> SemanticPayload {
    match payload {
        SemanticPayload::Inline(value) => SemanticPayload::Inline(*value),
        SemanticPayload::Static(value) => SemanticPayload::Static(*value),
        SemanticPayload::String(bytes) => SemanticPayload::String(bytes.clone()),
        SemanticPayload::Path(bytes) => SemanticPayload::Path(bytes.clone()),
        SemanticPayload::Bytes(bytes) => SemanticPayload::Bytes(bytes.clone()),
        SemanticPayload::ByteVector(bytes) => SemanticPayload::ByteVector(bytes.clone()),
        SemanticPayload::Product(_) | SemanticPayload::Enum { .. } => unreachable!(),
    }
}
