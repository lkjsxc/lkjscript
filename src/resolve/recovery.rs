//! Private diagnostic resolution: successful paths retain their real identities,
//! while failures remain unavailable source nodes. This graph never authorizes K.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Site {
    Import(usize),
    Type(usize),
    Pattern(usize),
    Expression(usize),
}
pub(crate) struct Failure {
    pub site: Site,
    pub problem: Diagnostic,
}
pub(crate) struct Report {
    pub program: Program,
    pub failures: Vec<Failure>,
}

pub(crate) fn resolve(source: &Program) -> Result<Report> {
    let (resolver, imports) = Resolver::new_recovery(source)?;
    let mut failures = Vec::new();
    for failure in imports {
        failures.push(Failure {
            site: Site::Import(failure.import),
            problem: failure.problem,
        });
    }
    let mut program = source.clone();
    resolve_nodes(&mut program, &resolver, &mut |site, problem| {
        failures.push(Failure { site, problem });
        Ok(())
    })?;
    let mut imports = Vec::new();
    for failure in &failures {
        if let Site::Import(id) = failure.site {
            imports.push(id);
        }
    }
    for (id, node) in source.types.iter().enumerate() {
        if let Type::Path(path) = &node.kind {
            if let Some(import) = resolver.uncertain_import(
                &imports,
                node.scope,
                &path.name,
                Namespace::Type,
                node.span,
            ) {
                failures.push(Failure {
                    site: Site::Type(id),
                    problem: Diagnostic::new(
                        node.span,
                        "E_BLOCKED",
                        &format!(
                            "lookup {} depends on unresolved import {}",
                            path.name, source.uses[import].path
                        ),
                    ),
                });
            }
        }
    }
    program.declarations = resolver.declarations;
    program.function_owners = resolver
        .function_owners
        .unwrap_or_else(|| source.function_owners.clone());
    // This records canonical identity, not successful resolution. The graph is
    // confined to diagnostic orchestration; strict callers still use resolve.
    program.resolved = true;
    Ok(Report { program, failures })
}
