mod decode;
mod decode_maps;
mod encode;
mod error;
mod heap_decode;
mod heap_encode;
mod reader;
#[cfg(test)]
mod tests;
mod values;
mod writer;

pub use error::ImageCodecError;

use self::{reader::Reader, writer::Writer};
use super::InstallableImage;

const MAGIC: &[u8; 8] = b"LKJNIC01";
const ENVELOPE_BYTES: usize = 112;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageCodecLimits {
    pub max_encoded_bytes: usize,
    pub max_records: u64,
}

impl Default for ImageCodecLimits {
    fn default() -> Self {
        Self {
            max_encoded_bytes: 16 * 1024 * 1024,
            max_records: 100_000,
        }
    }
}

pub fn encode_image(
    image: &InstallableImage,
    key_digest: [u8; 32],
    limits: ImageCodecLimits,
) -> Result<Vec<u8>, ImageCodecError> {
    let payload_maximum = limits
        .max_encoded_bytes
        .checked_sub(ENVELOPE_BYTES)
        .ok_or_else(|| ImageCodecError::new("image encoding limit is too small"))?;
    let payload = encode::payload(
        image,
        ImageCodecLimits {
            max_encoded_bytes: payload_maximum,
            ..limits
        },
    )?;
    let mut output = Writer::new(limits.max_encoded_bytes);
    output.fixed(MAGIC)?;
    output.fixed(&lkjscript_contracts::NATIVE_IMAGE_CACHE_DIGEST.as_bytes())?;
    output.fixed(&key_digest)?;
    output.u64(
        u64::try_from(payload.len())
            .map_err(|_| ImageCodecError::new("image payload length overflow"))?,
    )?;
    output.fixed(&payload)?;
    let mut bytes = output.finish();
    let digest = lkjscript_contracts::sha256(&bytes);
    bytes.extend_from_slice(&digest);
    Ok(bytes)
}

pub fn decode_image(
    bytes: &[u8],
    expected_key: [u8; 32],
    limits: ImageCodecLimits,
) -> Result<InstallableImage, ImageCodecError> {
    if bytes.len() < ENVELOPE_BYTES || bytes.len() > limits.max_encoded_bytes {
        return Err(ImageCodecError::new("encoded image byte limit mismatch"));
    }
    let content_end = bytes.len() - 32;
    let (content, digest) = bytes.split_at(content_end);
    if lkjscript_contracts::sha256(content) != digest {
        return Err(ImageCodecError::new("encoded image digest mismatch"));
    }
    let mut input = Reader::new(content, limits.max_records);
    if input.fixed::<8>()? != *MAGIC {
        return Err(ImageCodecError::new("encoded image magic mismatch"));
    }
    if input.fixed::<32>()? != lkjscript_contracts::NATIVE_IMAGE_CACHE_DIGEST.as_bytes() {
        return Err(ImageCodecError::new("encoded image contract mismatch"));
    }
    if input.fixed::<32>()? != expected_key {
        return Err(ImageCodecError::new("encoded image key mismatch"));
    }
    let payload = input.bytes()?;
    if !input.done() {
        return Err(ImageCodecError::new("trailing encoded image bytes"));
    }
    decode::payload(payload, limits)
}
