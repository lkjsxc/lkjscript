mod atom;
mod declaration_shapes;
mod declarations;
mod element;
mod lex;
mod limits;
mod lines;
mod names;
mod syntax;
mod text;

use std::path::PathBuf;

use lkjscript_core::Limits;

use crate::source::{
    validate::check_foundation_file_bytes, SourceFile, SourceNode, SourceOrigin, SourceResult,
};

pub(crate) use names::is_source_identifier;

pub(crate) fn parse_file(
    source: &str,
    origin: SourceOrigin,
    path: PathBuf,
    limits: &Limits,
) -> SourceResult<SourceFile> {
    let exact_source_len = u64::try_from(source.len()).map_err(|_| {
        limits::resource_error(
            &origin,
            crate::source::SourceSpan::zero(),
            "source exceeds the Edition 1 byte-addressable span limit",
        )
    })?;
    check_foundation_file_bytes(&origin, exact_source_len)?;
    if source.len() > u32::MAX as usize {
        return Err(limits::resource_error(
            &origin,
            crate::source::SourceSpan::zero(),
            "source exceeds the Edition 1 byte-addressable span limit",
        ));
    }
    let exact_source_sha256 = lkjscript_core::sha256(source.as_bytes());
    let lines = lines::source_lines(source);
    let lexed = lex::lex(&lines, &origin)?;
    limits::check_file_limits(&lexed.tokens, limits, &origin)?;
    let mut syntax = syntax::parse_tokens(&lexed.tokens, &origin)?;
    let edition =
        crate::source::edition::validate_marker(source, &mut syntax, &lexed.tokens, &origin)?;
    let declarations = if edition == crate::source::SourceEdition::Edition2 {
        &syntax[1..]
    } else {
        &syntax
    };
    declarations::validate_top_level(declarations, limits, &origin)?;
    let forms = declarations.iter().map(SourceNode::project).collect();
    let identity = crate::source::identity::source_identity(
        edition,
        origin.logical_path(),
        exact_source_len,
        exact_source_sha256,
    );
    Ok(SourceFile {
        path,
        origin,
        edition,
        identity,
        exact_source: source.to_owned(),
        exact_source_len,
        exact_source_sha256,
        forms,
        syntax,
        tokens: lexed.tokens,
        trailing_trivia: lexed.trailing_trivia,
    })
}
