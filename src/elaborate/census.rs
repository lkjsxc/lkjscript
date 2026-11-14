//! Diagnostic inference never returns an executable or checked representation.
//!
//! Imports affect visibility within the supplied source graph, not its complete
//! implementation inventory. An empty, complete candidate domain can therefore
//! establish absence despite an unrelated failed import. An unloaded module or
//! excluded/recovered implementation makes that domain incomplete; a known
//! trait candidate behind an unavailable import remains a visibility blocker.
//! Type constraints may use real nominal identities without their field layout.
//! Ownership recovery is deliberately narrower: closed structural bodies use
//! the ordinary builder/checker, while nominal, generic, named-lifetime, and
//! callable contracts wait for their complete declaration checks.
use super::{a, definition_types, generic_check, methods};
use crate::diagnostics::lookup_domains::LookupDomains;
use crate::diagnostics::readiness::Readiness;
use crate::diagnostics::{SemanticObservation, SemanticReport};
use crate::resolve::{Namespace, Resolver};
use std::collections::BTreeSet;

pub(super) struct Context {
    pub(super) readiness: Readiness,
    pub(super) domains: LookupDomains,
    pub(super) imports: Vec<usize>,
}
impl Context {
    pub(super) fn path_problem(
        &self,
        names: &Resolver<'_>,
        source: &a::Program,
        scope: a::ModuleId,
        name: &str,
        namespace: Namespace,
        span: a::Span,
    ) -> Option<a::Diagnostic> {
        if let Some(import) = names.uncertain_import(&self.imports, scope, name, namespace, span) {
            return Some(a::Diagnostic::new(
                span,
                "E_BLOCKED",
                &format!(
                    "lookup {name} depends on unresolved import {}",
                    source.uses[import].path
                ),
            ));
        }
        self.domains
            .name_problem(names, scope, name, namespace, span)
            .cloned()
    }
}
fn function_name(source: &a::Program, function: usize) -> String {
    let definition = &source.functions[function];
    match source.function_owner(a::FunctionId { index: function }) {
        a::FunctionOwner::Free => crate::names::qualified(&definition.module, &definition.name),
        _ => format!(
            "{} at {}:{}",
            crate::names::qualified(&definition.module, &definition.name),
            definition.span.line,
            definition.span.column
        ),
    }
}
pub(super) fn record(
    report: &mut SemanticReport,
    source: &a::Program,
    function: Option<usize>,
    phase: &str,
    mut problem: a::Diagnostic,
    receiver: Option<String>,
    member: Option<String>,
) {
    if problem.path.is_empty() {
        if let Some(file) = source.files.get(problem.file) {
            problem.path = file.path.clone();
        }
    }
    let observation = SemanticObservation {
        function: function.map(|id| function_name(source, id)),
        phase: phase.to_string(),
        problem,
        receiver,
        member,
    };
    if observation.problem.code == "E_BLOCKED" {
        report.blocked.push(observation);
    } else {
        report.observations.push(observation);
    }
}
pub(super) fn blocked(span: a::Span, message: &str) -> a::Diagnostic {
    a::Diagnostic::new(span, "E_BLOCKED", message)
}
// Partly parsed items can leave orphan nodes in the syntax arenas. Their
// resolution failures describe skipped input, not absent declarations.
fn qualify_skipped_nodes(
    resolved: &mut crate::resolve::recovery::Report,
    skipped: &[crate::diagnostics::BlockedRegion],
) {
    let mut regions: Vec<(usize, usize, usize)> = Vec::new();
    for (index, region) in skipped.iter().enumerate() {
        if matches!(region.kind.as_str(), "item" | "statement" | "file") {
            regions.push((region.span.file, region.span.start, index));
        }
    }
    regions.sort();
    let mut ranges: Vec<(a::Span, usize)> = Vec::new();
    for (_, _, index) in regions {
        let region = &skipped[index];
        if !ranges.is_empty() {
            let last_index = ranges.len() - 1;
            let last = ranges[last_index].0;
            if last.file == region.span.file && region.span.start <= last.end {
                ranges[last_index].0.end = last.end.max(region.span.end);
                continue;
            }
        }
        ranges.push((region.span, index));
    }
    let mut failure_index = 0;
    while failure_index < resolved.failures.len() {
        let failure = &mut resolved.failures[failure_index];
        let source = &resolved.program;
        let span = match failure.site {
            crate::resolve::recovery::Site::Import(id) => source.uses[id].span,
            crate::resolve::recovery::Site::Type(id) => source.types[id].span,
            crate::resolve::recovery::Site::Pattern(id) => source.patterns[id].span,
            crate::resolve::recovery::Site::Expression(id) => source.exprs[id].span,
        };
        let mut low = 0;
        let mut high = ranges.len();
        while low < high {
            let middle = low + (high - low) / 2;
            let range = ranges[middle].0;
            if (range.file, range.start) <= (span.file, span.start) {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        if low > 0 {
            let (range, region_index) = ranges[low - 1];
            if range.file == span.file && span.end <= range.end {
                let reason = &skipped[region_index].reason;
                failure.problem = blocked(span, &format!("lookup lies in skipped input: {reason}"));
            }
        }
        failure_index += 1;
    }
}
pub(crate) fn report(input: &crate::source::SourceReport) -> SemanticReport {
    let mut output = SemanticReport::default();
    let mut resolved = match crate::resolve::recovery::resolve(&input.program) {
        Ok(resolved) => resolved,
        Err(problem) => {
            record(
                &mut output,
                &input.program,
                None,
                "resolution",
                problem,
                None,
                None,
            );
            record(
                &mut output,
                &input.program,
                None,
                "source types",
                blocked(a::Span::default(), "declaration identities are unavailable"),
                None,
                None,
            );
            return output;
        }
    };
    qualify_skipped_nodes(&mut resolved, &input.blocked);
    let source = &resolved.program;
    let (names, _) = match Resolver::new_recovery(source) {
        Ok(value) => value,
        Err(problem) => {
            record(&mut output, source, None, "resolution", problem, None, None);
            return output;
        }
    };
    let mut readiness = Readiness::new(&resolved);
    let (_, failed_index) = crate::generics::ImplementationIndex::for_diagnostics(
        source,
        &readiness.implementation_headers,
    );
    let domains = LookupDomains::new(
        source,
        &readiness.types,
        &readiness.implementation_headers,
        &failed_index,
        &input.blocked,
    )
    .propagate_globs(&names);
    let mut additional = Vec::new();
    for (id, node) in input.program.types.iter().enumerate() {
        if let a::Type::Path(path) = &node.kind {
            if let Some(problem) =
                domains.name_problem(&names, node.scope, &path.name, Namespace::Type, node.span)
            {
                additional.push(crate::resolve::recovery::Failure {
                    site: crate::resolve::recovery::Site::Type(id),
                    problem: problem.clone(),
                });
            }
        }
    }
    if !additional.is_empty() {
        for failure in additional {
            resolved.failures.push(failure);
        }
        readiness = Readiness::new(&resolved);
    }
    let (implementations, failed_index) = crate::generics::ImplementationIndex::for_diagnostics(
        source,
        &readiness.implementation_headers,
    );
    let domains = LookupDomains::new(
        source,
        &readiness.types,
        &readiness.implementation_headers,
        &failed_index,
        &input.blocked,
    )
    .propagate_globs(&names);
    let mut imports = Vec::new();
    for failure in &resolved.failures {
        if let crate::resolve::recovery::Site::Import(id) = failure.site {
            imports.push(id);
        }
    }
    let context = Context {
        readiness,
        domains,
        imports,
    };
    let mut blocked_sites: BTreeSet<crate::resolve::recovery::Site> = BTreeSet::new();
    for failure in &resolved.failures {
        if failure.problem.code == "E_BLOCKED" {
            blocked_sites.insert(failure.site);
        }
    }
    for failure in &resolved.failures {
        if failure.problem.code != "E_BLOCKED" && blocked_sites.contains(&failure.site) {
            continue;
        }
        record(
            &mut output,
            source,
            None,
            "resolution",
            failure.problem.clone(),
            None,
            None,
        );
    }
    for problem in &context.readiness.extra_problems {
        record(
            &mut output,
            source,
            None,
            "signature",
            problem.clone(),
            None,
            None,
        );
    }
    for (_, problem) in &failed_index {
        record(
            &mut output,
            source,
            None,
            "implementation",
            problem.clone(),
            None,
            None,
        );
    }
    for problem in &input.diagnostics {
        if crate::diagnostics::semantic_problem(problem) {
            record(
                &mut output,
                source,
                None,
                "previous analysis",
                problem.clone(),
                None,
                None,
            );
        }
    }
    // When all global declarations are present, retain the complete ordinary
    // inference/construction/check pipeline, including independent ownership errors.
    let syntax_complete = input.blocked.iter().all(|region| {
        matches!(
            region.kind.as_str(),
            "semantic phase" | "remaining semantic analysis"
        )
    });
    if resolved.failures.is_empty() && syntax_complete {
        let declaration_problems = match super::infer_resolved_report(source) {
            Ok(definitions) => {
                let mut body_count = 0;
                for function in &source.functions {
                    if function.body.is_some() {
                        body_count += 1;
                    }
                }
                crate::diagnostics::add_count(
                    &mut output.coverage,
                    "complete pipeline attempted",
                    body_count,
                );
                match super::lower_definitions(definitions) {
                    Ok(program) => {
                        if let Err(problems) = crate::check::check_all(program) {
                            for problem in problems {
                                record(&mut output, source, None, "ownership", problem, None, None);
                            }
                        }
                    }
                    Err(problems) => {
                        for problem in problems {
                            record(
                                &mut output,
                                source,
                                None,
                                "complete pipeline",
                                problem,
                                None,
                                None,
                            );
                        }
                    }
                }
                output.deduplicate();
                return output;
            }
            Err(problems) => problems,
        };
        for problem in declaration_problems {
            record(
                &mut output,
                source,
                None,
                "declaration validation",
                problem,
                None,
                None,
            );
        }
    }
    let mut definitions = std::collections::BTreeMap::new();
    for (function, definition) in source.functions.iter().enumerate() {
        if definition.body.is_none() {
            continue;
        }
        crate::diagnostics::count(&mut output.coverage, "bodies present");
        if let Some(problem) = context.readiness.signature_problem(function) {
            record(
                &mut output,
                source,
                Some(function),
                "source types",
                problem.clone(),
                None,
                None,
            );
            continue;
        }
        if let Some(problem) = context
            .readiness
            .body_problem(source, function, &input.blocked)
        {
            record(
                &mut output,
                source,
                Some(function),
                "source types",
                problem,
                None,
                None,
            );
            continue;
        }
        let body = source.blocks[definition.body.unwrap()].span;
        if let Some(failure) = resolved.failures.iter().find(|failure| {
            let span = match failure.site {
                crate::resolve::recovery::Site::Pattern(id) => source.patterns[id].span,
                crate::resolve::recovery::Site::Expression(id) => source.exprs[id].span,
                _ => return false,
            };
            span.file == body.file && span.start >= body.start && span.end <= body.end
        }) {
            record(
                &mut output,
                source,
                Some(function),
                "source types",
                blocked(
                    body,
                    &format!(
                        "body resolution is unavailable: {}",
                        failure.problem.message
                    ),
                ),
                None,
                None,
            );
            continue;
        }
        crate::diagnostics::count(&mut output.coverage, "source bodies attempted");
        let inferred = match definition_types::infer_available(
            source,
            &names,
            &implementations,
            function,
            Some(&context),
        ) {
            Ok(inferred) => inferred,
            Err(problem) => {
                let unavailable = problem.code == "E_BLOCKED";
                record(
                    &mut output,
                    source,
                    Some(function),
                    "source types",
                    problem,
                    None,
                    None,
                );
                crate::diagnostics::count(
                    &mut output.coverage,
                    if unavailable {
                        "source attempts blocked"
                    } else {
                        "source attempts failed"
                    },
                );
                continue;
            }
        };
        crate::diagnostics::count(&mut output.coverage, "source inference returned facts");
        for (_, problem) in inferred.unresolved_paths() {
            record(
                &mut output,
                source,
                Some(function),
                "value lookup",
                problem.clone(),
                None,
                None,
            );
        }
        for expression in inferred.expressions() {
            let node = &source.exprs[expression];
            let a::Expr::Method { receiver, name, .. } = &node.kind else {
                continue;
            };
            let Some(receiver) = inferred.receiver(*receiver) else {
                record(
                    &mut output,
                    source,
                    Some(function),
                    "method lookup",
                    blocked(node.span, "method receiver type is unavailable"),
                    None,
                    Some(name.clone()),
                );
                continue;
            };
            let display = receiver.display(source);
            let head = receiver.head(source);
            if methods::direct(receiver.primitive(), name).is_some() {
                crate::diagnostics::count(
                    &mut output.coverage,
                    "direct method sites with known receiver",
                );
            } else if let Some(problem) =
                context
                    .domains
                    .method_problem(head.as_ref().map(|name| name.as_str()), name, false)
            {
                record(
                    &mut output,
                    source,
                    Some(function),
                    "method lookup",
                    blocked(
                        node.span,
                        &format!("{display}::{name}: {}", problem.message),
                    ),
                    Some(display),
                    Some(name.clone()),
                );
            } else if receiver.rigid() {
                record(
                    &mut output,
                    source,
                    Some(function),
                    "method lookup",
                    blocked(
                        node.span,
                        "generic method selection requires the declared trait contracts",
                    ),
                    Some(display),
                    Some(name.clone()),
                );
            } else if generic_check::BuiltinMethod::named(name).is_some() {
                record(
                    &mut output,
                    source,
                    Some(function),
                    "method lookup",
                    blocked(
                        node.span,
                        "builtin trait method requires checked receiver prerequisites",
                    ),
                    Some(display),
                    Some(name.clone()),
                );
            } else {
                let candidates = implementations
                    .method_candidates_by_head(head.as_ref().map(|name| name.as_str()), name);
                if candidates.is_empty() {
                    record(
                        &mut output,
                        source,
                        Some(function),
                        "method lookup",
                        a::Diagnostic::new(
                            node.span,
                            "E_UNSUPPORTED",
                            "method resolution requires one implementation",
                        ),
                        Some(display),
                        Some(name.clone()),
                    );
                } else {
                    let visibility = candidates.iter().any(|id| source.impls[*id].trait_ty.is_some_and(|ty| {
                        matches!(&source.types[ty].kind, a::Type::Path(path) if path.declaration.is_some_and(|declaration| !names.trait_in_scope(node.scope, declaration, node.span)))
                    }));
                    let uncertain_import = if visibility {
                        names.uncertain_trait_import(&context.imports, node.scope, node.span)
                    } else {
                        None
                    };
                    let reason: String = if let Some(import) = uncertain_import {
                        format!(
                            "trait candidate visibility depends on unresolved import {}",
                            source.uses[import].path
                        )
                    } else {
                        "method candidates require complete signature, prerequisite, and visibility checks".to_string()
                    };
                    record(
                        &mut output,
                        source,
                        Some(function),
                        "method lookup",
                        blocked(node.span, &reason),
                        Some(display),
                        Some(name.clone()),
                    );
                }
            }
        }
        definitions.insert(function, inferred);
    }
    super::census_ownership::analyze(
        source,
        names,
        implementations,
        &context,
        definitions,
        &mut output,
    );
    output.deduplicate();
    output
}
