//! Digest-bound relative-offset images in the existing transactional DataStore.
//! The mmap names denote the slab format; this module uses safe owned buffers.

use super::{
    ALIGNMENT, ExecutionControl, ExecutionError, ExecutionFailureClass, HEADER_BYTES, RegionRead,
    Slab, admit_image, capacity_error, image_error, invalid,
};
use crate::platform::data::{DataExpectation, DataKey, DataTransaction};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};

const HASH_DOMAIN: &str = "lkjscript.region.image.v2";
const CHUNK_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionImageDigest([u8; 32]);

impl RegionImageDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// An admitted immutable image with one aligned allocation and no node decoding.
pub struct RegionImage {
    slab: Slab,
    digest: RegionImageDigest,
}

impl RegionImage {
    pub(super) fn into_slab(self) -> Slab {
        self.slab
    }

    pub fn from_bytes(
        image: &[u8],
        expected: RegionImageDigest,
        maximum_bytes: usize,
        control: &ExecutionControl,
        mut reserve: impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if image.len() > maximum_bytes {
            return Err(capacity_error());
        }
        if image.len() < HEADER_BYTES as usize {
            return Err(image_error());
        }
        if digest(image, control)? != expected {
            return Err(invalid(
                "region_image_digest",
                "region image digest does not match",
            ));
        }
        let allocation = image
            .len()
            .checked_add(ALIGNMENT as usize - 1)
            .ok_or_else(capacity_error)?;
        let charge = (allocation as u64)
            .checked_add(std::mem::size_of::<Self>() as u64)
            .ok_or_else(capacity_error)?;
        reserve(charge)?;
        control.check()?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(allocation)
            .map_err(|_| capacity_error())?;
        bytes.resize(allocation, 0);
        let start = (bytes.as_ptr() as usize).wrapping_neg() & (ALIGNMENT as usize - 1);
        for (source, destination) in image
            .chunks(CHUNK_BYTES)
            .zip(bytes[start..start + image.len()].chunks_mut(CHUNK_BYTES))
        {
            control.check()?;
            destination.copy_from_slice(source);
        }
        let header = admit_image(&bytes[start..start + image.len()], maximum_bytes, control)?;
        Ok(Self {
            slab: Slab {
                allocation: bytes,
                start,
                header,
            },
            digest: expected,
        })
    }

    pub fn image(&self) -> &[u8] {
        self.slab.image()
    }

    pub fn digest(&self) -> RegionImageDigest {
        self.digest
    }

    pub fn with_region<'image, Output>(
        &'image self,
        control: &ExecutionControl,
        body: impl for<'region> FnOnce(RegionRead<'image, 'region>) -> Result<Output, ExecutionError>,
    ) -> Result<Output, ExecutionError> {
        control.check()?;
        body(self.slab.read())
    }
}

pub fn digest(
    image: &[u8],
    control: &ExecutionControl,
) -> Result<RegionImageDigest, ExecutionError> {
    control.check()?;
    let mut hash = blake3::Hasher::new_derive_key(HASH_DOMAIN);
    hash.update(&(image.len() as u64).to_le_bytes());
    for chunk in image.chunks(CHUNK_BYTES) {
        control.check()?;
        hash.update(chunk);
    }
    control.check()?;
    Ok(RegionImageDigest(*hash.finalize().as_bytes()))
}

/// Stage exact image bytes under the transaction's normal key/namespace/CAS
/// rules. The caller owns commit and its visibility outcome; this does not commit.
/// A digest is returned only for a successful staged write.
pub fn save(
    transaction: &mut DataTransaction,
    space: &str,
    key: &DataKey,
    read: RegionRead<'_, '_>,
    expected: DataExpectation,
    control: &ExecutionControl,
    mut reserve: impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<Option<RegionImageDigest>, ExecutionError> {
    control.check()?;
    transaction.get_ref(space, key).map_err(storage_error)?;
    if read.image.len() > transaction.limits().maximum_value_bytes {
        return Err(capacity_error());
    }
    let digest = digest(read.image, control)?;
    reserve(u64::try_from(read.image.len()).ok()
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Vec<u8>>() as u64))
        .ok_or_else(capacity_error)?)?;
    control.check()?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(read.image.len())
        .map_err(|_| capacity_error())?;
    for chunk in read.image.chunks(CHUNK_BYTES) {
        control.check()?;
        bytes.extend_from_slice(chunk);
    }
    control.check()?;
    transaction
        .put(space, key, bytes, expected)
        .map_err(storage_error)
        .map(|saved| saved.then_some(digest))
}

/// Load from one immutable transaction snapshot. The expected digest binds the
/// selected image independently of later writes to the same application key.
pub fn load(
    transaction: &DataTransaction,
    space: &str,
    key: &DataKey,
    expected: RegionImageDigest,
    maximum_bytes: usize,
    control: &ExecutionControl,
    reserve: impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<Option<RegionImage>, ExecutionError> {
    control.check()?;
    transaction
        .get_ref(space, key)
        .map_err(storage_error)?
        .map(|entry| {
            RegionImage::from_bytes(&entry.value, expected, maximum_bytes, control, reserve)
        })
        .transpose()
}

fn storage_error(error: Diagnostic) -> ExecutionError {
    let class = match error.class {
        DiagnosticClass::Resource => ExecutionFailureClass::Resource,
        DiagnosticClass::Cancelled => ExecutionFailureClass::Cancelled,
        DiagnosticClass::Capability => ExecutionFailureClass::Capability,
        DiagnosticClass::Infrastructure => ExecutionFailureClass::Infrastructure,
        DiagnosticClass::Source | DiagnosticClass::Semantic | DiagnosticClass::Corrupt => {
            ExecutionFailureClass::Trap
        }
    };
    ExecutionError::new(class, error.code, error.message)
}

#[cfg(test)]
mod tests;
