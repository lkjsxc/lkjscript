use lkjscript::application::{
    self, ApplicationDigest, ApplicationFieldValue, ApplicationTarget, ApplicationValue,
};
use lkjscript::release::{ReleaseId, ReleaseItemId};
use lkjscript::schema::{ByteString, MAXIMUM_BYTE_STRING_BYTES, MAXIMUM_TEXT_BYTES, TextString};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const APPLICATION_BYTES: &[u8] = include_bytes!("../../../applications/lkjwork/lkjwork.lkja");
const BINDINGS_BYTES: &[u8] = include_bytes!("../../../applications/lkjwork/bindings.json");

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingFile {
    contract_version: u16,
    application_digest: ApplicationDigest,
    release: ReleaseId,
    types: BTreeMap<String, TypeBinding>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypeBinding {
    target: u64,
    #[serde(default)]
    fields: BTreeMap<String, u64>,
    #[serde(default)]
    variants: BTreeMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct Bindings {
    file: BindingFile,
}

impl Bindings {
    pub fn load() -> Result<Self, String> {
        let file: BindingFile =
            lkjscript::instance::strict_json(BINDINGS_BYTES, "lkjwork bindings")
                .map_err(|error| error.to_string())?;
        if file.contract_version != 1 {
            return Err(
                "embedded lkjwork bindings have an unsupported contract version".to_owned(),
            );
        }
        let inspection =
            application::inspect(APPLICATION_BYTES).map_err(|error| error.to_string())?;
        if inspection.digest != file.application_digest || inspection.root_release != file.release {
            return Err("embedded lkjwork application and generated bindings disagree".to_owned());
        }
        Ok(Self { file })
    }

    pub const fn application_bytes(&self) -> &'static [u8] {
        APPLICATION_BYTES
    }

    pub fn application_digest(&self) -> ApplicationDigest {
        self.file.application_digest
    }

    pub fn release(&self) -> ReleaseId {
        self.file.release
    }

    pub fn target(&self, name: &str) -> Result<ApplicationTarget, String> {
        let item = self
            .file
            .types
            .get(name)
            .ok_or_else(|| format!("lkjwork binding type {name} is missing"))?
            .target;
        self.application_target(item)
    }

    pub fn field(&self, owner: &str, name: &str) -> Result<ApplicationTarget, String> {
        let item = self
            .file
            .types
            .get(owner)
            .and_then(|binding| binding.fields.get(name))
            .copied()
            .ok_or_else(|| format!("lkjwork binding field {owner}.{name} is missing"))?;
        self.application_target(item)
    }

    pub fn variant(&self, owner: &str, name: &str) -> Result<ApplicationTarget, String> {
        let item = self
            .file
            .types
            .get(owner)
            .and_then(|binding| binding.variants.get(name))
            .copied()
            .ok_or_else(|| format!("lkjwork binding variant {owner}.{name} is missing"))?;
        self.application_target(item)
    }

    pub fn variant_name(&self, owner: &str, target: ApplicationTarget) -> Option<&str> {
        if target.release != self.file.release {
            return None;
        }
        self.file
            .types
            .get(owner)?
            .variants
            .iter()
            .find_map(|(name, item)| (target.item.get() == *item).then_some(name.as_str()))
    }

    pub fn field_name(&self, owner: &str, target: ApplicationTarget) -> Option<&str> {
        if target.release != self.file.release {
            return None;
        }
        self.file
            .types
            .get(owner)?
            .fields
            .iter()
            .find_map(|(name, item)| (target.item.get() == *item).then_some(name.as_str()))
    }

    pub fn text(&self, value: &str) -> Result<ApplicationValue, String> {
        if value.len() > MAXIMUM_TEXT_BYTES {
            return Err(format!(
                "product input requests {} UTF-8 text bytes; limit is {MAXIMUM_TEXT_BYTES}",
                value.len()
            ));
        }
        TextString::try_from_str(value)
            .map(ApplicationValue::Text)
            .map_err(|error| error.to_string())
    }

    pub fn bytes(&self, value: &[u8]) -> Result<ApplicationValue, String> {
        if value.len() > MAXIMUM_BYTE_STRING_BYTES {
            return Err(format!(
                "product input requests {} byte-value bytes; limit is {MAXIMUM_BYTE_STRING_BYTES}",
                value.len()
            ));
        }
        ByteString::from_slice(value)
            .map(ApplicationValue::Bytes)
            .map_err(|error| error.to_string())
    }

    pub const fn integer(&self, value: i64) -> ApplicationValue {
        ApplicationValue::I64(value)
    }

    pub const fn boolean(&self, value: bool) -> ApplicationValue {
        ApplicationValue::Bool(value)
    }

    pub fn product(
        &self,
        owner: &str,
        fields: Vec<(&str, ApplicationValue)>,
    ) -> Result<ApplicationValue, String> {
        Ok(ApplicationValue::Product {
            ty: self.target(owner)?,
            fields: fields
                .into_iter()
                .map(|(name, value)| {
                    Ok(ApplicationFieldValue {
                        field: self.field(owner, name)?,
                        value,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        })
    }

    pub fn sum(
        &self,
        owner: &str,
        variant: &str,
        payload: Option<ApplicationValue>,
    ) -> Result<ApplicationValue, String> {
        Ok(ApplicationValue::Sum {
            ty: self.target(owner)?,
            variant: self.variant(owner, variant)?,
            payload: payload.map(Box::new),
        })
    }

    pub fn sequence(
        &self,
        owner: &str,
        elements: Vec<ApplicationValue>,
    ) -> Result<ApplicationValue, String> {
        Ok(ApplicationValue::Sequence {
            ty: self.target(owner)?,
            elements,
        })
    }

    pub fn expect_product<'a>(
        &self,
        owner: &str,
        value: &'a ApplicationValue,
    ) -> Result<BTreeMap<String, &'a ApplicationValue>, String> {
        let ApplicationValue::Product { ty, fields } = value else {
            return Err(format!(
                "lkjwork application returned a non-{owner} product"
            ));
        };
        if *ty != self.target(owner)? {
            return Err(format!(
                "lkjwork application returned the wrong {owner} identity"
            ));
        }
        let mut result = BTreeMap::new();
        for field in fields {
            let name = self
                .field_name(owner, field.field)
                .ok_or_else(|| format!("lkjwork application returned a foreign {owner} field"))?;
            if result.insert(name.to_owned(), &field.value).is_some() {
                return Err(format!(
                    "lkjwork application returned duplicate {owner}.{name}"
                ));
            }
        }
        if result.len()
            != self
                .file
                .types
                .get(owner)
                .map_or(0, |binding| binding.fields.len())
        {
            return Err(format!("lkjwork application omitted a {owner} field"));
        }
        Ok(result)
    }

    pub fn expect_sum<'a>(
        &self,
        owner: &str,
        value: &'a ApplicationValue,
    ) -> Result<(String, Option<&'a ApplicationValue>), String> {
        let ApplicationValue::Sum {
            ty,
            variant,
            payload,
        } = value
        else {
            return Err(format!("lkjwork application returned a non-{owner} sum"));
        };
        if *ty != self.target(owner)? {
            return Err(format!(
                "lkjwork application returned the wrong {owner} identity"
            ));
        }
        let name = self
            .variant_name(owner, *variant)
            .ok_or_else(|| format!("lkjwork application returned a foreign {owner} variant"))?;
        Ok((name.to_owned(), payload.as_deref()))
    }

    pub fn expect_sequence<'a>(
        &self,
        owner: &str,
        value: &'a ApplicationValue,
    ) -> Result<&'a [ApplicationValue], String> {
        let ApplicationValue::Sequence { ty, elements } = value else {
            return Err(format!(
                "lkjwork application returned a non-{owner} sequence"
            ));
        };
        if *ty != self.target(owner)? {
            return Err(format!(
                "lkjwork application returned the wrong {owner} identity"
            ));
        }
        Ok(elements)
    }

    fn application_target(&self, item: u64) -> Result<ApplicationTarget, String> {
        Ok(ApplicationTarget {
            release: self.file.release,
            item: ReleaseItemId::new(item).map_err(|error| error.to_string())?,
        })
    }
}

pub fn expect_text(value: &ApplicationValue) -> Result<&str, String> {
    let ApplicationValue::Text(value) = value else {
        return Err("lkjwork application returned a non-text value".to_owned());
    };
    Ok(value.as_str())
}

pub fn expect_i64(value: &ApplicationValue) -> Result<i64, String> {
    let ApplicationValue::I64(value) = value else {
        return Err("lkjwork application returned a non-i64 value".to_owned());
    };
    Ok(*value)
}

pub fn expect_bool(value: &ApplicationValue) -> Result<bool, String> {
    let ApplicationValue::Bool(value) = value else {
        return Err("lkjwork application returned a non-bool value".to_owned());
    };
    Ok(*value)
}
