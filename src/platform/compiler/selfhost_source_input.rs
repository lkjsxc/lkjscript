//! Current raw source carrier and request framing for source-driven compilation.
//!
//! This production owner copies a complete canonical source view. It does not
//! validate source semantics, infer types, construct custody evidence, normalize
//! instructions, select reachable owners, or repair caller claims. In particular,
//! the exact encoded revision core is supplied by the source export; it is never
//! synthesized from semantic state. Source readers remain the admission authority.

#![forbid(unsafe_code)]

mod export;

pub(crate) use export::ExportedSource;

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::TargetId;

pub(crate) const SOURCE_PROFILE: u32 = 3;
pub(crate) const REQUEST_PROFILE: u32 = 2;
const SOURCE_HEADER_BYTES: usize = 128;
const REQUEST_HEADER_BYTES: usize = 32;
const OWNER_ROW_BYTES: usize = 50;
const TYPE_ROW_BYTES: usize = 32;
const RETIREMENT_ROW_BYTES: usize = 49;
const SPAN_ROW_BYTES: usize = 8;

/// Exact claims and the unchanged canonical core window from one public export.
///
/// These fields intentionally remain raw. Negative fixtures can carry an invalid
/// core, incorrect claimed revision, or mismatched state through the same framer
/// and exercise the source reader instead of a Rust semantic prefilter.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawRevisionProvenance<'a> {
    pub(crate) claimed_revision: [u8; 32],
    pub(crate) canonical_core: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawOwner<'a> {
    pub(crate) key: [u8; 17],
    pub(crate) kind: u8,
    pub(crate) digest: [u8; 32],
    pub(crate) bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawType<'a> {
    pub(crate) digest: [u8; 32],
    pub(crate) bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawRetirement<'a> {
    pub(crate) key: [u8; 17],
    pub(crate) digest: [u8; 32],
    pub(crate) bytes: &'a [u8],
}

/// Complete original source inventory, in canonical directory order.
///
/// Production callers obtain this view from [`ExportedSource`]. The low-level
/// public fields also serve deliberately malformed source fixtures. The writer
/// preserves every supplied row and object byte, including uncalled owners,
/// unselected targets, and valid or invalid unused types. It neither sorts nor
/// deduplicates a malformed inventory. No semantic-validity claim follows from
/// successful framing.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RawSourceGraph<'a> {
    pub(crate) package: [u8; 16],
    pub(crate) package_name: &'a [u8],
    pub(crate) semantic_state: [u8; 32],
    pub(crate) provenance: RawRevisionProvenance<'a>,
    pub(crate) owners: &'a [RawOwner<'a>],
    pub(crate) types: &'a [RawType<'a>],
    pub(crate) retirements: &'a [RawRetirement<'a>],
}

impl<'a> RawSourceGraph<'a> {
    fn objects(&self) -> impl Iterator<Item = &'a [u8]> + '_ {
        self.owners
            .iter()
            .map(|row| row.bytes)
            .chain(self.types.iter().map(|row| row.bytes))
            .chain(self.retirements.iter().map(|row| row.bytes))
    }
}

/// Request framing only. A mode value is not a claim that an entrypoint has an
/// implemented backend. The profile cut and routing are adopted together.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceOutputMode {
    Elf,
    CanonicalArtifact,
}

impl SourceOutputMode {
    const fn tag(self) -> u8 {
        match self {
            Self::Elf => 0,
            Self::CanonicalArtifact => 1,
        }
    }
}

/// Ordinary source compilation has no default proof allowance.
pub(crate) fn carrier(source: &RawSourceGraph<'_>) -> Result<Vec<u8>, Diagnostic> {
    carrier_with_allowance(source, None)
}

/// Serialize the exact caller-selected E1 proof policy without evaluating it.
/// `Some(0)` and `Some(u64::MAX)` are distinct finite allowances, never aliases
/// for None. This is not an artifact, allocation, or summary-work budget.
pub(crate) fn carrier_with_allowance(
    source: &RawSourceGraph<'_>,
    allowance: Option<u64>,
) -> Result<Vec<u8>, Diagnostic> {
    let layout = SourceLayout::measure(source)?;
    let mut header = [0_u8; SOURCE_HEADER_BYTES];
    header[..8].copy_from_slice(b"LKJSG002");
    for (at, value) in [
        (8, layout.length),
        (12, wire_count(source.owners.len())?),
        (16, wire_count(source.types.len())?),
        (20, wire_count(source.retirements.len())?),
        (24, wire_count(source.package_name.len())?),
        (80, layout.owners_at),
        (84, layout.types_at),
        (88, layout.retirements_at),
        (92, layout.spans_at),
        (96, layout.objects_at),
        (112, SOURCE_PROFILE),
        (116, layout.provenance_at),
        (120, layout.provenance_bytes),
    ] {
        put_u32(&mut header, at, value)?;
    }
    // Dependency count at 28 and reserved word at 124 remain canonical zero.
    header[32..48].copy_from_slice(&source.package);
    header[48..80].copy_from_slice(&source.semantic_state);
    if let Some(allowance) = allowance {
        put_u32(&mut header, 100, 1)?;
        header[104..112].copy_from_slice(&allowance.to_le_bytes());
    }

    // Every extent, including each object length, was checked before reservation.
    // One exact reservation owns all eventual bytes; no partially filled carrier
    // can escape and no append is allowed to exceed the measured length.
    let mut output = Output::new(layout.length as usize)?;
    output.append(&header)?;
    output.append(source.package_name)?;
    output.append(&source.provenance.claimed_revision)?;
    output.append(source.provenance.canonical_core)?;
    for owner in source.owners {
        output.append(&owner.key)?;
        output.append(&[owner.kind])?;
        output.append(&owner.digest)?;
    }
    for ty in source.types {
        output.append(&ty.digest)?;
    }
    for retirement in source.retirements {
        output.append(&retirement.key)?;
        output.append(&retirement.digest)?;
    }
    let mut object_at = layout.objects_at as usize;
    for bytes in source.objects() {
        output.append(&wire_count(object_at)?.to_le_bytes())?;
        output.append(&wire_count(bytes.len())?.to_le_bytes())?;
        object_at = source_extent(object_at, bytes.len(), 1)?;
    }
    if wire_count(object_at)? != layout.length {
        return Err(layout_error());
    }
    for bytes in source.objects() {
        output.append(bytes)?;
    }
    output.finish()
}

