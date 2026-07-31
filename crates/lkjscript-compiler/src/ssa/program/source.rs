fn lower_source_metadata(source: &hir::Source) -> Result<SourceMetadata> {
    let path = source.path.to_str().ok_or_else(|| {
        Error::msg(format!(
            "validated source path is not UTF-8: {:?}",
            source.path
        ))
    })?;
    Ok(SourceMetadata {
        id: source.id.raw(),
        path: path.to_owned(),
    })
}
