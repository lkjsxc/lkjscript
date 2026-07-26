use super::*;
use crate::*;

fn image() -> Result<InstallableImage, Box<dyn std::error::Error>> {
    let buffer = ValueType::Reference(ReferenceType::Buf);
    let mut plan = MachinePlanBuilder::new();
    let function = plan.declare_function(
        SourceFunctionId::new(9),
        Signature::new(vec![buffer], ValueType::Unit)?,
    )?;
    let mut builder = plan.function_builder(function)?;
    let entry = builder.create_block()?;
    builder.set_entry(entry)?;
    let input = builder.parameter(0)?;
    let index = builder.i64_const(entry, 0)?;
    let byte = builder.i64_const(entry, 1)?;
    let descriptor = HeapCallDescriptor::new(
        HeapOperation::BufSet,
        vec![buffer, ValueType::I64, ValueType::I64],
        ValueType::Unit,
        AllocationClass::None,
        StoreClass::Scalar,
    )?;
    let result = builder.heap_call(entry, descriptor, vec![input, index, byte])?;
    builder.return_value(entry, result)?;
    plan.define_function(builder.finish())?;
    Ok(encode(
        plan.verify(BackendLimits::default())?,
        EncodingConfig::default(),
    )?)
}

#[test]
fn image_codec_is_canonical_and_roundtrips_complete_metadata(
) -> Result<(), Box<dyn std::error::Error>> {
    let key = [7; 32];
    let encoded = encode_image(&image()?, key, ImageCodecLimits::default())?;
    let decoded = decode_image(&encoded, key, ImageCodecLimits::default())?;
    assert_eq!(decoded.validate_integrity(), Ok(()));
    assert_eq!(
        encode_image(&decoded, key, ImageCodecLimits::default())?,
        encoded
    );
    Ok(())
}

#[test]
fn image_codec_rejects_wrong_key_corruption_truncation_and_bounds(
) -> Result<(), Box<dyn std::error::Error>> {
    let key = [11; 32];
    let encoded = encode_image(&image()?, key, ImageCodecLimits::default())?;
    assert!(decode_image(&encoded, [12; 32], ImageCodecLimits::default()).is_err());
    let mut corrupt = encoded.clone();
    corrupt[80] ^= 1;
    assert!(decode_image(&corrupt, key, ImageCodecLimits::default()).is_err());
    let content_end = corrupt.len() - 32;
    let digest = lkjscript_contracts::sha256(&corrupt[..content_end]);
    corrupt[content_end..].copy_from_slice(&digest);
    assert!(decode_image(&corrupt, key, ImageCodecLimits::default()).is_err());
    for length in 0..encoded.len() {
        assert!(decode_image(&encoded[..length], key, ImageCodecLimits::default()).is_err());
    }
    let limits = ImageCodecLimits {
        max_encoded_bytes: encoded.len() - 1,
        ..ImageCodecLimits::default()
    };
    assert!(decode_image(&encoded, key, limits).is_err());
    let records = ImageCodecLimits {
        max_records: 0,
        ..ImageCodecLimits::default()
    };
    assert!(decode_image(&encoded, key, records).is_err());
    Ok(())
}
