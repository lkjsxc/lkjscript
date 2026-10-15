//! Bind the sole wire schema to the complete normalized production source closure.

use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit_mut::{self, VisitMut};
use syn::{Attribute, Item, Meta};

type BuildResult<T> = Result<T, Box<dyn Error>>;

const SCHEMA_ROOTS: &[&str] = &["src/lib.rs"];

const ENCODING_POLICY: &str = "lkjscript.canonical.schema;kind8;schema32;length-le64;blake3-domain-checksum32;bincode2.0.1;little-endian;variable-int;closed-enums;exact-consumption;bounded-payload";

fn main() -> BuildResult<()> {
    verify_schema_derivation()?;
    let mut source = BTreeMap::new();
    for root in SCHEMA_ROOTS {
        read_module(Path::new(root), &mut source)?;
    }
    let mut schema = blake3::Hasher::new_derive_key("lkjscript.canonical.schema-definition");
    hash_field(&mut schema, ENCODING_POLICY.as_bytes());
    let mut validator =
        blake3::Hasher::new_derive_key("lkjscript.canonical.validator-implementation");
    for policy in ["Cargo.toml", "Cargo.lock", "build.rs"] {
        println!("cargo:rerun-if-changed={policy}");
        let bytes = fs::read(policy)?;
        let bytes = if policy == "build.rs" {
            let mut file = syn::parse_file(std::str::from_utf8(&bytes)?)?;
            StripDocumentation.visit_file_mut(&mut file);
            file.to_token_stream().to_string().into_bytes()
        } else {
            bytes
        };
        hash_field(&mut schema, policy.as_bytes());
        hash_field(&mut schema, &bytes);
        hash_field(&mut validator, policy.as_bytes());
        hash_field(&mut validator, &bytes);
    }
    let mut cfg = CfgBindings::default();
    for file in source.values() {
        cfg.visit_file_mut(&mut file.clone());
    }
    for key in &cfg.keys {
        println!("cargo:rerun-if-env-changed={key}");
        hash_field(&mut schema, key.as_bytes());
        match std::env::var(key) {
            Ok(value) => {
                hash_field(&mut schema, b"present");
                hash_field(&mut schema, value.as_bytes());
            }
            Err(std::env::VarError::NotPresent) => hash_field(&mut schema, b"absent"),
            Err(error) => return Err(error.into()),
        }
    }
    for (path, file) in &source {
        let tokens = production_tokens(path, file);
        hash_field(&mut schema, path.as_bytes());
        hash_field(&mut schema, tokens.as_bytes());
        hash_field(&mut validator, path.as_bytes());
        hash_field(&mut validator, tokens.as_bytes());
    }
    let output = PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    let generated = format!(
        "pub const CANONICAL_SCHEMA_HASH: [u8; 32] = {:?};\npub const CANONICAL_SCHEMA_HASH_HEX: &str = {:?};\npub const VALIDATOR_IMPLEMENTATION_HASH: [u8; 32] = {:?};\n",
        schema.finalize().as_bytes(),
        schema.finalize().to_hex().as_str(),
        validator.finalize().as_bytes(),
    );
    fs::write(output.join("canonical_schema.rs"), generated)?;
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}

fn hash_field(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn test_only(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg")
            && matches!(&attribute.meta, Meta::List(list) if list.tokens.to_string() == "test")
    })
}

fn item_attributes(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(value) => &value.attrs,
        Item::Enum(value) => &value.attrs,
        Item::Fn(value) => &value.attrs,
        Item::Impl(value) => &value.attrs,
        Item::Macro(value) => &value.attrs,
        Item::Mod(value) => &value.attrs,
        Item::Static(value) => &value.attrs,
        Item::Struct(value) => &value.attrs,
        Item::Trait(value) => &value.attrs,
        Item::Type(value) => &value.attrs,
        Item::Union(value) => &value.attrs,
        Item::Use(value) => &value.attrs,
        _ => &[],
    }
}

fn read_module(path: &Path, files: &mut BTreeMap<String, syn::File>) -> BuildResult<()> {
    let key = path.to_string_lossy().replace('\\', "/");
    if files.contains_key(&key) {
        return Ok(());
    }
    println!("cargo:rerun-if-changed={key}");
    let mut file = syn::parse_file(&fs::read_to_string(path)?)?;
    StripDocumentation.visit_file_mut(&mut file);
    let parent = path.parent().ok_or("source module has no parent")?;
    let module_dir = if path
        .file_name()
        .is_some_and(|name| name == "mod.rs" || name == "lib.rs" || name == "main.rs")
    {
        parent.to_path_buf()
    } else {
        path.with_extension("")
    };
    read_children(&file.items, parent, &module_dir, files)?;
    files.insert(key, file);
    Ok(())
}

