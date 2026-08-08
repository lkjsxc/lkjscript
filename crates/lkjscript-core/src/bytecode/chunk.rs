//! Bytecode chunk and function prototypes.

mod failure;
pub use failure::*;

use std::collections::HashMap;

use crate::{opcode::Op, Error, Result};

include!("chunk/product.rs");

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ConstId(pub u64);

impl ConstId {
    pub fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct GlobalId(pub u64);

impl GlobalId {
    pub fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

#[derive(Debug, Clone)]
pub enum Constant {
    I64(i64),
    F64(f64),
    Str(String),
    StaticBytes(Box<[u8]>),
    Symbol(String),
    /// Prototype index for MakeClosure.
    Proto(u64),
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
enum ConstantKey {
    I64(i64),
    F64(u64),
    Str(String),
    StaticBytes(Box<[u8]>),
    Symbol(String),
    Proto(u64),
}

impl ConstantKey {
    fn copy(value: &Constant) -> Result<Self> {
        match value {
            Constant::I64(value) => Ok(Self::I64(*value)),
            Constant::F64(value) => Ok(Self::F64(value.to_bits())),
            Constant::Str(value) => copy_string(value).map(Self::Str),
            Constant::StaticBytes(value) => copy_bytes(value).map(Self::StaticBytes),
            Constant::Symbol(value) => copy_string(value).map(Self::Symbol),
            Constant::Proto(value) => Ok(Self::Proto(*value)),
        }
    }

    fn matches(&self, value: &Constant) -> bool {
        match (self, value) {
            (Self::I64(left), Constant::I64(right)) => left == right,
            (Self::F64(left), Constant::F64(right)) => *left == right.to_bits(),
            (Self::Str(left), Constant::Str(right))
            | (Self::Symbol(left), Constant::Symbol(right)) => left == right,
            (Self::StaticBytes(left), Constant::StaticBytes(right)) => left == right,
            (Self::Proto(left), Constant::Proto(right)) => left == right,
            _ => false,
        }
    }
}

fn copy_string(value: &str) -> Result<String> {
    let mut copy = String::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| Error::host("bytecode constant-index string allocation failed"))?;
    copy.push_str(value);
    Ok(copy)
}

fn copy_bytes(value: &[u8]) -> Result<Box<[u8]>> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| Error::host("bytecode constant-index bytes allocation failed"))?;
    copy.extend_from_slice(value);
    Ok(copy.into_boxed_slice())
}

