//! Current physical codecs preserve canonical object bytes and bound decompression first.

use super::*;
use flate2::{Compress, Compression, Decompress, FlushCompress, FlushDecompress, Status};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PackEncoding {
    Raw,
    Deflate,
}

impl PackEncoding {
    pub(super) fn from_flags(flags: u16) -> Result<Self, StoreError> {
        match flags {
            0 => Ok(Self::Raw),
            1 => Ok(Self::Deflate),
            _ => Err(corrupt(
                "pack_contract",
                "pack physical codec flags are unknown",
            )),
        }
    }

    pub(super) const fn flags(self) -> u16 {
        match self {
            Self::Raw => 0,
            Self::Deflate => 1,
        }
    }

    pub(super) fn validate_lengths(
        self,
        domain: ObjectDomain,
        encoded: u64,
        raw: u64,
    ) -> Result<(), StoreError> {
        if raw > domain.maximum_bytes() as u64 || raw > contract::MAXIMUM_PACK_ENTRY_BYTES as u64 {
            return Err(pack_error(
                StoreErrorClass::Resource,
                "pack_entry_size",
                "decoded pack entry exceeds its object-domain bound",
            ));
        }
        let valid = match self {
            Self::Raw => encoded == raw,
            Self::Deflate => encoded == raw || (encoded >= 8 && encoded < raw),
        };
        if !valid {
            return Err(corrupt(
                "pack_entry_layout",
                "pack codec and bounded entry lengths disagree",
            ));
        }
        Ok(())
    }

    pub(super) fn encode(self, bytes: Vec<u8>) -> Result<Vec<u8>, StoreError> {
        if self == Self::Raw {
            return Ok(bytes);
        }
        let bound = usize_from_u64(encoded_bound(bytes.len() as u64)?, "pack_encoding_length")?;
        let mut output = buffer(bound)?;
        let mut encoder = Compress::new(Compression::new(6), true);
        let status = encoder
            .compress(&bytes, &mut output, FlushCompress::Finish)
            .map_err(|error| {
                corrupt(
                    "pack_compression",
                    format!("pack compression failed: {error}"),
                )
            })?;
        if status != Status::StreamEnd || encoder.total_in() != bytes.len() as u64 {
            return Err(pack_error(
                StoreErrorClass::Resource,
                "pack_compression_capacity",
                "pack encoder did not finish within its bounded output",
            ));
        }
        output.truncate(usize_from_u64(encoder.total_out(), "pack_encoding_length")?);
        // Equal lengths denote raw bytes; compressed entries must strictly shrink.
        // This keeps tiny or incompressible objects from growing during compaction.
        Ok(if output.len() < bytes.len() {
            output
        } else {
            bytes
        })
    }

    pub(super) fn decode(self, bytes: &[u8], raw: u64) -> Result<Vec<u8>, StoreError> {
        let length = usize_from_u64(raw, "pack_decoding_length")?;
        if self == Self::Raw || bytes.len() == length {
            return Ok(bytes.to_vec());
        }
        // One extra byte distinguishes an expanding stream from exact declared output.
        let capacity = length
            .checked_add(1)
            .ok_or_else(|| corrupt("pack_decoding_length", "decoded size overflowed"))?;
        let mut output = buffer(capacity)?;
        let mut decoder = Decompress::new(true);
        let status = decoder
            .decompress(bytes, &mut output, FlushDecompress::Finish)
            .map_err(|error| {
                corrupt(
                    "pack_decompression",
                    format!("invalid compressed object: {error}"),
                )
            })?;
        if status != Status::StreamEnd
            || decoder.total_in() != bytes.len() as u64
            || decoder.total_out() != raw
        {
            return Err(corrupt(
                "pack_decompression_length",
                "compressed object is truncated, trailing, or differs from its declared decoded size",
            ));
        }
        output.truncate(length);
        Ok(output)
    }
}

fn encoded_bound(raw: u64) -> Result<u64, StoreError> {
    raw.checked_mul(2)
        .and_then(|value| value.checked_add(64))
        .ok_or_else(|| corrupt("pack_encoding_length", "encoded size bound overflowed"))
}

fn buffer(length: usize) -> Result<Vec<u8>, StoreError> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|_| {
        pack_error(
            StoreErrorClass::Resource,
            "pack_codec_allocation",
            "pack codec allocation exceeds available capacity",
        )
    })?;
    bytes.resize(length, 0);
    Ok(bytes)
}
