//! Fixed region storage with generative custody and relative addresses.
//!
//! An admitted evaluator supplies exact layout keys. This layer checks allocation
//! boundaries and layout equality; it does not derive language types or grant I/O.
//! One allocation contains the header, slot bitmap and payloads. No cell owns a
//! heap allocation or reference counter, so dropping custody releases one buffer.
//!
//! A pointer cannot escape its region brand:
//! ```compile_fail
//! use lkjscript::platform::execution::{ExecutionControl, region::*};
//! let control = ExecutionControl::uncancelled();
//! let layout = RegionLayout::<i64>::new([1; 32], 8, 8).unwrap();
//! let pointer = with_region(256, &control, |_| Ok(()), |mut region| {
//!     region.allocate(layout, &42_i64.to_le_bytes(), &control)
//! }).unwrap();
//! ```
//! Frozen custody has no mutation operation:
//! ```compile_fail
//! use lkjscript::platform::execution::{ExecutionControl, region::*};
//! let control = ExecutionControl::uncancelled();
//! let layout = RegionLayout::<i64>::new([1; 32], 8, 8).unwrap();
//! with_region(256, &control, |_| Ok(()), |mut region| {
//!     let pointer = region.allocate(layout, &42_i64.to_le_bytes(), &control)?;
//!     let mut frozen = region.freeze(&control)?;
//!     frozen.write(layout, pointer, &0_i64.to_le_bytes(), &control)
//! }).unwrap();
//! ```
//! A different lexical region cannot consume the pointer's brand:
//! ```compile_fail
//! use lkjscript::platform::execution::{ExecutionControl, region::*};
//! let control = ExecutionControl::uncancelled();
//! let layout = RegionLayout::<i64>::new([1; 32], 8, 8).unwrap();
//! with_region(256, &control, |_| Ok(()), |mut outer| {
//!     let pointer = outer.allocate(layout, &42_i64.to_le_bytes(), &control)?;
//!     with_region(256, &control, |_| Ok(()), |inner| {
//!         inner.read_cap().read(layout, pointer, &control)?;
//!         Ok(())
//!     })
//! }).unwrap();
//! ```
//! Scoped use returns ordinary results after bulk release:
//! ```
//! use lkjscript::platform::execution::{ExecutionControl, region::*};
//! let control = ExecutionControl::uncancelled();
//! let layout = RegionLayout::<i64>::new([1; 32], 8, 8).unwrap();
//! let result = with_region(256, &control, |_| Ok(()), |mut region| {
//!     let pointer = region.allocate(layout, &42_i64.to_le_bytes(), &control)?;
//!     let frozen = region.freeze(&control)?;
//!     let bytes = frozen.read_cap().read(layout, pointer, &control)?;
//!     Ok(i64::from_le_bytes(bytes.try_into().unwrap()))
//! }).unwrap();
//! assert_eq!(result, 42);
//! ```

use super::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use std::marker::PhantomData;
use std::ops::Range;

pub mod persistence;
pub(crate) mod runtime;

const ALIGNMENT: u64 = 64;
const HEADER_BYTES: u64 = 64;
const CELL_BYTES: u64 = 64;
const MAGIC: &[u8; 8] = b"LKJREGN2";
const PROFILE: u16 = 2;
type Brand<'region> = PhantomData<fn(&'region ()) -> &'region ()>;

/// A byte layout selected by an admitted type derivation, independent of addresses.
pub struct RegionLayout<T = ()> {
    key: [u8; 32],
    bytes: u64,
    alignment: u64,
    marker: PhantomData<fn(T) -> T>,
}

impl<T> Copy for RegionLayout<T> {}
impl<T> Clone for RegionLayout<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> RegionLayout<T> {
    pub fn new(key: [u8; 32], bytes: u64, alignment: u64) -> Result<Self, ExecutionError> {
        validate_layout(bytes, alignment)?;
        Ok(Self {
            key,
            bytes,
            alignment,
            marker: PhantomData,
        })
    }
}

/// Exactly eight bytes. Dereference requires a live, matching region capability.
pub struct RelPtr<'region, T = ()> {
    offset: u64,
    brand: Brand<'region>,
    marker: PhantomData<fn(T) -> T>,
}

impl<T> Copy for RelPtr<'_, T> {}
impl<T> Clone for RelPtr<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> RelPtr<'_, T> {
    pub fn offset(self) -> u64 {
        self.offset
    }
}

/// Exactly sixteen bytes. The count belongs to one complete allocated array.
pub struct RegionSlice<'region, T = ()> {
    pointer: RelPtr<'region, T>,
    length: u64,
}