#[derive(Debug, Clone)]
pub struct FunctionProto {
    pub name: String,
    pub arity: usize,
    pub locals: usize,
    pub memory_plan: Option<crate::MemoryPlanId>,
    pub memory_witness_parameters: Vec<crate::MemoryWitnessParameter>,
    pub call_witnesses: Vec<crate::CallWitnessSite>,
    pub parameter_structurals: Vec<Option<crate::StructuralRepresentationId>>,
    pub parameter_structural_places: Vec<Option<usize>>,
    pub parameter_type_variables: Vec<Option<u64>>,
    pub parameter_copy_kinds: Vec<Option<crate::StructuralKind>>,
    pub return_copy_kind: Option<crate::StructuralKind>,
    pub parameter_region_products: Vec<Option<ProductId>>,
    pub return_region_product: Option<ProductId>,
    pub return_structural: Option<crate::StructuralRepresentationId>,
    pub return_type_variable: Option<u64>,
    pub parameter_resources: Vec<Option<crate::ResourceKind>>,
    pub parameter_resource_places: Vec<Option<usize>>,
    pub return_resource: Option<ResourceReturnKind>,
    pub parameter_uniques: Vec<Option<UniqueValueKind>>,
    pub parameter_unique_places: Vec<Option<usize>>,
    pub return_unique: Option<UniqueValueKind>,
    pub unique_places: usize,
    pub failure_cleanups: Vec<FailureCleanupNode>,
    pub failure_cleanup_ranges: Vec<FailureCleanupRange>,
    pub code: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub constants: Vec<Constant>,
    pub protos: Vec<FunctionProto>,
    pub main: FunctionProto,
    pub memory_plan: Option<crate::MemoryPlanId>,
    pub memory_witness_groups: Vec<crate::InstalledMemoryWitnessGroup>,
    pub memory_witnesses: Vec<crate::InstalledMemoryWitness>,
    pub structural_types: Vec<crate::StructuralTypeMetadata>,
    pub structural_layouts: Vec<crate::StructuralLayoutMetadata>,
    pub structural_representations: Vec<crate::StructuralRepresentationMetadata>,
    pub structural_destinations: Vec<crate::StructuralDestinationMetadata>,
    pub structural_destination_fields: Vec<crate::StructuralDestinationFieldRef>,
    pub structural_aggregate_fields: Vec<crate::StructuralAggregateFieldRef>,
    pub structural_payloads: Vec<crate::StructuralPayloadRef>,
    pub required_capabilities: Vec<crate::CapabilityKind>,
    pub global_names: Vec<String>,
    pub global_prototypes: Vec<Option<u64>>,
    pub products: Vec<ProductMetadata>,
    pub product_fields: Vec<ProductFieldRef>,
    pub enums: Vec<crate::EnumMetadata>,
    pub enum_constructions: Vec<crate::EnumConstructionRef>,
    pub enum_variants: Vec<crate::EnumVariantRef>,
    pub enum_fields: Vec<crate::EnumFieldRef>,
    constant_indexes: HashMap<ConstantKey, ConstId>,
    indexed_constants: usize,
    global_indexes: HashMap<String, GlobalId>,
    indexed_globals: usize,
    product_field_indexes: HashMap<ProductFieldRef, u64>,
    indexed_product_fields: usize,
    enum_construction_indexes: HashMap<crate::EnumConstructionRef, u64>,
    indexed_enum_constructions: usize,
    enum_variant_indexes: HashMap<crate::EnumVariantRef, u64>,
    indexed_enum_variants: usize,
    enum_field_indexes: HashMap<crate::EnumFieldRef, u64>,
    indexed_enum_fields: usize,
    structural_destination_field_indexes: HashMap<crate::StructuralDestinationFieldRef, u64>,
    indexed_structural_destination_fields: usize,
    structural_aggregate_field_indexes: HashMap<crate::StructuralAggregateFieldRef, u64>,
    indexed_structural_aggregate_fields: usize,
    structural_payload_indexes: HashMap<crate::StructuralPayloadRef, u64>,
    indexed_structural_payloads: usize,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            protos: Vec::new(),
            main: FunctionProto {
                name: "<main>".into(),
                arity: 0,
                locals: 0,
                memory_plan: None,
                memory_witness_parameters: Vec::new(),
                call_witnesses: Vec::new(),
                parameter_structurals: Vec::new(),
                parameter_structural_places: Vec::new(),
                parameter_type_variables: Vec::new(),
                parameter_copy_kinds: Vec::new(),
                return_copy_kind: None,
                parameter_region_products: Vec::new(),
                return_region_product: None,
                return_structural: None,
                return_type_variable: None,
                parameter_resources: Vec::new(),
                parameter_resource_places: Vec::new(),
                return_resource: None,
                parameter_uniques: Vec::new(),
                parameter_unique_places: Vec::new(),
                return_unique: None,
                unique_places: 0,
                failure_cleanups: Vec::new(),
                failure_cleanup_ranges: Vec::new(),
                code: Vec::new(),
            },
            memory_plan: None,
            memory_witness_groups: Vec::new(),
            memory_witnesses: Vec::new(),
            structural_types: Vec::new(),
            structural_layouts: Vec::new(),
            structural_representations: Vec::new(),
            structural_destinations: Vec::new(),
            structural_destination_fields: Vec::new(),
            structural_aggregate_fields: Vec::new(),
            structural_payloads: Vec::new(),
            required_capabilities: Vec::new(),
            global_names: Vec::new(),
            global_prototypes: Vec::new(),
            products: Vec::new(),
            product_fields: Vec::new(),
            enums: Vec::new(),
            enum_constructions: Vec::new(),
            enum_variants: Vec::new(),
            enum_fields: Vec::new(),
            constant_indexes: HashMap::new(),
            indexed_constants: 0,
            global_indexes: HashMap::new(),
            indexed_globals: 0,
            product_field_indexes: HashMap::new(),
            indexed_product_fields: 0,
            enum_construction_indexes: HashMap::new(),
            indexed_enum_constructions: 0,
            enum_variant_indexes: HashMap::new(),
            indexed_enum_variants: 0,
            enum_field_indexes: HashMap::new(),
            indexed_enum_fields: 0,
            structural_destination_field_indexes: HashMap::new(),
            indexed_structural_destination_fields: 0,
            structural_aggregate_field_indexes: HashMap::new(),
            indexed_structural_aggregate_fields: 0,
            structural_payload_indexes: HashMap::new(),
            indexed_structural_payloads: 0,
        }
    }

    pub fn add_const(&mut self, constant: Constant) -> Result<ConstId> {
        self.rebuild_constant_indexes()?;
        let key = ConstantKey::copy(&constant)?;
        if let Some(id) = self.constant_indexes.get(&key).copied() {
            if id
                .index()
                .and_then(|index| self.constants.get(index))
                .is_some_and(|constant| key.matches(constant))
            {
                return Ok(id);
            }
            self.indexed_constants = usize::MAX;
            self.rebuild_constant_indexes()?;
            if let Some(id) = self.constant_indexes.get(&key).copied() {
                return Ok(id);
            }
        }
        let id = ConstId(
            u64::try_from(self.constants.len())
                .map_err(|_| Error::host("bytecode constant identity exceeds u64"))?,
        );
        self.constants
            .try_reserve(1)
            .map_err(|_| Error::host("bytecode constant table allocation failed"))?;
        self.constant_indexes
            .try_reserve(1)
            .map_err(|_| Error::host("bytecode constant index allocation failed"))?;
        self.constants.push(constant);
        self.constant_indexes.insert(key, id);
        self.indexed_constants = self.constants.len();
        Ok(id)
    }

    pub fn intern_global(&mut self, name: &str) -> Result<GlobalId> {
        self.rebuild_global_indexes()?;
        if let Some(id) = self.global_indexes.get(name).copied() {
            if id
                .index()
                .and_then(|index| self.global_names.get(index))
                .is_some_and(|stored| stored == name)
            {
                return Ok(id);
            }
            self.indexed_globals = usize::MAX;
            self.rebuild_global_indexes()?;
            if let Some(id) = self.global_indexes.get(name).copied() {
                return Ok(id);
            }
        }
        let id = GlobalId(
            u64::try_from(self.global_names.len())
                .map_err(|_| Error::host("bytecode global identity exceeds u64"))?,
        );
        let vector_name = copy_string(name)?;
        let index_name = copy_string(name)?;
        self.global_names
            .try_reserve(1)
            .map_err(|_| Error::host("bytecode global table allocation failed"))?;
        self.global_prototypes
            .try_reserve(1)
            .map_err(|_| Error::host("bytecode global prototype table allocation failed"))?;
        self.global_indexes
            .try_reserve(1)
            .map_err(|_| Error::host("bytecode global index allocation failed"))?;
        self.global_names.push(vector_name);
        self.global_prototypes.push(None);
        self.global_indexes.insert(index_name, id);
        self.indexed_globals = self.global_names.len();
        Ok(id)
    }

    pub fn intern_product_field(&mut self, descriptor: ProductFieldRef) -> Result<u64> {
        intern_descriptor(
            &mut self.product_fields,
            &mut self.product_field_indexes,
            &mut self.indexed_product_fields,
            descriptor,
            "product field",
        )
    }

    pub fn intern_enum_construction(
        &mut self,
        descriptor: crate::EnumConstructionRef,
    ) -> Result<u64> {
        intern_descriptor(
            &mut self.enum_constructions,
            &mut self.enum_construction_indexes,
            &mut self.indexed_enum_constructions,
            descriptor,
            "enum construction",
        )
    }

    pub fn intern_enum_variant(&mut self, descriptor: crate::EnumVariantRef) -> Result<u64> {
        intern_descriptor(
            &mut self.enum_variants,
            &mut self.enum_variant_indexes,
            &mut self.indexed_enum_variants,
            descriptor,
            "enum variant",
        )
    }

    pub fn intern_enum_field(&mut self, descriptor: crate::EnumFieldRef) -> Result<u64> {
        intern_descriptor(
            &mut self.enum_fields,
            &mut self.enum_field_indexes,
            &mut self.indexed_enum_fields,
            descriptor,
            "enum field",
        )
    }

    pub fn intern_structural_destination_field(
        &mut self,
        descriptor: crate::StructuralDestinationFieldRef,
    ) -> Result<u64> {
        intern_descriptor(
            &mut self.structural_destination_fields,
            &mut self.structural_destination_field_indexes,
            &mut self.indexed_structural_destination_fields,
            descriptor,
            "structural destination field",
        )
    }

    pub fn intern_structural_aggregate_field(
        &mut self,
        descriptor: crate::StructuralAggregateFieldRef,
    ) -> Result<u64> {
        intern_descriptor(
            &mut self.structural_aggregate_fields,
            &mut self.structural_aggregate_field_indexes,
            &mut self.indexed_structural_aggregate_fields,
            descriptor,
            "structural aggregate field",
        )
    }

    pub fn intern_structural_payload(
        &mut self,
        descriptor: crate::StructuralPayloadRef,
    ) -> Result<u64> {
        intern_descriptor(
            &mut self.structural_payloads,
            &mut self.structural_payload_indexes,
            &mut self.indexed_structural_payloads,
            descriptor,
            "structural payload",
        )
    }

    fn rebuild_constant_indexes(&mut self) -> Result<()> {
        if self.indexed_constants == self.constants.len() {
            return Ok(());
        }
        let mut indexes = HashMap::new();
        indexes
            .try_reserve(self.constants.len())
            .map_err(|_| Error::host("bytecode constant index allocation failed"))?;
        for (index, constant) in self.constants.iter().enumerate() {
            let id = ConstId(
                u64::try_from(index)
                    .map_err(|_| Error::host("bytecode constant identity exceeds u64"))?,
            );
            indexes.entry(ConstantKey::copy(constant)?).or_insert(id);
        }
        self.constant_indexes = indexes;
        self.indexed_constants = self.constants.len();
        Ok(())
    }

    fn rebuild_global_indexes(&mut self) -> Result<()> {
        if self.indexed_globals == self.global_names.len() {
            return Ok(());
        }
        let mut indexes = HashMap::new();
        indexes
            .try_reserve(self.global_names.len())
            .map_err(|_| Error::host("bytecode global index allocation failed"))?;
        for (index, name) in self.global_names.iter().enumerate() {
            let id = GlobalId(
                u64::try_from(index)
                    .map_err(|_| Error::host("bytecode global identity exceeds u64"))?,
            );
            indexes.entry(copy_string(name)?).or_insert(id);
        }
        self.global_indexes = indexes;
        self.indexed_globals = self.global_names.len();
        Ok(())
    }
}