/// Frame one caller-selected target and mode around the unchanged source bytes.
/// This checks representation/allocation only; it deliberately does not reject
/// malformed embedded source, infer a target entry index, or filter the carrier.
pub(crate) fn request(
    source_carrier: &[u8],
    target: TargetId,
    mode: SourceOutputMode,
) -> Result<Vec<u8>, Diagnostic> {
    wire_count(source_carrier.len())?;
    let length = contiguous_extent(REQUEST_HEADER_BYTES, source_carrier.len(), 1)?;
    let mut header = [0_u8; REQUEST_HEADER_BYTES];
    header[..8].copy_from_slice(b"LKJSRQ01");
    header[8..12].copy_from_slice(&REQUEST_PROFILE.to_le_bytes());
    header[12] = 12;
    header[13..29].copy_from_slice(&target.bytes());
    header[29] = mode.tag();
    let mut output = Output::new(length)?;
    output.append(&header)?;
    output.append(source_carrier)?;
    output.finish()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SourceLayout {
    provenance_at: u32,
    provenance_bytes: u32,
    owners_at: u32,
    types_at: u32,
    retirements_at: u32,
    spans_at: u32,
    objects_at: u32,
    length: u32,
}

impl SourceLayout {
    fn measure(source: &RawSourceGraph<'_>) -> Result<Self, Diagnostic> {
        let provenance_at = source_extent(SOURCE_HEADER_BYTES, source.package_name.len(), 1)?;
        let provenance_bytes = source_extent(32, source.provenance.canonical_core.len(), 1)?;
        let owners_at = source_extent(provenance_at, provenance_bytes, 1)?;
        let types_at = source_extent(owners_at, source.owners.len(), OWNER_ROW_BYTES)?;
        let retirements_at = source_extent(types_at, source.types.len(), TYPE_ROW_BYTES)?;
        let spans_at = source_extent(
            retirements_at,
            source.retirements.len(),
            RETIREMENT_ROW_BYTES,
        )?;
        let object_count = source
            .owners
            .len()
            .checked_add(source.types.len())
            .and_then(|count| count.checked_add(source.retirements.len()))
            .ok_or_else(extent_error)?;
        let objects_at = source_extent(spans_at, object_count, SPAN_ROW_BYTES)?;
        let mut length = objects_at;
        for bytes in source.objects() {
            length = source_extent(length, bytes.len(), 1)?;
        }
        Ok(Self {
            provenance_at: wire_count(provenance_at)?,
            provenance_bytes: wire_count(provenance_bytes)?,
            owners_at: wire_count(owners_at)?,
            types_at: wire_count(types_at)?,
            retirements_at: wire_count(retirements_at)?,
            spans_at: wire_count(spans_at)?,
            objects_at: wire_count(objects_at)?,
            length: wire_count(length)?,
        })
    }
}

fn wire_count(value: usize) -> Result<u32, Diagnostic> {
    u32::try_from(value).map_err(|_| extent_error())
}

/// A representation boundary, not a default logical quota. Profile 3 retains
/// u32 extents, and a Rust Vec additionally requires a contiguous isize extent.
fn source_extent(start: usize, count: usize, stride: usize) -> Result<usize, Diagnostic> {
    let end = contiguous_extent(start, count, stride)?;
    wire_count(end)?;
    Ok(end)
}

fn contiguous_extent(start: usize, count: usize, stride: usize) -> Result<usize, Diagnostic> {
    let end = count
        .checked_mul(stride)
        .and_then(|bytes| start.checked_add(bytes))
        .ok_or_else(extent_error)?;
    if end > isize::MAX as usize {
        return Err(extent_error());
    }
    Ok(end)
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) -> Result<(), Diagnostic> {
    let end = at.checked_add(4).ok_or_else(layout_error)?;
    bytes
        .get_mut(at..end)
        .ok_or_else(layout_error)?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

struct Output {
    bytes: Vec<u8>,
    length: usize,
}

impl Output {
    fn new(length: usize) -> Result<Self, Diagnostic> {
        contiguous_extent(0, length, 1)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| allocation_error())?;
        Ok(Self { bytes, length })
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), Diagnostic> {
        let remaining = self
            .length
            .checked_sub(self.bytes.len())
            .ok_or_else(layout_error)?;
        if bytes.len() > remaining {
            return Err(layout_error());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn finish(self) -> Result<Vec<u8>, Diagnostic> {
        if self.bytes.len() != self.length {
            return Err(layout_error());
        }
        Ok(self.bytes)
    }
}

fn extent_error() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "compiler_source_input_extent",
        "source framing exceeds a checked current u32 or contiguous host extent",
    )
}

fn allocation_error() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "compiler_source_input_allocation",
        "source framing could not reserve its measured output or inventory storage",
    )
}

fn layout_error() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        "compiler_source_input_layout",
        "source framing did not reproduce its checked measured layout",
    )
}

#[cfg(test)]
mod tests;
