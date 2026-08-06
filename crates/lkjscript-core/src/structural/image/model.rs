use super::super::value_runtime::{
    InlineStructuralValue, StaticStructuralLeaf, StructuralType, StructuralValueError,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalNodeId(u64);

impl LocalNodeId {
    pub const ROOT: Self = Self(0);

    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedU64Range {
    start: u64,
    length: u64,
}

impl CheckedU64Range {
    pub const fn new(start: u64, length: u64) -> Option<Self> {
        if start.checked_add(length).is_some() {
            Some(Self { start, length })
        } else {
            None
        }
    }

    pub const fn start(self) -> u64 {
        self.start
    }

    pub const fn len(self) -> u64 {
        self.length
    }

    pub const fn is_empty(self) -> bool {
        self.length == 0
    }

    pub const fn end(self) -> u64 {
        self.start + self.length
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralNodePayload {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    Bytes(CheckedU64Range),
    Product(CheckedU64Range),
    Enum { tag: u64, fields: CheckedU64Range },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralNodeRecord {
    pub value_type: StructuralType,
    pub payload: StructuralNodePayload,
}

#[derive(Debug, Eq, PartialEq)]
pub struct StructuralImage {
    pub(super) nodes: Vec<StructuralNodeRecord>,
    pub(super) fields: Vec<LocalNodeId>,
    pub(super) blob: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralNodeView<'a> {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    Bytes(&'a [u8]),
    Product(&'a [LocalNodeId]),
    Enum { tag: u64, fields: &'a [LocalNodeId] },
}

#[derive(Clone, Copy, Debug)]
pub struct StructuralNode<'a> {
    pub(super) image: &'a StructuralImage,
    pub(super) id: LocalNodeId,
}

impl StructuralImage {
    pub fn root(&self) -> StructuralNode<'_> {
        StructuralNode {
            image: self,
            id: LocalNodeId::ROOT,
        }
    }

    pub fn node(&self, id: LocalNodeId) -> Option<StructuralNode<'_>> {
        self.nodes.get(id.index()?)?;
        Some(StructuralNode { image: self, id })
    }

    pub fn node_count(&self) -> u64 {
        self.nodes.len() as u64
    }

    pub fn field_cell_count(&self) -> u64 {
        self.fields.len() as u64
    }

    pub fn blob_len(&self) -> u64 {
        self.blob.len() as u64
    }

    pub(super) fn record(
        &self,
        id: LocalNodeId,
    ) -> Result<&StructuralNodeRecord, StructuralValueError> {
        self.nodes
            .get(id.index().ok_or(StructuralValueError::InvariantViolation)?)
            .ok_or(StructuralValueError::InvariantViolation)
    }

    pub(super) fn range<T>(values: &[T], range: CheckedU64Range) -> Option<&[T]> {
        let start = usize::try_from(range.start()).ok()?;
        let end = usize::try_from(range.end()).ok()?;
        values.get(start..end)
    }
}

// A `StructuralNode` can only be produced by `root` or `node`, which check host indexing; image
// construction also validates every stored range before publication.
#[allow(clippy::expect_used)]
impl<'a> StructuralNode<'a> {
    pub const fn id(self) -> LocalNodeId {
        self.id
    }

    pub fn image_node_count(self) -> u64 {
        self.image.node_count()
    }

    pub fn value_type(self) -> StructuralType {
        self.image.nodes[self
            .id
            .index()
            .expect("validated structural node is host-addressable")]
        .value_type
    }

    pub fn child(self, field: usize) -> Option<Self> {
        let fields = match self.payload() {
            StructuralNodeView::Product(fields) | StructuralNodeView::Enum { fields, .. } => fields,
            StructuralNodeView::Inline(_)
            | StructuralNodeView::Static(_)
            | StructuralNodeView::Bytes(_) => return None,
        };
        self.image.node(*fields.get(field)?)
    }

    pub fn payload(self) -> StructuralNodeView<'a> {
        let node = &self.image.nodes[self
            .id
            .index()
            .expect("validated structural node is host-addressable")];
        match &node.payload {
            StructuralNodePayload::Inline(value) => StructuralNodeView::Inline(*value),
            StructuralNodePayload::Static(value) => StructuralNodeView::Static(*value),
            StructuralNodePayload::Bytes(range) => StructuralNodeView::Bytes(
                StructuralImage::range(&self.image.blob, *range)
                    .expect("validated structural byte range is host-addressable"),
            ),
            StructuralNodePayload::Product(range) => StructuralNodeView::Product(
                StructuralImage::range(&self.image.fields, *range)
                    .expect("validated structural field range is host-addressable"),
            ),
            StructuralNodePayload::Enum { tag, fields } => StructuralNodeView::Enum {
                tag: *tag,
                fields: StructuralImage::range(&self.image.fields, *fields)
                    .expect("validated structural enum range is host-addressable"),
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{CheckedU64Range, LocalNodeId};

    #[test]
    fn high_local_node_and_range_values_do_not_alias() {
        let high = u64::from(u32::MAX) + 1;
        assert_eq!(LocalNodeId::new(high).get(), high);
        assert_ne!(LocalNodeId::new(high), LocalNodeId::ROOT);
        let range = CheckedU64Range::new(high, 7).expect("wide checked range");
        assert_eq!(range.start(), high);
        assert_eq!(range.end(), high + 7);
    }
}
