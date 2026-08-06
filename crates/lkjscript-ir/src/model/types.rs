use super::*;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    Source { source: u64, node: u64 },
    Synthetic,
}

impl Origin {
    pub const SYNTHETIC: Self = Self::Synthetic;

    pub const fn source(source: u64, node: u64) -> Self {
        Self::Source { source, node }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signature {
    pub type_parameters: Vec<String>,
    pub bounds: Vec<TraitBound>,
    pub memory_witness_parameters: Vec<MemoryWitnessParameter>,
    pub parameters: Vec<SsaType>,
    pub result: Box<SsaType>,
}

impl Signature {
    pub fn monomorphic(parameters: Vec<SsaType>, result: SsaType) -> Self {
        Self {
            type_parameters: Vec::new(),
            bounds: Vec::new(),
            memory_witness_parameters: Vec::new(),
            parameters,
            result: Box::new(result),
        }
    }
}

pub enum SsaType {
    Unit,
    Bool,
    I64,
    F64,
    Str,
    Symbol,
    /// Exact immutable bytes value; constants are static and runtime values are affine.
    Bytes,
    /// Exact affine deterministic byte-vector owner.
    ByteVector,
    /// Exact shared bounded byte-vector view.
    ByteSlice,
    /// Exact exclusive bounded byte-vector view.
    ByteSliceMut,
    Path,
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(lkjscript_contracts::ResourceKind),
    /// Verifier-only private aggregate construction state; never crosses a signature.
    StructuralDestination(StructuralTypeId),
    Product(ProductId),
    Enum {
        id: EnumId,
        arguments: Vec<SsaType>,
    },
    List(Box<SsaType>),
    Function(Box<Signature>),
    TypeParameter(String),
}

impl Clone for SsaType {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a SsaType),
            Enum(EnumId, usize),
            List,
            Function {
                type_parameters: &'a [String],
                bounds: &'a [TraitBound],
                witnesses: &'a [MemoryWitnessParameter],
                parameter_count: usize,
            },
        }

        let mut pending = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = pending.pop() {
            match item {
                Work::Visit(ty) => match ty {
                    SsaType::Unit => completed.push(SsaType::Unit),
                    SsaType::Bool => completed.push(SsaType::Bool),
                    SsaType::I64 => completed.push(SsaType::I64),
                    SsaType::F64 => completed.push(SsaType::F64),
                    SsaType::Str => completed.push(SsaType::Str),
                    SsaType::Symbol => completed.push(SsaType::Symbol),
                    SsaType::Bytes => completed.push(SsaType::Bytes),
                    SsaType::ByteVector => completed.push(SsaType::ByteVector),
                    SsaType::ByteSlice => completed.push(SsaType::ByteSlice),
                    SsaType::ByteSliceMut => completed.push(SsaType::ByteSliceMut),
                    SsaType::Path => completed.push(SsaType::Path),
                    SsaType::Capability(kind) => completed.push(SsaType::Capability(*kind)),
                    SsaType::Resource(kind) => completed.push(SsaType::Resource(*kind)),
                    SsaType::StructuralDestination(id) => {
                        completed.push(SsaType::StructuralDestination(*id));
                    }
                    SsaType::Product(id) => completed.push(SsaType::Product(*id)),
                    SsaType::Enum { id, arguments } => {
                        pending.push(Work::Enum(*id, arguments.len()));
                        pending.extend(arguments.iter().rev().map(Work::Visit));
                    }
                    SsaType::List(inner) => {
                        pending.push(Work::List);
                        pending.push(Work::Visit(inner));
                    }
                    SsaType::Function(signature) => {
                        pending.push(Work::Function {
                            type_parameters: &signature.type_parameters,
                            bounds: &signature.bounds,
                            witnesses: &signature.memory_witness_parameters,
                            parameter_count: signature.parameters.len(),
                        });
                        pending.push(Work::Visit(&signature.result));
                        pending.extend(signature.parameters.iter().rev().map(Work::Visit));
                    }
                    SsaType::TypeParameter(name) => {
                        completed.push(SsaType::TypeParameter(name.clone()));
                    }
                },
                Work::Enum(id, count) => {
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("SSA type clone enum completion order")
                    };
                    let arguments = completed.split_off(split);
                    completed.push(SsaType::Enum { id, arguments });
                }
                Work::List => {
                    let Some(inner) = completed.pop() else {
                        unreachable!("SSA type clone list completion order")
                    };
                    completed.push(SsaType::List(Box::new(inner)));
                }
                Work::Function {
                    type_parameters,
                    bounds,
                    witnesses,
                    parameter_count,
                } => {
                    let Some(result) = completed.pop() else {
                        unreachable!("SSA type clone function result completion order")
                    };
                    let Some(split) = completed.len().checked_sub(parameter_count) else {
                        unreachable!("SSA type clone function parameter completion order")
                    };
                    let parameters = completed.split_off(split);
                    completed.push(SsaType::Function(Box::new(Signature {
                        type_parameters: type_parameters.to_vec(),
                        bounds: bounds.to_vec(),
                        memory_witness_parameters: witnesses.to_vec(),
                        parameters,
                        result: Box::new(result),
                    })));
                }
            }
        }
        match completed.pop() {
            Some(ty) => ty,
            None => unreachable!("SSA type clone omitted its root"),
        }
    }
}

