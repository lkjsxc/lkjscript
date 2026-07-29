const RECORD_MAGIC: &[u8] = b"lkjscript.application-record\0";

fn encode_record(record: &DurableApplication) -> Result<Vec<u8>, CoordinatorError> {
    let root = record
        .package_root
        .to_str()
        .ok_or(CoordinatorError::InvalidApplicationRegistry)?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(RECORD_MAGIC);
    bytes.extend_from_slice(&record.id.to_le_bytes());
    put_text(&mut bytes, &record.name, 64)?;
    bytes.extend_from_slice(&record.package);
    put_text(&mut bytes, root, 4_096)?;
    put_text(&mut bytes, record.entry.as_str(), 4_096)?;
    bytes.push(
        u8::try_from(record.capabilities.len())
            .map_err(|_| CoordinatorError::InvalidApplicationRegistry)?,
    );
    for capability in &record.capabilities {
        bytes.push(*capability as u8);
    }
    bytes.extend_from_slice(&record.max_concurrent.to_le_bytes());
    bytes.extend_from_slice(&record.max_total.to_le_bytes());
    bytes.push(u8::from(record.desired_running));
    Ok(bytes)
}

fn decode_record(bytes: &[u8]) -> Result<DurableApplication, CoordinatorError> {
    let mut input = RecordInput::new(bytes);
    if input.take(RECORD_MAGIC.len())? != RECORD_MAGIC {
        return Err(CoordinatorError::InvalidApplicationRegistry);
    }
    let id = input.nonzero()?;
    let name = input.text(64)?;
    let package = input.array::<32>()?;
    if package == [0; 32] {
        return Err(CoordinatorError::InvalidApplicationRegistry);
    }
    let package_root = PathBuf::from(input.text(4_096)?);
    let entry = ApplicationPath::parse(input.text(4_096)?)
        .map_err(|_| CoordinatorError::InvalidApplicationRegistry)?;
    let count = usize::from(input.u8()?);
    if count > CapabilityKind::ALL.len() {
        return Err(CoordinatorError::InvalidApplicationRegistry);
    }
    let mut capabilities = Vec::with_capacity(count);
    for _ in 0..count {
        capabilities.push(
            CapabilityKind::from_tag(input.u8()?)
                .ok_or(CoordinatorError::InvalidApplicationRegistry)?,
        );
    }
    let max_concurrent = input.u16()?;
    let max_total = input.u64()?;
    let desired_running = input.boolean()?;
    input.finish()?;
    if name.is_empty()
        || !capabilities.windows(2).all(|pair| pair[0] < pair[1])
        || !(1..=64).contains(&max_concurrent)
        || max_total == 0
    {
        return Err(CoordinatorError::InvalidApplicationRegistry);
    }
    Ok(DurableApplication {
        id,
        name,
        package,
        package_root,
        entry,
        capabilities,
        max_concurrent,
        max_total,
        desired_running,
    })
}

fn put_text(
    bytes: &mut Vec<u8>,
    value: &str,
    maximum: usize,
) -> Result<(), CoordinatorError> {
    if value.is_empty() || value.len() > maximum {
        return Err(CoordinatorError::InvalidApplicationRegistry);
    }
    bytes.extend_from_slice(
        &u16::try_from(value.len())
            .map_err(|_| CoordinatorError::InvalidApplicationRegistry)?
            .to_le_bytes(),
    );
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

struct RecordInput<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

include!("input.rs");