impl<T> Copy for RegionSlice<'_, T> {}
impl<T> Clone for RegionSlice<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> RegionSlice<'_, T> {
    pub fn offset(self) -> u64 {
        self.pointer.offset
    }

    pub fn len(self) -> u64 {
        self.length
    }

    pub fn is_empty(self) -> bool {
        self.length == 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionObservation {
    /// Requested cell area, including cell headers and alignment padding.
    pub capacity_bytes: u64,
    pub used_cell_bytes: u64,
    pub cells: u64,
    /// Bytes in the directly readable image, excluding unused cell capacity.
    pub image_bytes: u64,
}

#[derive(Clone, Copy)]
struct Header {
    capacity: u64,
    payload: u64,
    used: u64,
    cells: u64,
}

struct Slab {
    // This buffer never grows. A bounded prefix makes the image 64-byte aligned
    // without unsafe casts, allocator assumptions or a second payload allocation.
    allocation: Vec<u8>,
    start: usize,
    header: Header,
}

pub struct UniqueRegion<'region> {
    slab: Slab,
    brand: Brand<'region>,
}

pub struct FrozenRegion<'region> {
    slab: Slab,
    brand: Brand<'region>,
}

/// A scoped read capability; copying it copies no payload or reference count.
#[derive(Clone, Copy)]
pub struct RegionRead<'image, 'region> {
    image: &'image [u8],
    header: Header,
    brand: Brand<'region>,
}

/// Reserve all modeled storage before allocation, then enter a fresh region brand.
/// The callback's result cannot retain pointers or capabilities with that brand.
pub fn with_region<Output>(
    capacity: u64,
    control: &ExecutionControl,
    mut reserve: impl FnMut(u64) -> Result<(), ExecutionError>,
    body: impl for<'region> FnOnce(UniqueRegion<'region>) -> Result<Output, ExecutionError>,
) -> Result<Output, ExecutionError> {
    control.check()?;
    let (payload, allocation) = storage_extent(capacity)?;
    let charge = (allocation as u64)
        .checked_add(std::mem::size_of::<Slab>() as u64)
        .ok_or_else(capacity_error)?;
    reserve(charge)?;
    control.check()?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(allocation)
        .map_err(|_| capacity_error())?;
    bytes.resize(allocation, 0);
    let start = (bytes.as_ptr() as usize).wrapping_neg() & (ALIGNMENT as usize - 1);
    let header = Header {
        capacity,
        payload,
        used: payload,
        cells: 0,
    };
    let mut slab = Slab {
        allocation: bytes,
        start,
        header,
    };
    slab.initialize();
    control.check()?;
    body(UniqueRegion {
        slab,
        brand: PhantomData,
    })
}

/// Admit an immutable, aligned slab image and borrow it without copying its bytes.
/// Complete slot/bitmap admission precedes the callback. Language type/layout and
/// persistence authority must be admitted separately by the evaluator or data owner.
pub fn with_frozen_image<'image, Output>(
    image: &'image [u8],
    maximum_bytes: usize,
    control: &ExecutionControl,
    body: impl for<'region> FnOnce(RegionRead<'image, 'region>) -> Result<Output, ExecutionError>,
) -> Result<Output, ExecutionError> {
    let header = admit_image(image, maximum_bytes, control)?;
    body(RegionRead {
        image,
        header,
        brand: PhantomData,
    })
}

impl Slab {
    fn initialize(&mut self) {
        let start = self.start;
        self.allocation[start..start + 8].copy_from_slice(MAGIC);
        self.allocation[start + 8..start + 10]
            .copy_from_slice(&crate::platform::canonical_schema::SCHEMA_EPOCH.to_le_bytes());
        self.allocation[start + 10..start + 12].copy_from_slice(&PROFILE.to_le_bytes());
        self.set_u64(16, self.header.capacity);
        self.set_u64(24, self.header.payload);
        self.update_cursor();
    }

    fn set_u64(&mut self, offset: usize, value: u64) {
        self.allocation[self.start + offset..self.start + offset + 8]
            .copy_from_slice(&value.to_le_bytes());
    }

    fn update_cursor(&mut self) {
        self.set_u64(32, self.header.used);
        self.set_u64(40, self.header.cells);
    }

    fn image(&self) -> &[u8] {
        &self.allocation[self.start..self.start + self.header.used as usize]
    }

    fn read<'image, 'region>(&'image self) -> RegionRead<'image, 'region> {
        RegionRead {
            image: self.image(),
            header: self.header,
            brand: PhantomData,
        }
    }

