use std::path::{Path, PathBuf};

#[cfg(test)]
use crate::source::SourceOrigin;
use crate::source::{DeclarationSummary, SourceFile};

pub(crate) struct ValidatedSourceParts {
    pub(crate) root: PathBuf,
    #[cfg(test)]
    pub(crate) root_origin: SourceOrigin,
    pub(crate) files: Vec<SourceFile>,
    pub(crate) declarations: Vec<DeclarationSummary>,
}

/// Private parsed input consumed exactly once by the workspace importer.
#[derive(Clone, Debug)]
pub(crate) struct ValidatedSourceTree {
    root: PathBuf,
    #[cfg(test)]
    root_origin: SourceOrigin,
    files: Vec<SourceFile>,
    declarations: Vec<DeclarationSummary>,
}

impl ValidatedSourceTree {
    pub(crate) fn from_authority(parts: ValidatedSourceParts) -> Self {
        Self {
            root: parts.root,
            #[cfg(test)]
            root_origin: parts.root_origin,
            files: parts.files,
            declarations: parts.declarations,
        }
    }

    pub(crate) fn declarations(&self) -> &[DeclarationSummary] {
        &self.declarations
    }

    pub(crate) fn module_scoped_projection(&self) -> crate::source::SourceResult<Self> {
        let mut projection = self.clone();
        crate::source::modules::scope(&mut projection.files, &mut projection.declarations)?;
        Ok(projection)
    }

    pub(crate) fn files(&self) -> &[SourceFile] {
        &self.files
    }

    pub(crate) fn root_path(&self) -> &Path {
        &self.root
    }

    #[cfg(test)]
    pub(crate) fn root_origin(&self) -> &SourceOrigin {
        &self.root_origin
    }
}
