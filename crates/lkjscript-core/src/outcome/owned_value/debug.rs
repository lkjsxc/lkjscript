impl fmt::Debug for OwnedValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(bytes) = self.as_byte_vector() {
            return write!(formatter, "#<owned-byte-vector:{}>", bytes.len());
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
        if let Some(value) = self.as_resource() {
            return write!(formatter, "resource#{value}");
        }
        if let Some(prototype) = self.as_function() {
            return write!(formatter, "#<owned-fn:{prototype}>");
        }
        match self.object() {
            Some(HeapObj::Pair { .. }) => formatter.write_str("#<owned-pair>"),
            Some(HeapObj::Buf(bytes)) => write!(formatter, "#<owned-buf:{}>", bytes.len()),
            Some(HeapObj::Path(bytes)) => write!(formatter, "#<owned-path:{}>", bytes.len()),
            Some(HeapObj::Product { product, .. }) => {
                write!(formatter, "#<owned-product:{}>", product.raw())
            }
            Some(HeapObj::Enum { physical_tag, .. }) => {
                write!(formatter, "#<owned-enum:{physical_tag}>")
            }
            Some(HeapObj::Str(_) | HeapObj::Symbol(_)) => formatter.write_str("#<owned-value>"),
            None => self.root.fmt(formatter),
        }
    }
}
