fn measure_constants(
    chunk: &Chunk,
    mut metadata_bytes: usize,
    mut encoded_bytes: usize,
) -> Result<(usize, usize)> {
    for (index, constant) in chunk.constants.iter().enumerate() {
        encoded_bytes = checked_add(encoded_bytes, 1, "encoded byte size")?;
        match constant {
            Constant::I64(_) | Constant::F64(_) => {
                encoded_bytes = checked_add(encoded_bytes, 8, "encoded byte size")?;
            }
            Constant::Str(text) | Constant::Symbol(text) => {
                encoded_bytes = checked_add(encoded_bytes, text.len(), "encoded byte size")?;
            }
            Constant::StaticBytes(bytes) => {
                metadata_bytes = checked_add(metadata_bytes, 4, "metadata byte size")?;
                encoded_bytes = checked_add(encoded_bytes, 4, "encoded byte size")?;
                encoded_bytes = checked_add(encoded_bytes, bytes.len(), "encoded byte size")?;
            }
            Constant::Proto(proto) => {
                if usize::try_from(*proto)
                    .ok()
                    .is_none_or(|proto| proto >= chunk.protos.len())
                {
                    return Err(Error::msg(format!(
                        "constant {index} references prototype {proto} out of range"
                    )));
                }
                encoded_bytes = checked_add(encoded_bytes, 4, "encoded byte size")?;
            }
        }
    }
    Ok((metadata_bytes, encoded_bytes))
}