    fn allocate<T>(
        &mut self,
        layout: RegionLayout<T>,
        count: u64,
        bytes: &[u8],
        control: &ExecutionControl,
    ) -> Result<u64, ExecutionError> {
        control.check()?;
        let length = layout.bytes.checked_mul(count).ok_or_else(capacity_error)?;
        if u64::try_from(bytes.len()).map_err(|_| capacity_error())? != length {
            return Err(invalid(
                "region_layout",
                "value bytes differ from the exact region layout",
            ));
        }
        let offset = align(
            self.header
                .used
                .checked_add(CELL_BYTES)
                .ok_or_else(capacity_error)?,
            layout.alignment.max(8),
        )?;
        let end = align(
            offset
                .checked_add(length.max(1))
                .ok_or_else(capacity_error)?,
            8,
        )?;
        let cells = self
            .header
            .cells
            .checked_add(1)
            .ok_or_else(capacity_error)?;
        if end
            > self
                .header
                .payload
                .checked_add(self.header.capacity)
                .ok_or_else(capacity_error)?
        {
            return Err(capacity_error());
        }
        // All bounds and fallible admission finish before the first mutation.
        let cell = (offset - CELL_BYTES) as usize;
        let start = self.start;
        self.allocation[start + cell..start + cell + 32].copy_from_slice(&layout.key);
        self.set_u64(cell + 32, layout.bytes);
        self.set_u64(cell + 40, count);
        self.set_u64(cell + 48, layout.alignment);
        self.allocation[start + offset as usize..start + offset as usize + bytes.len()]
            .copy_from_slice(bytes);
        let slot = (offset - self.header.payload) / 8;
        self.allocation[start + HEADER_BYTES as usize + (slot / 8) as usize] |= 1 << (slot % 8);
        self.header.used = end;
        self.header.cells = cells;
        self.update_cursor();
        Ok(offset)
    }
}

impl<'region> UniqueRegion<'region> {
    pub fn read_cap(&self) -> RegionRead<'_, 'region> {
        self.slab.read()
    }

    pub fn allocate<T>(
        &mut self,
        layout: RegionLayout<T>,
        bytes: &[u8],
        control: &ExecutionControl,
    ) -> Result<RelPtr<'region, T>, ExecutionError> {
        self.slab.allocate(layout, 1, bytes, control).map(pointer)
    }

    pub fn allocate_slice<T>(
        &mut self,
        layout: RegionLayout<T>,
        count: u64,
        bytes: &[u8],
        control: &ExecutionControl,
    ) -> Result<RegionSlice<'region, T>, ExecutionError> {
        let offset = self.slab.allocate(layout, count, bytes, control)?;
        Ok(RegionSlice {
            pointer: pointer(offset),
            length: count,
        })
    }

    pub fn write<T>(
        &mut self,
        layout: RegionLayout<T>,
        pointer: RelPtr<'region, T>,
        bytes: &[u8],
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        let range = self.read_cap().range(layout, pointer.offset, 1)?;
        if range.len() != bytes.len() {
            return Err(invalid(
                "region_layout",
                "replacement differs from the exact cell size",
            ));
        }
        self.slab.allocation[self.slab.start + range.start..self.slab.start + range.end]
            .copy_from_slice(bytes);
        Ok(())
    }

    pub fn freeze(
        self,
        control: &ExecutionControl,
    ) -> Result<FrozenRegion<'region>, ExecutionError> {
        control.check()?;
        Ok(FrozenRegion {
            slab: self.slab,
            brand: PhantomData,
        })
    }
}

impl<'region> FrozenRegion<'region> {
    pub fn read_cap(&self) -> RegionRead<'_, 'region> {
        self.slab.read()
    }

    pub fn image(&self) -> &[u8] {
        self.slab.image()
    }
}

impl<'image, 'region> RegionRead<'image, 'region> {
    pub fn observe(self) -> RegionObservation {
        RegionObservation {
            capacity_bytes: self.header.capacity,
            used_cell_bytes: self.header.used - self.header.payload,
            cells: self.header.cells,
            image_bytes: self.header.used,
        }
    }

