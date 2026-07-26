use std::collections::{BTreeMap, BTreeSet};

use crate::source::Expr;

pub(super) fn declaration_names(kind: &str, args: &[Expr], names: &mut BTreeSet<String>) {
    if let Some(name) = named(args) {
        names.insert(name.to_string());
    }
    if kind != "enum" {
        return;
    }
    let Some(Expr::Call { args: variants, .. }) =
        args.iter().find(|item| call_name(item) == Some("variants"))
    else {
        return;
    };
    for variant in variants {
        if let Expr::Call { args, .. } = variant {
            if let Some(name) = named(args) {
                names.insert(name.to_string());
            }
        }
    }
}

pub(super) fn top_level(form: &mut Expr, bindings: &BTreeMap<String, String>) {
    let Expr::Call { name, args } = form else {
        return;
    };
    if matches!(name.as_str(), "def" | "product" | "enum" | "trait") {
        rewrite_named(args, bindings);
        if name == "enum" {
            if let Some(Expr::Call { args: variants, .. }) = args
                .iter_mut()
                .find(|item| call_name(item) == Some("variants"))
            {
                for variant in variants {
                    if let Expr::Call { args, .. } = variant {
                        rewrite_named(args, bindings);
                    }
                }
            }
        }
        args.retain(|arg| !matches!(arg, Expr::Symbol(name) if name == "public"));
    }
    rewrite_uses(form, bindings);
}

fn rewrite_named(args: &mut [Expr], bindings: &BTreeMap<String, String>) {
    let Some(Expr::Call { args: values, .. }) =
        args.iter_mut().find(|item| call_name(item) == Some("name"))
    else {
        return;
    };
    let Some(Expr::LitStr(name)) = values.first_mut() else {
        return;
    };
    if let Some(replacement) = bindings.get(name) {
        *name = replacement.clone();
    }
}

fn rewrite_uses(expr: &mut Expr, bindings: &BTreeMap<String, String>) {
    match expr {
        Expr::Call { name, args } => {
            if let Some(replacement) = bindings.get(name) {
                *name = replacement.clone();
            }
            for argument in args {
                rewrite_uses(argument, bindings);
            }
        }
        Expr::Symbol(name) => {
            if let Some(replacement) = bindings.get(name) {
                *name = replacement.clone();
            }
        }
        Expr::List(items) => {
            for item in items {
                rewrite_uses(item, bindings);
            }
        }
        _ => {}
    }
}

fn named(args: &[Expr]) -> Option<&str> {
    let Expr::Call { args, .. } = args.iter().find(|item| call_name(item) == Some("name"))? else {
        return None;
    };
    match args.as_slice() {
        [Expr::LitStr(name)] => Some(name),
        _ => None,
    }
}

fn call_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Call { name, .. } => Some(name),
        _ => None,
    }
}
