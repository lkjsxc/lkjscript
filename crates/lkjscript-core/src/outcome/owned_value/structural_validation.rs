pub(super) fn validate_structural_snapshot(
    value: &SemanticValue,
    limits: StructuralSnapshotLimits,
    work: SnapshotWork,
) -> Result<StructuralSnapshotMetrics> {
    let limits = limits.validate()?;
    let mut validator = StructuralValidator {
        limits,
        work,
        metrics: StructuralSnapshotMetrics::default(),
        active: Vec::new(),
    };
    validator
        .active
        .try_reserve_exact(usize::from(limits.max_depth))
        .map_err(|_| Error::msg("structural snapshot validation allocation failed"))?;
    validator.node(value, 1)?;
    validator.metrics.decode_work = validator.metrics.encode_work;
    Ok(validator.metrics)
}

struct StructuralValidator {
    limits: StructuralSnapshotLimits,
    work: SnapshotWork,
    metrics: StructuralSnapshotMetrics,
    active: Vec<*const SemanticValue>,
}

impl StructuralValidator {
    fn node(&mut self, value: &SemanticValue, depth: u16) -> Result<()> {
        if depth > self.limits.max_depth {
            return Err(Error::msg("structural snapshot depth exceeds bound"));
        }
        let address = std::ptr::from_ref(value);
        if self.active.contains(&address) {
            return Err(Error::msg("cyclic structural snapshot"));
        }
        self.active.push(address);
        self.metrics.nodes = self
            .metrics
            .nodes
            .checked_add(1)
            .ok_or_else(|| Error::msg("structural snapshot node count overflow"))?;
        if self.metrics.nodes > self.limits.max_nodes {
            return Err(Error::msg("structural snapshot nodes exceed bound"));
        }
        self.charge_work(1)?;
        let expected = match &value.payload {
            SemanticPayload::Inline(inline) => inline_kind(*inline),
            SemanticPayload::Static(_) => StructuralKind::Static,
            SemanticPayload::String(bytes) => {
                std::str::from_utf8(bytes)
                    .map_err(|_| Error::msg("structural snapshot string is not UTF-8"))?;
                self.charge_bytes(bytes.len(), ByteClass::String)?;
                StructuralKind::String
            }
            SemanticPayload::Path(bytes) => {
                validate_snapshot_path(bytes)?;
                self.charge_bytes(bytes.len(), ByteClass::Path)?;
                StructuralKind::Path
            }
            SemanticPayload::Bytes(bytes) => {
                self.charge_bytes(bytes.len(), ByteClass::Other)?;
                StructuralKind::Bytes
            }
            SemanticPayload::ByteVector(bytes) => {
                self.charge_bytes(bytes.len(), ByteClass::Other)?;
                StructuralKind::ByteVector
            }
            SemanticPayload::Product(fields) => {
                self.fields(fields, depth)?;
                StructuralKind::Product
            }
            SemanticPayload::Enum { active_payload, .. } => {
                self.fields(active_payload, depth)?;
                StructuralKind::Enum
            }
        };
        require_snapshot_kind(value.value_type, expected)?;
        self.active.pop();
        Ok(())
    }

    fn fields(&mut self, fields: &[SemanticValue], depth: u16) -> Result<()> {
        let count = u32::try_from(fields.len())
            .map_err(|_| Error::msg("structural snapshot field count overflow"))?;
        self.metrics.fields = self
            .metrics
            .fields
            .checked_add(count)
            .ok_or_else(|| Error::msg("structural snapshot field count overflow"))?;
        if self.metrics.fields > self.limits.max_fields {
            return Err(Error::msg("structural snapshot fields exceed bound"));
        }
        self.charge_work(u64::from(count))?;
        for field in fields {
            self.node(field, depth + 1)?;
        }
        Ok(())
    }

    fn charge_bytes(&mut self, length: usize, class: ByteClass) -> Result<()> {
        let length = u64::try_from(length)
            .map_err(|_| Error::msg("structural snapshot byte count overflow"))?;
        self.metrics.aggregate_bytes = checked_add(self.metrics.aggregate_bytes, length)?;
        if matches!(class, ByteClass::String) {
            self.metrics.string_bytes = checked_add(self.metrics.string_bytes, length)?;
        }
        if matches!(class, ByteClass::Path) {
            self.metrics.path_bytes = checked_add(self.metrics.path_bytes, length)?;
        }
        if self.metrics.aggregate_bytes > self.limits.max_aggregate_bytes
            || self.metrics.string_bytes > self.limits.max_string_bytes
            || self.metrics.path_bytes > self.limits.max_path_bytes
        {
            return Err(Error::msg("structural snapshot bytes exceed bound"));
        }
        self.charge_work(length)
    }

    fn charge_work(&mut self, amount: u64) -> Result<()> {
        self.metrics.encode_work = checked_add(self.metrics.encode_work, amount)?;
        let limit = match self.work {
            SnapshotWork::Encode => self.limits.max_encode_work,
            SnapshotWork::Decode => self.limits.max_decode_work,
        };
        if self.metrics.encode_work > limit {
            return Err(Error::msg("structural snapshot work exceeds bound"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ByteClass {
    String,
    Path,
    Other,
}

fn checked_add(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| Error::msg("structural snapshot accounting overflow"))
}

pub(super) fn require_snapshot_kind(actual: StructuralType, expected: StructuralKind) -> Result<()> {
    if actual.kind == expected {
        Ok(())
    } else {
        Err(Error::msg("structural snapshot type and payload disagree"))
    }
}

fn inline_kind(value: InlineStructuralValue) -> StructuralKind {
    match value {
        InlineStructuralValue::Unit => StructuralKind::Unit,
        InlineStructuralValue::Bool(_) => StructuralKind::Bool,
        InlineStructuralValue::I64(_) => StructuralKind::I64,
        InlineStructuralValue::F64Bits(_) => StructuralKind::F64,
    }
}

pub(super) fn validate_snapshot_path(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty()
        || bytes.len() > MAX_STRUCTURAL_SNAPSHOT_PATH_BYTES
        || bytes.first() != Some(&b'/')
        || bytes.contains(&0)
    {
        Err(Error::msg("structural snapshot path is invalid"))
    } else {
        Ok(())
    }
}
