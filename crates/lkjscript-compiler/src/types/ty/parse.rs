use super::*;

pub fn parse_one(atoms: &[String], i: usize) -> Result<(Type, usize), String> {
    let Some(a) = atoms.get(i) else {
        return Err("expected type".into());
    };
    match a.as_str() {
        "Any" => Err("Any is not a permitted type".into()),
        "Never" => Ok((Type::Never, i + 1)),
        "Unit" => Ok((Type::Unit, i + 1)),
        "Nil" => Err("Nil was removed; use Unit, Option T, or List T".into()),
        "Bool" => Ok((Type::Bool, i + 1)),
        "I64" => Ok((Type::I64, i + 1)),
        "F64" => Ok((Type::F64, i + 1)),
        "I32" | "U32" | "U64" | "F32" | "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "Int"
        | "Float" => Err(format!(
            "unsupported numeric type {a}; use canonical I64 or F64"
        )),
        "Str" => Ok((Type::Str, i + 1)),
        "Buf" => Ok((Type::Buf, i + 1)),
        "Owned" | "Ref" | "RefMut" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            if inner != Type::Buf {
                return Err(format!(
                    "{a} accepts only exact Buf in the initial ownership slice"
                ));
            }
            let ty = match a.as_str() {
                "Owned" => Type::Owned(Box::new(inner)),
                "Ref" => Type::Ref(Box::new(inner)),
                "RefMut" => Type::RefMut(Box::new(inner)),
                _ => return Err("invalid ownership type".into()),
            };
            Ok((ty, next))
        }
        "Symbol" => Ok((Type::Symbol, i + 1)),
        "Handle" => Ok((Type::Handle, i + 1)),
        "Product" => {
            let Some(name) = atoms.get(i + 1) else {
                return Err("Product requires a declared product name".into());
            };
            if !is_product_type_name(name) {
                return Err(format!("invalid product type name {name}"));
            }
            Ok((Type::Product(name.clone()), i + 2))
        }
        "List" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            Ok((Type::List(Box::new(inner)), next))
        }
        "Option" => {
            let (inner, next) = parse_one(atoms, i + 1)?;
            Ok((Type::Option(Box::new(inner)), next))
        }
        "Result" => {
            let (ok, n1) = parse_one(atoms, i + 1)?;
            let (err, n2) = parse_one(atoms, n1)?;
            Ok((Type::Result(Box::new(ok), Box::new(err)), n2))
        }
        other if is_numeric_width_name(other) => Err(format!(
            "unsupported numeric type {other}; use canonical I64 or F64"
        )),
        // Type parameter: single uppercase letter or T, U, E, …
        other if is_type_param_name(other) => Ok((Type::Param(other.to_string()), i + 1)),
        other => Err(format!("unknown type {other}")),
    }
}

fn is_numeric_width_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'I' | b'U' | b'F' | b'i' | b'u' | b'f'))
        && bytes.clone().next().is_some()
        && bytes.all(|byte| byte.is_ascii_digit())
}

fn is_type_param_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && s.chars().all(|c| c.is_ascii_alphanumeric())
        && !matches!(
            s,
            "Never"
                | "Unit"
                | "Bool"
                | "I64"
                | "F64"
                | "Str"
                | "Buf"
                | "Owned"
                | "Ref"
                | "RefMut"
                | "Symbol"
                | "Handle"
                | "List"
                | "Option"
                | "Result"
                | "Product"
                | "Int"
                | "Float"
                | "Any"
        )
}

fn is_product_type_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}
