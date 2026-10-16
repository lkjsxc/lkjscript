//! Closed compact notation for lexical regions and exact slab persistence.
use super::*;
use crate::platform::kernel::RegionExpression as R;

impl Decoder {
    pub(super) fn decode_region_expression(
        &mut self,
        record: &CompactRecord,
    ) -> Result<AuthoredExpressionOperation, Diagnostic> {
        let fields: &[&str] = match record.operation.as_str() {
            "expression.region-with" => &["as", "binding", "name", "type", "capacity", "body"],
            "expression.region-alloc" => &["as", "type", "capability", "value"],
            "expression.region-read" => &["as", "type", "capability", "pointer"],
            "expression.region-write" => &["as", "type", "capability", "pointer", "value"],
            "expression.region-freeze" => &["as", "type", "capability"],
            "expression.region-resolve" => &["as", "type", "capability", "offset"],
            "expression.region-offset" => &["as", "pointer"],
            "expression.region-mmap-save" => &[
                "as",
                "capability",
                "requirement",
                "operation",
                "space",
                "key",
                "expected",
            ],
            "expression.region-mmap-load" => &[
                "as",
                "type",
                "capability",
                "requirement",
                "operation",
                "space",
                "key",
                "digest",
            ],
            _ => {
                return Err(record_error(
                    record,
                    "change_region_form",
                    "unknown region expression",
                ));
            }
        };
        check_fields(record, fields)?;
        let operation = match record.operation.as_str() {
            "expression.region-with" => R::With {
                binding: Box::new(AuthoredBindingDefinition {
                    symbol: symbol_field(record, "binding")?,
                    name: parse_name(record, "name")?,
                    declared_type: Some(self.decode_type(required(record, "type")?)?),
                }),
                capacity: self.region_child(record, "capacity")?,
                body: self.region_child(record, "body")?,
            },
            "expression.region-alloc" => R::Alloc {
                pointer_type: self.decode_type(required(record, "type")?)?,
                capability: self.parse_local_reference(record, "capability")?,
                value: self.region_child(record, "value")?,
            },
            "expression.region-read" => R::Read {
                pointer_type: self.decode_type(required(record, "type")?)?,
                capability: self.parse_local_reference(record, "capability")?,
                pointer: self.region_child(record, "pointer")?,
            },
            "expression.region-write" => R::Write {
                pointer_type: self.decode_type(required(record, "type")?)?,
                capability: self.parse_local_reference(record, "capability")?,
                pointer: self.region_child(record, "pointer")?,
                value: self.region_child(record, "value")?,
            },
            "expression.region-freeze" => R::Freeze {
                frozen_type: self.decode_type(required(record, "type")?)?,
                capability: self.parse_local_reference(record, "capability")?,
            },
            "expression.region-resolve" => R::Resolve {
                pointer_type: self.decode_type(required(record, "type")?)?,
                capability: self.parse_local_reference(record, "capability")?,
                offset: self.region_child(record, "offset")?,
            },
            "expression.region-offset" => R::Offset {
                pointer: self.region_child(record, "pointer")?,
            },
            "expression.region-mmap-save" => R::MmapSave {
                capability: self.parse_local_reference(record, "capability")?,
                store: (
                    self.parse_requirement_reference(record, "requirement")?,
                    self.parse_operation_reference(record, "operation")?,
                ),
                space: self.region_child(record, "space")?,
                key: self.region_child(record, "key")?,
                expected: self.region_child(record, "expected")?,
            },
            "expression.region-mmap-load" => R::MmapLoad {
                frozen_type: self.decode_type(required(record, "type")?)?,
                capability: self.parse_local_reference(record, "capability")?,
                store: (
                    self.parse_requirement_reference(record, "requirement")?,
                    self.parse_operation_reference(record, "operation")?,
                ),
                space: self.region_child(record, "space")?,
                key: self.region_child(record, "key")?,
                digest: self.region_child(record, "digest")?,
            },
            _ => {
                return Err(record_error(
                    record,
                    "change_region_form",
                    "unknown region expression",
                ));
            }
        };
        Ok(AuthoredExpressionOperation::Region { operation })
    }

    fn region_child(
        &mut self,
        record: &CompactRecord,
        name: &str,
    ) -> Result<Box<AuthoredExpression>, Diagnostic> {
        Ok(Box::new(self.decode_expression(required(record, name)?)?))
    }
}