    pub fn read<T>(
        self,
        layout: RegionLayout<T>,
        pointer: RelPtr<'region, T>,
        control: &ExecutionControl,
    ) -> Result<&'image [u8], ExecutionError> {
        control.check()?;
        self.image
            .get(self.range(layout, pointer.offset, 1)?)
            .ok_or_else(pointer_error)
    }

    pub fn read_slice<T>(
        self,
        layout: RegionLayout<T>,
        slice: RegionSlice<'region, T>,
        control: &ExecutionControl,
    ) -> Result<&'image [u8], ExecutionError> {
        control.check()?;
        self.image
            .get(self.range(layout, slice.pointer.offset, slice.length)?)
            .ok_or_else(pointer_error)
    }

    pub fn read_item<T>(
        self,
        layout: RegionLayout<T>,
        slice: RegionSlice<'region, T>,
        index: u64,
        control: &ExecutionControl,
    ) -> Result<&'image [u8], ExecutionError> {
        let bytes = self.read_slice(layout, slice, control)?;
        if index >= slice.length {
            return Err(pointer_error());
        }
        let offset = index.checked_mul(layout.bytes).ok_or_else(pointer_error)?;
        let start = usize::try_from(offset).map_err(|_| pointer_error())?;
        let end = start
            .checked_add(usize::try_from(layout.bytes).map_err(|_| pointer_error())?)
            .ok_or_else(pointer_error)?;
        bytes.get(start..end).ok_or_else(pointer_error)
    }

    /// Reconstitute a stored offset only through a live capability and exact layout.
    pub fn resolve<T>(
        self,
        layout: RegionLayout<T>,
        offset: u64,
        control: &ExecutionControl,
    ) -> Result<RelPtr<'region, T>, ExecutionError> {
        control.check()?;
        self.range(layout, offset, 1)?;
        Ok(pointer(offset))
    }

    pub fn resolve_slice<T>(
        self,
        layout: RegionLayout<T>,
        offset: u64,
        length: u64,
        control: &ExecutionControl,
    ) -> Result<RegionSlice<'region, T>, ExecutionError> {
        control.check()?;
        self.range(layout, offset, length)?;
        Ok(RegionSlice {
            pointer: pointer(offset),
            length,
        })
    }

    fn range<T>(
        self,
        layout: RegionLayout<T>,
        offset: u64,
        count: u64,
    ) -> Result<Range<usize>, ExecutionError> {
        let relative = offset
            .checked_sub(self.header.payload)
            .ok_or_else(pointer_error)?;
        if !relative.is_multiple_of(8) || offset >= self.header.used || relative < CELL_BYTES {
            return Err(pointer_error());
        }
        let slot = relative / 8;
        let marker = self
            .image
            .get(HEADER_BYTES as usize + (slot / 8) as usize)
            .ok_or_else(pointer_error)?;
        if marker & (1 << (slot % 8)) == 0 {
            return Err(pointer_error());
        }
        let header = (offset - CELL_BYTES) as usize;
        if self.image.get(header..header + 32) != Some(layout.key.as_slice())
            || read_u64(self.image, header + 32)? != layout.bytes
            || read_u64(self.image, header + 40)? != count
            || read_u64(self.image, header + 48)? != layout.alignment
        {
            return Err(invalid(
                "region_layout",
                "relative pointer has another exact layout or count",
            ));
        }
        let length = layout.bytes.checked_mul(count).ok_or_else(pointer_error)?;
        let end = offset
            .checked_add(length)
            .filter(|end| *end <= self.header.used)
            .ok_or_else(pointer_error)?;
        Ok(usize::try_from(offset).map_err(|_| pointer_error())?
            ..usize::try_from(end).map_err(|_| pointer_error())?)
    }
}

fn pointer<'region, T>(offset: u64) -> RelPtr<'region, T> {
    RelPtr {
        offset,
        brand: PhantomData,
        marker: PhantomData,
    }
}

fn storage_extent(capacity: u64) -> Result<(u64, usize), ExecutionError> {
    let payload = payload_start(capacity)?;
    let allocation = payload
        .checked_add(capacity)
        .and_then(|total| total.checked_add(ALIGNMENT - 1))
        .and_then(|total| usize::try_from(total).ok())
        .ok_or_else(capacity_error)?;
    // This is the host allocation-layout boundary, not a configured quota.
    std::alloc::Layout::array::<u8>(allocation).map_err(|_| capacity_error())?;
    Ok((payload, allocation))
}

fn payload_start(capacity: u64) -> Result<u64, ExecutionError> {
    let bitmap = capacity.div_ceil(64);
    align(
        HEADER_BYTES
            .checked_add(bitmap)
            .ok_or_else(capacity_error)?,
        ALIGNMENT,
    )
}

fn align(value: u64, alignment: u64) -> Result<u64, ExecutionError> {
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(capacity_error)
}

