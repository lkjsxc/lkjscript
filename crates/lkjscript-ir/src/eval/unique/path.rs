use super::{map_store_error, EvalUniqueRuntime};
use crate::eval::{EvalValue, Flow};
use lkjscript_core::UniqueLayout;

impl EvalUniqueRuntime {
    pub(crate) fn allocate_path(&mut self, bytes: Vec<u8>) -> Result<EvalValue, Flow> {
        let key = self
            .store
            .allocate_path(bytes.into_boxed_slice())
            .map_err(map_store_error)?;
        self.publish_path(key.packed_word())
    }

    pub(crate) fn copy_path(&mut self, value: &EvalValue) -> Result<Vec<u8>, Flow> {
        let EvalValue::Path(word) = value else {
            return Err(Flow::Trap("expected opaque-path owner".into()));
        };
        self.require_owner(*word, UniqueLayout::Path)?;
        let key = self.store.import_path_key(*word).map_err(map_store_error)?;
        Ok(self.store.path(key).map_err(map_store_error)?.to_vec())
    }

    pub(crate) fn clone_path(&mut self, value: &EvalValue) -> Result<EvalValue, Flow> {
        let EvalValue::Path(word) = value else {
            return Err(Flow::Trap("expected opaque-path owner".into()));
        };
        self.require_owner(*word, UniqueLayout::Path)?;
        let key = self.store.import_path_key(*word).map_err(map_store_error)?;
        let clone = self.store.clone_path(key).map_err(map_store_error)?;
        self.publish_path(clone.packed_word())
    }

    fn publish_path(&mut self, word: lkjscript_core::UniqueKeyWord) -> Result<EvalValue, Flow> {
        if self.owners.insert(word.get(), UniqueLayout::Path).is_some() {
            return Err(Flow::Trap("duplicate evaluator opaque-path owner".into()));
        }
        Ok(EvalValue::Path(word))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::EvalConfig;

    #[test]
    fn opaque_path_copy_clone_transfer_and_drop_are_generation_safe() -> Result<(), String> {
        let Some(mut runtime) = EvalUniqueRuntime::new(&EvalConfig::default()) else {
            return Err("construct evaluator unique runtime".into());
        };
        let owner = runtime
            .allocate_path(b"/tmp/evaluator-path".to_vec())
            .map_err(|error| format!("allocate path: {error:?}"))?;
        let clone = runtime
            .clone_path(&owner)
            .map_err(|error| format!("clone path: {error:?}"))?;
        assert_eq!(
            runtime
                .copy_path(&clone)
                .map_err(|error| format!("copy cloned path: {error:?}"))?,
            b"/tmp/evaluator-path"
        );
        runtime
            .drop_owner(owner)
            .map_err(|error| format!("drop original path: {error:?}"))?;
        let bytes = runtime
            .export_owner(clone)
            .map_err(|error| format!("transfer cloned path: {error:?}"))?;
        assert_eq!(bytes, b"/tmp/evaluator-path");
        runtime
            .verify_empty()
            .map_err(|error| format!("verify empty path runtime: {error:?}"))
    }
}
