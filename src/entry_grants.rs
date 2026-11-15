//! Invocation grants are explicit CLI data, validated before any program starts.
use crate::ir::{EntryGrant, Program, TypeKind};
use std::collections::BTreeMap;

fn problem(message: &str) -> String { format!("E_ENTRY {message}") }
fn valid_name(name: &str) -> bool {
    if name.is_empty() { return false; }
    for byte in name.as_bytes() {
        if *byte <= 32 || *byte == 127 || *byte == b'=' { return false; }
    }
    true
}
pub fn validate(program: &Program, entry: usize) -> Result<(), String> {
    let function = program.functions.get(entry).ok_or_else(|| problem("unknown entry function"))?;
    let result = program.types.get(function.result).ok_or_else(|| problem("entry result type is missing"))?;
    if !matches!(result.kind, TypeKind::Unit | TypeKind::Bool | TypeKind::Char | TypeKind::Int { .. } | TypeKind::F64) {
        return Err(problem("host entry result must be unit or a scalar"));
    }
    if function.params > function.locals.len() { return Err(problem("entry parameters are missing")); }
    let mut names: Vec<String> = Vec::new();
    for parameter in 0..function.params {
        let local = &function.locals[parameter];
        let ty = program.types.get(local.ty).ok_or_else(|| problem("entry parameter type is missing"))?;
        if matches!(ty.kind, TypeKind::World) && function.params == 1 { continue; }
        if matches!(ty.kind, TypeKind::File) {
            return Err(problem(&format!("File parameter {} cannot be granted; accept Directory and open the file inside the entry", local.name)));
        }
        if !matches!(ty.kind, TypeKind::Stdout | TypeKind::Stdin | TypeKind::Stderr | TypeKind::Clock | TypeKind::Process | TypeKind::Directory | TypeKind::Listener | TypeKind::Connector | TypeKind::Readiness | TypeKind::Entropy | TypeKind::Stop) {
            return Err(problem(&format!("parameter {} requires an owned grantable capability; World must be the only parameter", local.name)));
        }
        if !valid_name(&local.name) || names.contains(&local.name) {
            return Err(problem("grant parameter names must be distinct, nonempty and contain no whitespace or equals sign"));
        }
        names.push(local.name.clone());
    }
    Ok(())
}
fn decimal(text: &str, maximum: u64) -> Result<u64, String> {
    if text.is_empty() { return Err(problem("empty decimal address component")); }
    let mut value = 0u64;
    for byte in text.as_bytes() {
        if *byte < b'0' || *byte > b'9' { return Err(problem("network grant requires numeric IPv4:port")); }
        value = value * 10 + (*byte - b'0') as u64;
        if value > maximum { return Err(problem("network address component is out of range")); }
    }
    Ok(value)
}
fn address(endpoint: &str) -> Result<Vec<u8>, String> {
    let bytes = endpoint.as_bytes();
    let mut colon = None;
    for index in 0..bytes.len() {
        if bytes[index] == b':' {
            if colon.is_some() { return Err(problem("network grant requires numeric IPv4:port")); }
            colon = Some(index);
        }
    }
    let at = colon.ok_or_else(|| problem("network grant requires numeric IPv4:port"))?;
    let port = decimal(&endpoint[at + 1..], 65535)?;
    let mut output = vec![0u8; 16];
    output[0] = 2;
    output[2] = (port >> 8) as u8;
    output[3] = port as u8;
    let mut begin = 0usize;
    let mut part = 0usize;
    for end in 0..=at {
        if end == at || bytes[end] == b'.' {
            if part >= 4 { return Err(problem("network grant needs four IPv4 octets")); }
            output[4 + part] = decimal(&endpoint[begin..end], 255)? as u8;
            part += 1;
            begin = end + 1;
        }
    }
    if part != 4 { return Err(problem("network grant needs four IPv4 octets")); }
    Ok(output)
}
pub fn prepare(program: &Program, entry: usize, supplied: &[String]) -> Result<Vec<EntryGrant>, String> {
    validate(program, entry)?;
    let function = &program.functions[entry];
    if function.params == 1 && matches!(program.types[function.locals[0].ty].kind, TypeKind::World) {
        if !supplied.is_empty() { return Err(problem("sole World entry already declares full authority; named grants are unused")); }
        return Ok(vec![EntryGrant::World]);
    }
    let mut supplied_by_name: BTreeMap<String, Option<String>> = BTreeMap::new();
    for grant in supplied {
        let mut equals = None;
        for index in 0..grant.len() {
            if grant.as_bytes()[index] == b'=' { equals = Some(index); break; }
        }
        let name = match equals { Some(at) => grant[..at].to_string(), None => grant.clone() };
        let value = match equals { Some(at) => Some(grant[at + 1..].to_string()), None => None };
        if !valid_name(&name) { return Err(problem("invalid grant name")); }
        let mut known = false;
        for parameter in 0..function.params {
            if function.locals[parameter].name == name { known = true; }
        }
        if !known { return Err(problem(&format!("unknown grant {name}"))); }
        if supplied_by_name.insert(name.clone(), value).is_some() {
            return Err(problem(&format!("duplicate grant {name}")));
        }
    }
    let mut result: Vec<EntryGrant> = Vec::new();
    for parameter in 0..function.params {
        let local = &function.locals[parameter];
        let value = supplied_by_name.get(&local.name).ok_or_else(|| problem(&format!("missing --grant {}", local.name)))?;
        let kind = &program.types[local.ty].kind;
        let grant = match kind {
            TypeKind::Directory | TypeKind::Listener | TypeKind::Connector => {
                let value = value.as_ref().ok_or_else(|| problem(&format!("--grant {} requires a value", local.name)))?;
                if value.is_empty() { return Err(problem("grant path or address is empty")); }
                for byte in value.as_bytes() {
                    if *byte == 0 { return Err(problem("grant path or address contains a zero byte")); }
                }
                if matches!(kind, TypeKind::Directory) {
                    let mut path = value.as_bytes().to_vec();
                    path.push(0);
                    EntryGrant::Directory(path)
                } else if matches!(kind, TypeKind::Connector) {
                    EntryGrant::Connector(address(value)?)
                } else {
                    EntryGrant::Listener(address(value)?)
                }
            }
            _ => {
                if value.is_some() { return Err(problem(&format!("--grant {} takes no value", local.name))); }
                match kind {
                    TypeKind::Stdout => EntryGrant::Stdout,
                    TypeKind::Stdin => EntryGrant::Stdin,
                    TypeKind::Stderr => EntryGrant::Stderr,
                    TypeKind::Clock => EntryGrant::Clock,
                    TypeKind::Process => EntryGrant::Process,
                    TypeKind::Readiness => EntryGrant::Readiness,
                    TypeKind::Entropy => EntryGrant::Entropy,
                    TypeKind::Stop => EntryGrant::Stop,
                    _ => return Err(problem("ungrantable entry capability")),
                }
            }
        };
        result.push(grant);
    }
    Ok(result)
}
