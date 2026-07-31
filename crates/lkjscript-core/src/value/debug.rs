use std::fmt;

use super::Value;

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_invalid() {
            return formatter.write_str("#<invalid>");
        }
        if self.is_unit() {
            return formatter.write_str("unit");
        }
        if self.is_empty_list() {
            return formatter.write_str("empty-list");
        }
        if let Some(value) = self.as_bool() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_i64() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_f64() {
            return value.fmt(formatter);
        }
        if let Some(index) = self.as_resource() {
            return write!(formatter, "resource#{index}");
        }
        if let Some(prototype) = self.as_function() {
            return write!(formatter, "function#{prototype}");
        }
        if let Some(constant) = self.as_symbol() {
            return write!(formatter, "symbol#{constant}");
        }
        if let Some(kind) = self.as_capability() {
            return write!(formatter, "capability#{}", kind.as_str());
        }
        if let Some(word) = self.as_segmented_list() {
            return write!(formatter, "segmented-list#{word}");
        }
        if let Some(index) = self.as_owned_list() {
            return write!(formatter, "owned-list#{index}");
        }
        if let Some(word) = self.as_region_product_word() {
            return write!(formatter, "region-product#{word}");
        }
        if let Some(key) = self.as_aggregate_adapter() {
            return write!(formatter, "aggregate-adapter#{key}");
        }
        if let Some(key) = self.as_structural_root() {
            return write!(formatter, "structural-root#{}", key.get());
        }
        if let Some(key) = self.as_structural_view() {
            return write!(formatter, "structural-view#{key}");
        }
        if let Some(key) = self.as_structural_destination() {
            return write!(formatter, "structural-destination#{key}");
        }
        if let Some(index) = self.as_static_string() {
            return write!(formatter, "static-string#{index}");
        }
        if let Some(index) = self.as_static_bytes() {
            return write!(formatter, "static-bytes#{index}");
        }
        if let Some(key) = self.as_bytes_key() {
            return write!(formatter, "bytes-key#{key}");
        }
        if let Some(key) = self.as_byte_vector_key() {
            return write!(formatter, "byte-vector-key#{key}");
        }
        if let Some(token) = self.as_bytes_borrow() {
            return write!(formatter, "bytes-borrow#{token}");
        }
        if let Some((token, mutable)) = self.as_byte_slice() {
            return write!(
                formatter,
                "byte-slice{}#{token}",
                if mutable { "-mut" } else { "" }
            );
        }
        formatter.write_str("#<invalid-value-category>")
    }
}
