use super::{ContainerError, ELF_INSPECTOR, LINKAGE_MODEL, model::ElfIdentity};

pub fn inspect_static_elf_bytes(bytes: &[u8]) -> Result<ElfIdentity, ContainerError> {
    if bytes.len() < 64
        || bytes[0..4] != [0x7f, b'E', b'L', b'F']
        || bytes[4] != 2
        || bytes[5] != 1
        || bytes[6] != 1
    {
        return Err(ContainerError::corrupt(
            "candidate is not a complete little-endian ELF64 object",
        ));
    }
    let object_type = u16_at(bytes, 16)?;
    if !matches!(object_type, 2 | 3) || u16_at(bytes, 18)? != 62 || u32_at(bytes, 20)? != 1 {
        return Err(ContainerError::corrupt(
            "candidate is not an x86-64 ELF executable or position-independent executable",
        ));
    }
    if u16_at(bytes, 52)? != 64 || u16_at(bytes, 54)? != 56 {
        return Err(ContainerError::corrupt(
            "candidate ELF header uses a noncanonical header size",
        ));
    }
    let program_offset = usize_from_u64(u64_at(bytes, 32)?, "program-header offset")?;
    let program_count = usize::from(u16_at(bytes, 56)?);
    if program_count == 0 || program_count > 128 {
        return Err(ContainerError::corrupt(
            "candidate ELF program-header count is outside 1..=128",
        ));
    }
    let program_bytes = program_count
        .checked_mul(56)
        .and_then(|length| program_offset.checked_add(length))
        .ok_or_else(|| ContainerError::corrupt("candidate ELF program headers overflow"))?;
    if program_bytes > bytes.len() {
        return Err(ContainerError::corrupt(
            "candidate ELF program headers are truncated",
        ));
    }
    let mut load_headers = 0_u32;
    let mut interpreter_headers = 0_u32;
    let mut dynamic_range = None;
    for index in 0..program_count {
        let base = program_offset + index * 56;
        let kind = u32_at(bytes, base)?;
        let offset = usize_from_u64(u64_at(bytes, base + 8)?, "segment offset")?;
        let file_size = usize_from_u64(u64_at(bytes, base + 32)?, "segment size")?;
        let end = offset
            .checked_add(file_size)
            .ok_or_else(|| ContainerError::corrupt("candidate ELF segment range overflow"))?;
        if end > bytes.len() {
            return Err(ContainerError::corrupt(
                "candidate ELF segment is truncated",
            ));
        }
        match kind {
            1 => load_headers = load_headers.saturating_add(1),
            2 => {
                if dynamic_range.replace((offset, file_size)).is_some() {
                    return Err(ContainerError::corrupt(
                        "candidate ELF contains multiple dynamic program headers",
                    ));
                }
            }
            3 => interpreter_headers = interpreter_headers.saturating_add(1),
            _ => {}
        }
    }
    if load_headers == 0 {
        return Err(ContainerError::corrupt(
            "candidate ELF contains no loadable program header",
        ));
    }
    let mut dynamic_entries = 0_u32;
    let mut needed_libraries = 0_u32;
    let mut glibc_version_requirements = 0_u32;
    if let Some((offset, file_size)) = dynamic_range {
        if file_size == 0 || file_size % 16 != 0 {
            return Err(ContainerError::corrupt(
                "candidate ELF dynamic table has a noncanonical length",
            ));
        }
        let mut terminated = false;
        let (entries, remainder) = bytes[offset..offset + file_size].as_chunks::<16>();
        if !remainder.is_empty() {
            return Err(ContainerError::corrupt(
                "candidate ELF dynamic table has trailing bytes",
            ));
        }
        for entry in entries {
            let tag =
                i64::from_le_bytes(entry[0..8].try_into().map_err(|_| {
                    ContainerError::corrupt("candidate ELF dynamic tag is truncated")
                })?);
            let value = u64::from_le_bytes(entry[8..16].try_into().map_err(|_| {
                ContainerError::corrupt("candidate ELF dynamic value is truncated")
            })?);
            if terminated {
                if tag != 0 || value != 0 {
                    return Err(ContainerError::corrupt(
                        "candidate ELF dynamic table has trailing entries after DT_NULL",
                    ));
                }
                continue;
            }
            dynamic_entries = dynamic_entries.saturating_add(1);
            match tag {
                0 => terminated = true,
                1 => needed_libraries = needed_libraries.saturating_add(1),
                0x6fff_fffe | 0x6fff_ffff => {
                    glibc_version_requirements = glibc_version_requirements.saturating_add(1)
                }
                _ => {}
            }
        }
        if !terminated {
            return Err(ContainerError::corrupt(
                "candidate ELF dynamic table is missing DT_NULL",
            ));
        }
    }
    let identity = ElfIdentity {
        class: "ELF64".to_owned(),
        machine: "x86-64".to_owned(),
        object_type: if object_type == 3 {
            "position-independent-executable".to_owned()
        } else {
            "executable".to_owned()
        },
        inspector: ELF_INSPECTOR.to_owned(),
        program_headers: u32::try_from(program_count)
            .map_err(|_| ContainerError::corrupt("candidate ELF program-header count overflow"))?,
        load_headers,
        dynamic_entries,
        interpreter_headers,
        needed_libraries,
        glibc_version_requirements,
        position_independent: object_type == 3,
        runtime_linkage: LINKAGE_MODEL.to_owned(),
    };
    if identity.interpreter_headers != 0
        || identity.needed_libraries != 0
        || identity.glibc_version_requirements != 0
    {
        return Err(ContainerError::corrupt(
            "candidate ELF is not self-contained static linkage",
        ));
    }
    Ok(identity)
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, ContainerError> {
    let field = bytes
        .get(offset..offset.saturating_add(2))
        .ok_or_else(|| ContainerError::corrupt("candidate ELF u16 field is truncated"))?;
    Ok(u16::from_le_bytes(field.try_into().map_err(|_| {
        ContainerError::corrupt("candidate ELF u16 field is malformed")
    })?))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, ContainerError> {
    let field = bytes
        .get(offset..offset.saturating_add(4))
        .ok_or_else(|| ContainerError::corrupt("candidate ELF u32 field is truncated"))?;
    Ok(u32::from_le_bytes(field.try_into().map_err(|_| {
        ContainerError::corrupt("candidate ELF u32 field is malformed")
    })?))
}

fn u64_at(bytes: &[u8], offset: usize) -> Result<u64, ContainerError> {
    let field = bytes
        .get(offset..offset.saturating_add(8))
        .ok_or_else(|| ContainerError::corrupt("candidate ELF u64 field is truncated"))?;
    Ok(u64::from_le_bytes(field.try_into().map_err(|_| {
        ContainerError::corrupt("candidate ELF u64 field is malformed")
    })?))
}

fn usize_from_u64(value: u64, label: &str) -> Result<usize, ContainerError> {
    usize::try_from(value)
        .map_err(|_| ContainerError::corrupt(format!("candidate ELF {label} overflow")))
}
