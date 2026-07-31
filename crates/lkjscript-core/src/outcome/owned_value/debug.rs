impl fmt::Debug for OwnedValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(bytes) = self.as_byte_vector() {
            return write!(formatter, "#<owned-byte-vector:{}>", bytes.len());
        }
        if let Some(bytes) = self.as_bytes() {
            return write!(formatter, "#<owned-bytes:{}>", bytes.len());
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
        if let Some(value) = self.as_str() {
            return value.fmt(formatter);
        }
        if let Some(value) = self.as_path_bytes() {
            return write!(formatter, "#<owned-path:{}>", value.len());
        }
        if let Some(value) = self.as_resource() {
            return write!(formatter, "resource#{value}");
        }
        if let Some(prototype) = self.as_function() {
            return write!(formatter, "#<owned-fn:{prototype}>");
        }
        if let Some(index) = self.root.as_owned_list() {
            return write!(formatter, "#<owned-list:{}>", index.saturating_add(1));
        }
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Product(fields) => {
                    write!(formatter, "#<owned-structural-product:{}>", fields.len())
                }
                SemanticPayload::Enum {
                    tag,
                    active_payload,
                } => write!(
                    formatter,
                    "#<owned-structural-enum:{tag}:{}>",
                    active_payload.len()
                ),
                SemanticPayload::Static(_) => formatter.write_str("#<owned-static>"),
                _ => formatter.write_str("#<owned-structural-value>"),
            };
        }
        self.root.fmt(formatter)
    }
}
