use super::*;

pub fn parse_one(atoms: &[String], i: usize) -> Result<(Type, usize), String> {
    let Some(atom) = atoms.get(i) else {
        return Err("expected type".into());
    };
    match atom.as_str() {
        "any" => Err("any is not a permitted type".into()),
        "never" => Ok((Type::Never, i + 1)),
        "unit" => Ok((Type::Unit, i + 1)),
        "nil" => Err("nil was removed; use unit, option t, or list t".into()),
        "bool" => Ok((Type::Bool, i + 1)),
        "i64" => Ok((Type::I64, i + 1)),
        "f64" => Ok((Type::F64, i + 1)),
        "numeric-error" => Ok((crate::types::numeric_error_type(), i + 1)),
        "utf8-error" => Ok((crate::types::utf8_error_type(), i + 1)),
        "system-error" => Ok((crate::types::system_error_type(), i + 1)),
        "i32" | "u32" | "u64" | "f32" | "int" | "float" => Err(format!(
            "unsupported numeric type {atom}; use canonical i64 or f64"
        )),
        "string" => Ok((Type::Str, i + 1)),
        "buf" => Ok((Type::Buf, i + 1)),
        "bytes" => Err("PLACEHOLDER: immutable bytes is reserved but not Current".into()),
        "byte-vector" => Ok((Type::Owned(Box::new(Type::Buf)), i + 1)),
        "byte-slice" => Ok((Type::Ref(Box::new(Type::Buf)), i + 1)),
        "byte-slice-mut" => Ok((Type::RefMut(Box::new(Type::Buf)), i + 1)),
        "path" => Ok((Type::Path, i + 1)),
        "capability" => {
            let kind = atoms
                .get(i + 1)
                .and_then(|name| CapabilityKind::parse(name))
                .ok_or_else(|| "capability requires one closed capability kind".to_string())?;
            Ok((Type::Capability(kind), i + 2))
        }
        "owned" => Err("owned is removed; use byte-vector".into()),
        "ref" => Err("ref is removed; use byte-slice".into()),
        "ref-mut" => Err("ref-mut is removed; use byte-slice-mut".into()),
        "symbol" => Ok((Type::Symbol, i + 1)),
        "handle" => Err("handle is removed; use an exact typed resource kind".into()),
        resource if ResourceKind::parse(resource).is_some() => {
            let Some(kind) = ResourceKind::parse(resource) else {
                unreachable!("resource parse guard")
            };
            Ok((Type::Resource(kind), i + 1))
        }
        "product" => {
            let Some(name) = atoms.get(i + 1) else {
                return Err("product requires a declared product name".into());
            };
            if !is_product_type_name(name) {
                return Err(format!("invalid product type name {name}"));
            }
            Ok((Type::Product(name.clone()), i + 2))
        }
        "list" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            Ok((Type::List(Box::new(inner)), next))
        }
        "option" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            Ok((crate::types::option_type(inner), next))
        }
        "result" => {
            let (ok, next) = parse_one(atoms, i + 1)?;
            let (error, end) = parse_one(atoms, next)?;
            Ok((crate::types::result_type(ok, error), end))
        }
        other if is_numeric_width_name(other) => Err(format!(
            "unsupported numeric type {other}; use canonical i64 or f64"
        )),
        other if is_type_param_name(other) => Ok((Type::Param(other.to_string()), i + 1)),
        other => Err(format!("unknown type {other}")),
    }
}

fn is_numeric_width_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'i' | b'u' | b'f'))
        && bytes.clone().next().is_some()
        && bytes.all(|byte| byte.is_ascii_digit())
}

fn is_type_param_name(name: &str) -> bool {
    lkjscript_contracts::is_identifier(name)
        && !lkjscript_contracts::RESERVED_WORDS.contains(&name)
        && !lkjscript_contracts::BUILTIN_ERROR_NAMES.contains(&name)
}

fn is_product_type_name(name: &str) -> bool {
    lkjscript_contracts::is_identifier(name) || crate::source::module_names::is_internal_name(name)
}
