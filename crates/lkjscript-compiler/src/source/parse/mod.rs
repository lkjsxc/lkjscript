mod atom;
mod declaration_shapes;
mod declarations;
mod lex;
mod limits;
mod lines;
mod names;
mod syntax;
mod text;

use std::path::PathBuf;

use crate::source::{SourceFile, SourceNode, SourceOrigin, SourceResult};

pub(crate) use names::is_source_identifier;

pub(crate) fn parse_file(
    source: &str,
    origin: SourceOrigin,
    path: PathBuf,
) -> SourceResult<SourceFile> {
    let exact_source_len = u64::try_from(source.len()).map_err(|_| {
        limits::resource_error(
            &origin,
            crate::source::SourceSpan::zero(),
            "source exceeds the byte-addressable span limit",
        )
    })?;
    if source.len() > u32::MAX as usize {
        return Err(limits::resource_error(
            &origin,
            crate::source::SourceSpan::zero(),
            "source exceeds the byte-addressable span limit",
        ));
    }
    let exact_source_sha256 = lkjscript_core::sha256(source.as_bytes());
    let lines = lines::source_lines(source);
    let lexed = lex::lex(&lines, &origin)?;
    let syntax = syntax::parse_tokens(&lexed.tokens, &origin)?;
    declarations::validate_top_level(&syntax, &origin)?;
    let mut forms = Vec::new();
    forms.try_reserve(syntax.len()).map_err(|_| {
        limits::allocation_error(
            &origin,
            crate::source::SourceSpan::zero(),
            "projected source forms",
        )
    })?;
    forms.extend(SourceNode::project_all(&syntax).map_err(|_| {
        limits::allocation_error(
            &origin,
            crate::source::SourceSpan::zero(),
            "projected source forms",
        )
    })?);
    let identity =
        crate::source::identity::source_identity(&origin, exact_source_len, exact_source_sha256)?;
    Ok(SourceFile {
        path,
        origin,
        identity,
        exact_source_len,
        exact_source_sha256,
        forms,
        syntax,
        tokens: lexed.tokens,
        trailing_trivia: lexed.trailing_trivia,
    })
}