fn read_children(
    items: &[Item],
    parent: &Path,
    module_dir: &Path,
    files: &mut BTreeMap<String, syn::File>,
) -> BuildResult<()> {
    for item in items {
        let Item::Mod(module) = item else {
            continue;
        };
        if test_only(&module.attrs) {
            continue;
        }
        if let Some((_, children)) = &module.content {
            read_children(
                children,
                parent,
                &module_dir.join(module.ident.to_string()),
                files,
            )?;
            continue;
        }
        let explicit = module.attrs.iter().find_map(|attribute| {
            if !attribute.path().is_ident("path") {
                return None;
            }
            let Meta::NameValue(value) = &attribute.meta else {
                return None;
            };
            let syn::Expr::Lit(value) = &value.value else {
                return None;
            };
            let syn::Lit::Str(value) = &value.lit else {
                return None;
            };
            Some(parent.join(value.value()))
        });
        let child = explicit.unwrap_or_else(|| {
            let direct = module_dir.join(format!("{}.rs", module.ident));
            if direct.is_file() {
                direct
            } else {
                module_dir.join(module.ident.to_string()).join("mod.rs")
            }
        });
        read_module(&child, files)?;
    }
    Ok(())
}

fn production_tokens(path: &str, file: &syn::File) -> String {
    let mut production = file.clone();
    if path == "src/platform/builtin_standard.rs" {
        for item in &mut production.items {
            if let Item::Const(value) = item
                && (value.ident == "STANDARD_SEMANTIC_REVISION"
                    || value.ident == "STANDARD_PACKAGE_REVISION"
                    || value.ident == "STANDARD_PACKAGE_TRANSPORT")
            {
                // These are generated content identities, not format policy. Including
                // their values would make rebuilding the embedded assets self-referential.
                // Keep their names/types/attributes in the identity; redact only the data.
                *value.expr = syn::parse_quote!("content-addressed-asset");
            }
        }
    }
    production.to_token_stream().to_string()
}

fn schema_fixture(files: &[(&str, &str)]) -> BuildResult<[u8; 32]> {
    let mut hash = blake3::Hasher::new_derive_key("lkjscript.schema-derivation-control");
    for (path, source) in files {
        let mut file = syn::parse_file(source)?;
        StripDocumentation.visit_file_mut(&mut file);
        hash_field(&mut hash, path.as_bytes());
        hash_field(&mut hash, production_tokens(path, &file).as_bytes());
    }
    Ok(*hash.finalize().as_bytes())
}

fn verify_schema_derivation() -> BuildResult<()> {
    let root = "use nested::Nested; pub struct Root { pub value: Nested }";
    let nested = "pub struct Nested { pub flag: u32 }";
    let original = schema_fixture(&[("root.rs", root), ("nested.rs", nested)])?;
    let documented = schema_fixture(&[
        (
            "root.rs",
            "// comment\nuse nested::Nested; #[doc = \"documentation\"] pub struct Root { #[doc = \"field\"] pub value: Nested } #[cfg(test)] mod tests { struct Ignored; }",
        ),
        ("nested.rs", nested),
    ])?;
    if original != documented {
        return Err(
            "schema derivation must ignore comments, documentation and test-only types".into(),
        );
    }
    let changed = schema_fixture(&[
        ("root.rs", root),
        ("nested.rs", "pub struct Nested { pub flag: u64 }"),
    ])?;
    if original == changed {
        return Err("schema derivation must bind imported nested field types".into());
    }
    if schema_fixture(&[("enum.rs", "enum Value { First, Second }")])?
        == schema_fixture(&[("enum.rs", "enum Value { Second, First }")])?
    {
        return Err("schema derivation must bind enum variant order".into());
    }
    if schema_fixture(&[("codec.rs", "fn encode_value() -> u8 { 1 }")])?
        == schema_fixture(&[("codec.rs", "fn encode_value() -> u8 { 2 }")])?
    {
        return Err("schema derivation must bind manual codec implementation".into());
    }
    if schema_fixture(&[(
        "codec.rs",
        "const TAG: u8 = 1; fn encode_value() -> u8 { helper() } fn helper() -> u8 { TAG }",
    )])? == schema_fixture(&[(
        "codec.rs",
        "const TAG: u8 = 2; fn encode_value() -> u8 { helper() } fn helper() -> u8 { TAG }",
    )])? {
        return Err("schema derivation must bind codec constants and helpers".into());
    }
    if schema_fixture(&[("fields.rs", "struct Fields { first: u8, second: u32 }")])?
        == schema_fixture(&[("fields.rs", "struct Fields { second: u32, first: u8 }")])?
    {
        return Err("schema derivation must bind ordered record fields".into());
    }
    let tag_reader = "use object::Domain; fn bytes(value: Domain) -> u8 { value.tag() }";
    if schema_fixture(&[
        ("root.rs", tag_reader),
        (
            "object.rs",
            "enum Domain { Owner } impl Domain { fn tag(self) -> u8 { match self { Self::Owner => 1 } } }",
        ),
    ])? == schema_fixture(&[
        ("root.rs", tag_reader),
        (
            "object.rs",
            "enum Domain { Owner } impl Domain { fn tag(self) -> u8 { match self { Self::Owner => 2 } } }",
        ),
    ])? {
        return Err("schema derivation must bind transitive domain tag mappings".into());
    }
    if schema_fixture(&[
        (
            "root.rs",
            "fn bytes() -> &'static [u8] { &contract::PACK_MAGIC }",
        ),
        ("contract.rs", "const PACK_MAGIC: [u8; 8] = *b\"LKJPACK!\";"),
    ])? == schema_fixture(&[
        (
            "root.rs",
            "fn bytes() -> &'static [u8] { &contract::PACK_MAGIC }",
        ),
        ("contract.rs", "const PACK_MAGIC: [u8; 8] = *b\"LKJPAK!!\";"),
    ])? {
        return Err("schema derivation must bind external wire constants".into());
    }
    let generated_identity = "const STANDARD_PACKAGE_REVISION: &str = \"first\";";
    let regenerated_identity = "const STANDARD_PACKAGE_REVISION: &str = \"second\";";
    if schema_fixture(&[("src/platform/builtin_standard.rs", generated_identity)])?
        != schema_fixture(&[("src/platform/builtin_standard.rs", regenerated_identity)])?
    {
        return Err("generated asset identities must not create a fingerprint cycle".into());
    }
    Ok(())
}

