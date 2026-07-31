use super::*;

pub const MAX_PATH_BYTES: usize = 4095;

pub fn copy_validated_path(bytes: &[u8]) -> Result<Vec<u8>> {
    validate_path(bytes)?;
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| Error::msg("Path allocation failed"))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

pub fn validate_path(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_PATH_BYTES {
        return Err(Error::msg("Path must contain 1 through 4095 bytes"));
    }
    if bytes.first() != Some(&b'/') {
        return Err(Error::msg("Path must be absolute"));
    }
    if bytes.contains(&0) {
        return Err(Error::msg("Path contains an interior NUL"));
    }
    Ok(())
}