fn intern_descriptor<T: Copy + Eq + std::hash::Hash>(
    table: &mut Vec<T>,
    indexes: &mut HashMap<T, u64>,
    indexed: &mut usize,
    descriptor: T,
    name: &str,
) -> Result<u64> {
    if *indexed != table.len() {
        let mut rebuilt = HashMap::new();
        rebuilt
            .try_reserve(table.len())
            .map_err(|_| Error::host(format!("bytecode {name} index allocation failed")))?;
        for (index, item) in table.iter().copied().enumerate() {
            rebuilt.entry(item).or_insert(
                u64::try_from(index)
                    .map_err(|_| Error::host(format!("bytecode {name} index exceeds u64")))?,
            );
        }
        *indexes = rebuilt;
        *indexed = table.len();
    }
    if let Some(index) = indexes.get(&descriptor).copied() {
        return Ok(index);
    }
    let index = u64::try_from(table.len())
        .map_err(|_| Error::host(format!("bytecode {name} index exceeds u64")))?;
    table
        .try_reserve(1)
        .map_err(|_| Error::host(format!("bytecode {name} table allocation failed")))?;
    indexes
        .try_reserve(1)
        .map_err(|_| Error::host(format!("bytecode {name} index allocation failed")))?;
    table.push(descriptor);
    indexes.insert(descriptor, index);
    *indexed = table.len();
    Ok(index)
}