impl Drop for SsaType {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_type_children(self, &mut pending);
        while let Some(mut ty) = pending.pop() {
            take_type_children(&mut ty, &mut pending);
        }
    }
}

fn take_type_children(ty: &mut SsaType, pending: &mut Vec<SsaType>) {
    match ty {
        SsaType::Enum { arguments, .. } => pending.append(arguments),
        SsaType::List(inner) => {
            pending.push(std::mem::replace(inner.as_mut(), SsaType::Unit));
        }
        SsaType::Function(signature) => {
            pending.append(&mut signature.parameters);
            pending.push(std::mem::replace(signature.result.as_mut(), SsaType::Unit));
        }
        _ => {}
    }
}

impl PartialEq for SsaType {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for SsaType {}

impl PartialOrd for SsaType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

enum TypeComparisonWork<'a> {
    Pair(&'a SsaType, &'a SsaType),
    Length(usize, usize),
}

impl Ord for SsaType {
    fn cmp(&self, other: &Self) -> Ordering {
        let mut pending = vec![TypeComparisonWork::Pair(self, other)];
        while let Some(item) = pending.pop() {
            match item {
                TypeComparisonWork::Length(left, right) => match left.cmp(&right) {
                    Ordering::Equal => {}
                    result => return result,
                },
                TypeComparisonWork::Pair(left, right) => {
                    match ssa_type_tag(left).cmp(&ssa_type_tag(right)) {
                        Ordering::Equal => {}
                        result => return result,
                    }
                    match (left, right) {
                        (SsaType::Capability(left), SsaType::Capability(right)) => {
                            match capability_key(*left).cmp(&capability_key(*right)) {
                                Ordering::Equal => {}
                                result => return result,
                            }
                        }
                        (SsaType::Resource(left), SsaType::Resource(right)) => {
                            match resource_key(*left).cmp(&resource_key(*right)) {
                                Ordering::Equal => {}
                                result => return result,
                            }
                        }
                        (
                            SsaType::StructuralDestination(left),
                            SsaType::StructuralDestination(right),
                        ) => match left.cmp(right) {
                            Ordering::Equal => {}
                            result => return result,
                        },
                        (SsaType::Product(left), SsaType::Product(right)) => {
                            match left.cmp(right) {
                                Ordering::Equal => {}
                                result => return result,
                            }
                        }
                        (
                            SsaType::Enum {
                                id: left_id,
                                arguments: left_arguments,
                            },
                            SsaType::Enum {
                                id: right_id,
                                arguments: right_arguments,
                            },
                        ) => {
                            match left_id.cmp(right_id) {
                                Ordering::Equal => {}
                                result => return result,
                            }
                            push_pairs(&mut pending, left_arguments, right_arguments);
                        }
                        (SsaType::List(left), SsaType::List(right)) => {
                            pending.push(TypeComparisonWork::Pair(left, right));
                        }
                        (SsaType::Function(left), SsaType::Function(right)) => {
                            for comparison in [
                                left.type_parameters.cmp(&right.type_parameters),
                                left.bounds.cmp(&right.bounds),
                                left.memory_witness_parameters
                                    .cmp(&right.memory_witness_parameters),
                            ] {
                                if comparison != Ordering::Equal {
                                    return comparison;
                                }
                            }
                            pending.push(TypeComparisonWork::Pair(&left.result, &right.result));
                            push_pairs(&mut pending, &left.parameters, &right.parameters);
                        }
                        (SsaType::TypeParameter(left), SsaType::TypeParameter(right)) => {
                            match left.cmp(right) {
                                Ordering::Equal => {}
                                result => return result,
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ordering::Equal
    }
}

fn push_pairs<'a>(
    pending: &mut Vec<TypeComparisonWork<'a>>,
    left: &'a [SsaType],
    right: &'a [SsaType],
) {
    pending.push(TypeComparisonWork::Length(left.len(), right.len()));
    pending.extend(
        left.iter()
            .zip(right)
            .rev()
            .map(|(left, right)| TypeComparisonWork::Pair(left, right)),
    );
}

impl Hash for SsaType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut pending = vec![self];
        while let Some(ty) = pending.pop() {
            ssa_type_tag(ty).hash(state);
            match ty {
                SsaType::Capability(kind) => capability_key(*kind).hash(state),
                SsaType::Resource(kind) => resource_key(*kind).hash(state),
                SsaType::StructuralDestination(id) => id.hash(state),
                SsaType::Product(id) => id.hash(state),
                SsaType::Enum { id, arguments } => {
                    id.hash(state);
                    arguments.len().hash(state);
                    pending.extend(arguments.iter().rev());
                }
                SsaType::List(inner) => pending.push(inner),
                SsaType::Function(signature) => {
                    signature.type_parameters.hash(state);
                    signature.bounds.hash(state);
                    signature.memory_witness_parameters.hash(state);
                    signature.parameters.len().hash(state);
                    pending.push(&signature.result);
                    pending.extend(signature.parameters.iter().rev());
                }
                SsaType::TypeParameter(name) => name.hash(state),
                _ => {}
            }
        }
    }
}

impl std::fmt::Debug for SsaType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        enum Work<'a> {
            Type(&'a SsaType),
            Text(&'static str),
        }
        let mut pending = vec![Work::Type(self)];
        while let Some(item) = pending.pop() {
            match item {
                Work::Text(text) => formatter.write_str(text)?,
                Work::Type(ty) => match ty {
                    SsaType::Enum { id, arguments } => {
                        write!(formatter, "Enum({id:?}, [")?;
                        pending.push(Work::Text("])"));
                        for (index, argument) in arguments.iter().enumerate().rev() {
                            pending.push(Work::Type(argument));
                            if index != 0 {
                                pending.push(Work::Text(", "));
                            }
                        }
                    }
                    SsaType::List(inner) => {
                        formatter.write_str("List(")?;
                        pending.push(Work::Text(")"));
                        pending.push(Work::Type(inner));
                    }
                    SsaType::Function(signature) => {
                        formatter.write_str("Function(")?;
                        pending.push(Work::Text(")"));
                        pending.push(Work::Type(&signature.result));
                        pending.push(Work::Text(" -> "));
                        for (index, parameter) in signature.parameters.iter().enumerate().rev() {
                            pending.push(Work::Type(parameter));
                            if index != 0 {
                                pending.push(Work::Text(", "));
                            }
                        }
                    }
                    SsaType::Capability(kind) => write!(formatter, "Capability({kind:?})")?,
                    SsaType::Resource(kind) => write!(formatter, "Resource({kind:?})")?,
                    SsaType::StructuralDestination(id) => {
                        write!(formatter, "StructuralDestination({id:?})")?;
                    }
                    SsaType::Product(id) => write!(formatter, "Product({id:?})")?,
                    SsaType::TypeParameter(name) => {
                        write!(formatter, "TypeParameter({name:?})")?;
                    }
                    _ => formatter.write_str(ssa_type_name(ty))?,
                },
            }
        }
        Ok(())
    }
}

const fn ssa_type_tag(ty: &SsaType) -> u8 {
    match ty {
        SsaType::Unit => 0,
        SsaType::Bool => 1,
        SsaType::I64 => 2,
        SsaType::F64 => 3,
        SsaType::Str => 4,
        SsaType::Symbol => 5,
        SsaType::Bytes => 6,
        SsaType::ByteVector => 7,
        SsaType::ByteSlice => 8,
        SsaType::ByteSliceMut => 9,
        SsaType::Path => 10,
        SsaType::Capability(_) => 11,
        SsaType::Resource(_) => 12,
        SsaType::StructuralDestination(_) => 13,
        SsaType::Product(_) => 14,
        SsaType::Enum { .. } => 15,
        SsaType::List(_) => 16,
        SsaType::Function(_) => 17,
        SsaType::TypeParameter(_) => 18,
    }
}

const fn ssa_type_name(ty: &SsaType) -> &'static str {
    match ty {
        SsaType::Unit => "Unit",
        SsaType::Bool => "Bool",
        SsaType::I64 => "I64",
        SsaType::F64 => "F64",
        SsaType::Str => "Str",
        SsaType::Symbol => "Symbol",
        SsaType::Bytes => "Bytes",
        SsaType::ByteVector => "ByteVector",
        SsaType::ByteSlice => "ByteSlice",
        SsaType::ByteSliceMut => "ByteSliceMut",
        SsaType::Path => "Path",
        _ => "Type",
    }
}

const fn capability_key(kind: lkjscript_contracts::CapabilityKind) -> u8 {
    use lkjscript_contracts::CapabilityKind::*;
    match kind {
        Arguments => 0,
        Clock => 1,
        Entropy => 2,
        FileSystem => 3,
        Network => 4,
        Sqlite => 5,
        Stdio => 6,
        Terminal => 7,
    }
}

const fn resource_key(kind: lkjscript_contracts::ResourceKind) -> u8 {
    use lkjscript_contracts::ResourceKind::*;
    match kind {
        InputStream => 0,
        OutputStream => 1,
        FileReader => 2,
        FileWriter => 3,
        FileAppender => 4,
        Directory => 5,
        TcpListener => 6,
        TcpStream => 7,
        SqliteConnection => 8,
        SqliteStatement => 9,
        TerminalSession => 10,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraitBound {
    pub parameter: String,
    pub trait_id: TraitId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitRole {
    Copy,
    Clone,
    Drop,
    Send,
    Sync,
    User,
}

impl TraitRole {
    pub const fn is_auto(self) -> bool {
        matches!(self, Self::Copy | Self::Send | Self::Sync)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitMetadata {
    pub id: TraitId,
    pub name: String,
    pub role: TraitRole,
    pub source: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImplMetadata {
    pub id: ImplId,
    pub trait_id: TraitId,
    pub product: ProductId,
    pub source: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryWitnessParameter {
    pub parameter: String,
    pub operations: Vec<lkjscript_contracts::MemoryWitnessOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryWitnessBinding {
    pub parameter: String,
    pub witness: MemoryWitnessId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeSubstitution {
    pub parameter: String,
    pub ty: SsaType,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TraitWitnessKind {
    AutoTrait,
    Explicit(ImplId),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraitWitness {
    pub trait_id: TraitId,
    pub ty: SsaType,
    pub kind: TraitWitnessKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenericInstantiation {
    pub substitutions: Vec<TypeSubstitution>,
    pub witnesses: Vec<TraitWitness>,
    pub memory_witnesses: Vec<MemoryWitnessBinding>,
}