fn validate_layout(bytes: u64, alignment: u64) -> Result<(), ExecutionError> {
    if alignment == 0
        || !alignment.is_power_of_two()
        || alignment > ALIGNMENT
        || !bytes.is_multiple_of(alignment)
    {
        return Err(invalid(
            "region_layout",
            "region layouts need a 1..64 power-of-two alignment and aligned stride",
        ));
    }
    Ok(())
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ExecutionError> {
    let end = offset.checked_add(8).ok_or_else(pointer_error)?;
    let value = bytes.get(offset..end).ok_or_else(pointer_error)?;
    let array = <[u8; 8]>::try_from(value).map_err(|_| pointer_error())?;
    Ok(u64::from_le_bytes(array))
}

fn admit_image(
    image: &[u8],
    maximum: usize,
    control: &ExecutionControl,
) -> Result<Header, ExecutionError> {
    control.check()?;
    if image.len() > maximum {
        return Err(capacity_error());
    }
    if image.len() < HEADER_BYTES as usize
        || !(image.as_ptr() as usize).is_multiple_of(ALIGNMENT as usize)
        || image.get(..8) != Some(MAGIC.as_slice())
        || image[8..10] != crate::platform::canonical_schema::SCHEMA_EPOCH.to_le_bytes()
        || image[10..12] != PROFILE.to_le_bytes()
        || image[12..16].iter().any(|byte| *byte != 0)
        || image[48..64].iter().any(|byte| *byte != 0)
    {
        return Err(image_error());
    }
    let header = Header {
        capacity: read_u64(image, 16)?,
        payload: read_u64(image, 24)?,
        used: read_u64(image, 32)?,
        cells: read_u64(image, 40)?,
    };
    let total = header
        .payload
        .checked_add(header.capacity)
        .ok_or_else(image_error)?;
    if header.payload != payload_start(header.capacity)?
        || header.used < header.payload
        || header.used > total
        || u64::try_from(image.len()).map_err(|_| image_error())? != header.used
    {
        return Err(image_error());
    }
    let bitmap_end = HEADER_BYTES
        .checked_add(header.capacity.div_ceil(64))
        .and_then(|end| usize::try_from(end).ok())
        .ok_or_else(image_error)?;
    if image
        .get(bitmap_end..usize::try_from(header.payload).map_err(|_| image_error())?)
        .ok_or_else(image_error)?
        .iter()
        .any(|byte| *byte != 0)
    {
        return Err(image_error());
    }
    let mut previous = header.payload;
    let mut cells = 0_u64;
    for (index, markers) in image[HEADER_BYTES as usize..bitmap_end]
        .iter()
        .copied()
        .enumerate()
    {
        if index.is_multiple_of(1024) {
            control.check()?;
        }
        for bit in 0..8 {
            if markers & (1 << bit) == 0 {
                continue;
            }
            let offset = u64::try_from(index)
                .ok()
                .and_then(|index| index.checked_mul(8))
                .and_then(|index| index.checked_add(bit))
                .and_then(|slot| slot.checked_mul(8))
                .and_then(|offset| header.payload.checked_add(offset))
                .ok_or_else(image_error)?;
            if offset
                < header
                    .payload
                    .checked_add(CELL_BYTES)
                    .ok_or_else(image_error)?
                || offset >= header.used
            {
                return Err(image_error());
            }
            let cell = (offset - CELL_BYTES) as usize;
            let stride = read_u64(image, cell + 32)?;
            let count = read_u64(image, cell + 40)?;
            let alignment = read_u64(image, cell + 48)?;
            validate_layout(stride, alignment)?;
            let expected = align(
                previous.checked_add(CELL_BYTES).ok_or_else(image_error)?,
                alignment.max(8),
            )?;
            let length = stride.checked_mul(count).ok_or_else(image_error)?;
            let payload_end = offset.checked_add(length).ok_or_else(image_error)?;
            let end = align(
                offset.checked_add(length.max(1)).ok_or_else(image_error)?,
                8,
            )?;
            if offset != expected
                || end > header.used
                || read_u64(image, cell + 56)? != 0
                || image[previous as usize..cell].iter().any(|byte| *byte != 0)
                || image[payload_end as usize..end as usize]
                    .iter()
                    .any(|byte| *byte != 0)
            {
                return Err(image_error());
            }
            previous = end;
            cells = cells.checked_add(1).ok_or_else(image_error)?;
        }
    }
    if previous != header.used || cells != header.cells {
        return Err(image_error());
    }
    control.check()?;
    Ok(header)
}

fn invalid(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Trap, code, message)
}
fn pointer_error() -> ExecutionError {
    invalid(
        "region_pointer",
        "relative address is not an allocated cell of this live region",
    )
}
fn image_error() -> ExecutionError {
    invalid(
        "region_image",
        "region image has invalid or noncanonical allocation metadata",
    )
}
fn capacity_error() -> ExecutionError {
    ExecutionError::resource(
        "region_capacity",
        "region storage exceeds admitted capacity or relative address bounds",
    )
}

#[cfg(test)]
mod tests;
