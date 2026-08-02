use super::*;

impl JitValueServices<'_> {
    pub(crate) fn reference_value(
        &self,
        reference: lkjscript_native::NativeReference,
    ) -> Result<Value, String> {
        let reference_type = reference.reference_type();
        if let ReferenceType::RegionProduct(_, identity) = reference_type {
            let key = lkjscript_core::RegionProductKey::from_word(
                self.region_products.id(),
                reference.opaque_word(),
            )
            .ok_or_else(|| "invalid region-product reference".to_string())?;
            self.region_products
                .validate_identity(key, lkjscript_core::RuntimeLayoutId::new(identity))
                .map_err(|error| format!("region-product identity mismatch: {error:?}"))?;
            return Ok(Value::from_region_product(key));
        }
        if matches!(reference_type, ReferenceType::List(_, _, _, _)) {
            let key = self
                .lists
                .key_from_word(reference.opaque_word())
                .map_err(|error| format!("invalid segmented-list reference: {error:?}"))?;
            self.lists
                .validate_type(key, reference_layout_key(reference_type))
                .map_err(|error| format!("segmented-list layout mismatch: {error:?}"))?;
            return Ok(if key.is_empty() {
                Value::EMPTY_LIST
            } else {
                Value::from_segmented_list(key.to_word())
            });
        }
        Err("removed reference category reached deterministic runtime".into())
    }

    pub(crate) fn native_reference(
        &self,
        value: Value,
        reference_type: ReferenceType,
    ) -> Result<NativeValue, String> {
        if let ReferenceType::RegionProduct(_, identity) = reference_type {
            let word = value
                .as_region_product_word()
                .ok_or_else(|| "expected region-product result".to_string())?;
            let key = lkjscript_core::RegionProductKey::from_word(self.region_products.id(), word)
                .ok_or_else(|| "invalid region-product result".to_string())?;
            self.region_products
                .validate_identity(key, lkjscript_core::RuntimeLayoutId::new(identity))
                .map_err(|error| format!("region-product result identity mismatch: {error:?}"))?;
            return Ok(NativeValue::Reference(
                lkjscript_native::NativeReference::new(reference_type, word),
            ));
        }
        if matches!(reference_type, ReferenceType::List(_, _, _, _)) {
            let word = if value.is_empty_list() {
                0
            } else {
                value
                    .as_segmented_list()
                    .ok_or_else(|| "expected segmented-list result".to_string())?
            };
            let key = self
                .lists
                .key_from_word(word)
                .map_err(|error| format!("invalid segmented-list result: {error:?}"))?;
            self.lists
                .validate_type(key, reference_layout_key(reference_type))
                .map_err(|error| format!("segmented-list result layout mismatch: {error:?}"))?;
            return Ok(NativeValue::Reference(
                lkjscript_native::NativeReference::new(reference_type, word),
            ));
        }
        Err("removed reference result category reached deterministic runtime".into())
    }
}