impl FunctionProto {
    pub fn emit(&mut self, op: Op) {
        self.code.push(op as u8);
    }

    pub fn emit_u16(&mut self, n: u16) {
        self.code.extend_from_slice(&n.to_le_bytes());
    }

    pub fn emit_u64(&mut self, n: u64) {
        self.code.extend_from_slice(&n.to_le_bytes());
    }

    pub fn emit_op_u16(&mut self, op: Op, n: u16) {
        self.emit(op);
        self.emit_u16(n);
    }

    #[cfg(test)]
    pub fn emit_op_u8(&mut self, op: Op, n: u8) {
        self.emit_op_u64(op, u64::from(n));
    }

    pub fn emit_op_u64(&mut self, op: Op, n: u64) {
        self.emit(op);
        self.emit_u64(n);
    }

    pub fn emit_op_u64_pair(&mut self, op: Op, first: u64, second: u64) {
        self.emit(op);
        self.emit_u64(first);
        self.emit_u64(second);
    }

    pub fn try_reserve_code(&mut self, additional: usize) -> Result<()> {
        self.code
            .try_reserve(additional)
            .map_err(|_| Error::host("bytecode function-code reservation failed"))
    }

    pub fn try_emit(&mut self, op: Op) -> Result<()> {
        self.try_reserve_code(1)?;
        self.emit(op);
        Ok(())
    }

    pub fn try_emit_op_u16(&mut self, op: Op, n: u16) -> Result<()> {
        self.try_reserve_code(3)?;
        self.emit_op_u16(op, n);
        Ok(())
    }

    pub fn try_emit_op_u64(&mut self, op: Op, n: u64) -> Result<()> {
        self.try_reserve_code(9)?;
        self.emit_op_u64(op, n);
        Ok(())
    }

    pub fn try_emit_op_u64_pair(&mut self, op: Op, first: u64, second: u64) -> Result<()> {
        self.try_reserve_code(17)?;
        self.emit_op_u64_pair(op, first, second);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.code.len()
    }

    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }
}