struct StripDocumentation;

impl VisitMut for StripDocumentation {
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        strip_attributes(&mut file.attrs);
        file.items.retain(|item| !test_only(item_attributes(item)));
        visit_mut::visit_file_mut(self, file);
    }

    fn visit_item_mut(&mut self, item: &mut Item) {
        match item {
            Item::Const(value) => strip_attributes(&mut value.attrs),
            Item::Enum(value) => strip_attributes(&mut value.attrs),
            Item::Fn(value) => strip_attributes(&mut value.attrs),
            Item::Impl(value) => strip_attributes(&mut value.attrs),
            Item::Macro(value) => strip_attributes(&mut value.attrs),
            Item::Mod(value) => strip_attributes(&mut value.attrs),
            Item::Static(value) => strip_attributes(&mut value.attrs),
            Item::Struct(value) => strip_attributes(&mut value.attrs),
            Item::Trait(value) => strip_attributes(&mut value.attrs),
            Item::Type(value) => strip_attributes(&mut value.attrs),
            Item::Union(value) => strip_attributes(&mut value.attrs),
            Item::Use(value) => strip_attributes(&mut value.attrs),
            _ => {}
        }
        visit_mut::visit_item_mut(self, item);
    }

    fn visit_item_mod_mut(&mut self, module: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut module.content {
            items.retain(|item| !test_only(item_attributes(item)));
        }
        visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_field_mut(&mut self, field: &mut syn::Field) {
        strip_attributes(&mut field.attrs);
        visit_mut::visit_field_mut(self, field);
    }

    fn visit_variant_mut(&mut self, variant: &mut syn::Variant) {
        strip_attributes(&mut variant.attrs);
        visit_mut::visit_variant_mut(self, variant);
    }

    fn visit_impl_item_mut(&mut self, item: &mut syn::ImplItem) {
        match item {
            syn::ImplItem::Const(value) => strip_attributes(&mut value.attrs),
            syn::ImplItem::Fn(value) => strip_attributes(&mut value.attrs),
            syn::ImplItem::Type(value) => strip_attributes(&mut value.attrs),
            syn::ImplItem::Macro(value) => strip_attributes(&mut value.attrs),
            _ => {}
        }
        visit_mut::visit_impl_item_mut(self, item);
    }
}

fn strip_attributes(attributes: &mut Vec<Attribute>) {
    attributes.retain(|attribute| !attribute.path().is_ident("doc"));
}

#[derive(Default)]
struct CfgBindings {
    keys: BTreeSet<String>,
}

impl CfgBindings {
    fn record(&mut self, meta: &Meta) {
        match meta {
            Meta::Path(path) => {
                if let Some(name) = path.get_ident()
                    && name != "test"
                {
                    self.keys
                        .insert(format!("CARGO_CFG_{}", name.to_string().to_uppercase()));
                }
            }
            Meta::NameValue(value) => {
                if value.path.is_ident("feature") {
                    if let syn::Expr::Lit(value) = &value.value
                        && let syn::Lit::Str(value) = &value.lit
                    {
                        self.keys.insert(format!(
                            "CARGO_FEATURE_{}",
                            value.value().replace('-', "_").to_uppercase()
                        ));
                    }
                } else if let Some(name) = value.path.get_ident() {
                    self.keys
                        .insert(format!("CARGO_CFG_{}", name.to_string().to_uppercase()));
                }
            }
            Meta::List(value) => {
                use syn::parse::Parser;
                let parser = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated;
                if let Ok(values) = parser.parse2(value.tokens.clone()) {
                    for value in values {
                        self.record(&value);
                    }
                }
            }
        }
    }
}

impl VisitMut for CfgBindings {
    fn visit_attribute_mut(&mut self, attribute: &mut Attribute) {
        if attribute.path().is_ident("cfg")
            && let Meta::List(value) = &attribute.meta
            && let Ok(meta) = syn::parse2(value.tokens.clone())
        {
            self.record(&meta);
        }
        visit_mut::visit_attribute_mut(self, attribute);
    }
}
