use super::super::value_runtime::{
    InlineStructuralValue, StaticStructuralLeaf, StructuralType, StructuralValueError,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalNodeId(u32);

impl LocalNodeId {
    pub const ROOT: Self = Self(0);

    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedU32Range {
    start: u32,
    length: u32,
}

impl CheckedU32Range {
    pub const fn new(start: u32, length: u32) -> Option<Self> {
        if start.checked_add(length).is_some() {
            Some(Self { start, length })
        } else {
            None
        }
    }

    pub const fn start(self) -> u32 {
        self.start
    }

    pub const fn len(self) -> u32 {
        self.length
    }

    pub const fn is_empty(self) -> bool {
        self.length == 0
    }

    pub const fn end(self) -> u32 {
        self.start + self.length
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralNodePayload {
    Inline(InlineStructuralValue),
    Static(StaticStructuralLeaf),
    Bytes(CheckedU32Range),
    Product(CheckedU32Range),
    Enum { tag: u64, fields: CheckedU32Range },
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
        self.nodes.get(id.get() as usize)?;
        Some(StructuralNode { image: self, id })
    }

    pub fn node_count(&self) -> u32 {
        u32::try_from(self.nodes.len()).unwrap_or(u32::MAX)
    }

    pub fn field_cell_count(&self) -> u32 {
        u32::try_from(self.fields.len()).unwrap_or(u32::MAX)
    }

    pub fn blob_len(&self) -> u32 {
        u32::try_from(self.blob.len()).unwrap_or(u32::MAX)
    }

    pub(super) fn record(
        &self,
        id: LocalNodeId,
    ) -> Result<&StructuralNodeRecord, StructuralValueError> {
        self.nodes
            .get(id.get() as usize)
            .ok_or(StructuralValueError::InvariantViolation)
    }

    pub(super) fn range<T>(values: &[T], range: CheckedU32Range) -> Option<&[T]> {
        values.get(range.start() as usize..range.end() as usize)
    }
}

impl<'a> StructuralNode<'a> {
    pub const fn id(self) -> LocalNodeId {
        self.id
    }

    pub fn image_node_count(self) -> u32 {
        self.image.node_count()
    }

    pub fn value_type(self) -> StructuralType {
        self.image.nodes[self.id.get() as usize].value_type
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
        match &self.image.nodes[self.id.get() as usize].payload {
            StructuralNodePayload::Inline(value) => StructuralNodeView::Inline(*value),
            StructuralNodePayload::Static(value) => StructuralNodeView::Static(*value),
            StructuralNodePayload::Bytes(range) => StructuralNodeView::Bytes(
                &self.image.blob[range.start() as usize..range.end() as usize],
            ),
            StructuralNodePayload::Product(range) => StructuralNodeView::Product(
                &self.image.fields[range.start() as usize..range.end() as usize],
            ),
            StructuralNodePayload::Enum { tag, fields } => StructuralNodeView::Enum {
                tag: *tag,
                fields: &self.image.fields[fields.start() as usize..fields.end() as usize],
            },
        }
    }
}
