//! Fixed, independent bytes for the HTTP witness's {batch, edit} schema.
//! Shares canonical type identities only; no production/reference layout or value codec.
use crate::error::DevError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Identities {
    pub batch_package: String,
    pub batch: String,
    pub revision: String,
    pub items: String,
    pub edit_package: String,
    pub edit: String,
    pub keep: String,
    pub replace: String,
}

#[derive(Clone)]
struct TypeIdentity {
    bytes: [u8; 32],
    text: String,
}
fn ty(form: serde_json::Value) -> Result<TypeIdentity, DevError> {
    let (bytes, text) = lkjscript::platform::contributor::canonical_type_object_identity(
        &serde_json::to_vec(&form)?,
    )
    .map_err(|error| DevError::corrupt(error.to_string()))?;
    Ok(TypeIdentity { bytes, text })
}
fn hash(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new_derive_key(domain);
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
    *hash.finalize().as_bytes()
}
fn blob(out: &mut Vec<u8>, text: &str) -> Result<(), DevError> {
    if text.len() > 128 {
        return Err(DevError::corrupt(
            "nominal fixture identity exceeds its bound",
        ));
    }
    out.extend_from_slice(&(text.len() as u32).to_be_bytes());
    out.extend_from_slice(text.as_bytes());
    Ok(())
}
fn scalar(ty: &TypeIdentity, tag: u8) -> Vec<u8> {
    let mut out = ty.bytes.to_vec();
    out.push(tag);
    out
}

pub(super) fn expected(ids: &Identities, items: &[i64]) -> Result<Vec<u8>, DevError> {
    if items.len() > 8192 {
        return Err(DevError::corrupt(
            "fixed nominal HTTP oracle exceeds 8192 items",
        ));
    }
    let integer = ty(json!({"kind":"i64"}))?;
    let list = ty(json!({"kind":"list","item":integer.text}))?;
    let batch = ty(
        json!({"kind":"applied","declaration":{"package":ids.batch_package,"declaration":ids.batch},"arguments":[integer.text]}),
    )?;
    let edit = ty(
        json!({"kind":"applied","declaration":{"package":ids.edit_package,"declaration":ids.edit},"arguments":[integer.text]}),
    )?;
    let root = ty(
        json!({"kind":"structural_record","fields":[{"name":"batch","ty":batch.text},{"name":"edit","ty":edit.text}]}),
    )?;
    let prefix =
        |ty: &TypeIdentity, package: &str, declaration: &str| -> Result<Vec<u8>, DevError> {
            let mut out = scalar(ty, 9);
            out.extend_from_slice(&1u32.to_be_bytes());
            out.extend_from_slice(&scalar(&integer, 2));
            blob(&mut out, package)?;
            blob(&mut out, declaration)?;
            out.push(1);
            Ok(out)
        };
    let mut batch_description = prefix(&batch, &ids.batch_package, &ids.batch)?;
    batch_description.push(0);
    batch_description.extend_from_slice(&2u32.to_be_bytes());
    let mut data = Vec::new();
    let fields = BTreeMap::from([
        (ids.revision.as_str(), ("revision", &integer)),
        (ids.items.as_str(), ("items", &list)),
    ]);
    if fields.len() != 2 {
        return Err(DevError::corrupt("nominal fixture aliases its fields"));
    }
    for (id, (name, kind)) in fields {
        blob(&mut batch_description, &ids.batch_package)?;
        blob(&mut batch_description, id)?;
        blob(&mut batch_description, name)?;
        if name == "revision" {
            batch_description.extend_from_slice(&scalar(kind, 2));
            data.extend_from_slice(&7i64.to_be_bytes());
        } else {
            batch_description.extend_from_slice(&scalar(kind, 7));
            batch_description.extend_from_slice(&scalar(&integer, 2));
            data.extend_from_slice(&(items.len() as u32).to_be_bytes());
            for item in items {
                data.extend_from_slice(&item.to_be_bytes());
            }
        }
    }
    let mut edit_description = prefix(&edit, &ids.edit_package, &ids.edit)?;
    edit_description.push(1);
    edit_description.extend_from_slice(&2u32.to_be_bytes());
    let cases = BTreeMap::from([
        (ids.keep.as_str(), "keep"),
        (ids.replace.as_str(), "replace"),
    ]);
    if cases.len() != 2 {
        return Err(DevError::corrupt("nominal fixture aliases its cases"));
    }
    let mut replace_index = 0u32;
    for (index, (id, name)) in cases.into_iter().enumerate() {
        blob(&mut edit_description, &ids.edit_package)?;
        blob(&mut edit_description, id)?;
        blob(&mut edit_description, name)?;
        edit_description.push(u8::from(name == "replace"));
        if name == "replace" {
            edit_description.extend_from_slice(&batch_description);
            replace_index = index as u32;
        }
    }
    let mut description = scalar(&root, 6);
    description.extend_from_slice(&2u32.to_be_bytes());
    blob(&mut description, "batch")?;
    description.extend_from_slice(&batch_description);
    blob(&mut description, "edit")?;
    description.extend_from_slice(&edit_description);
    let mut out = b"LKJDVAL1".to_vec();
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&hash("lkjscript.data.typed-layout.v1", &description));
    out.extend_from_slice(&data);
    out.extend_from_slice(&replace_index.to_be_bytes());
    out.extend_from_slice(&data);
    out.extend_from_slice(&hash("lkjscript.data.typed-value-envelope.v1", &out));
    Ok(out)
}
