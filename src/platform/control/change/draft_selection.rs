//! Read-only local selectors. Names resolve once, into exact canonical draft owners.
use super::canonical::{Reader, error};
use super::*;
use crate::platform::kernel::owner_namespace;
use crate::platform::witness::NamespaceKey;

#[derive(Clone, Debug)]
pub(crate) enum NativeDraftSelection {
    Owner(OwnerKey),
    Module(Name),
    Declaration { module: Name, name: Name },
    Target(Name),
}

impl From<OwnerKey> for NativeDraftSelection {
    fn from(owner: OwnerKey) -> Self {
        Self::Owner(owner)
    }
}

fn selection_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, "change_draft_selection", message)
}

impl NativeDraftSelection {
    pub(crate) fn parse(option: &str, value: &str) -> Result<Self, Diagnostic> {
        let name = |value: &str| {
            Name::new(value).map_err(|e| selection_error(format!("{option}: {}", e.message)))
        };
        match option {
            "--owner" => value.parse().map(Self::Owner),
            "--module" => name(value).map(Self::Module),
            "--target" => name(value).map(Self::Target),
            "--declaration" => {
                let (module, declaration) = value.split_once("::").ok_or_else(|| {
                    selection_error("--declaration requires an exact local MODULE::NAME")
                })?;
                Ok(Self::Declaration {
                    module: name(module)?,
                    name: name(declaration)?,
                })
            }
            _ => Err(selection_error("unknown native draft selection option")),
        }
    }

    pub(super) fn resolve(&self, reader: &mut Reader<'_>) -> Result<OwnerKey, Diagnostic> {
        reader.check()?;
        match self {
            Self::Owner(owner) => Ok(*owner),
            Self::Module(name) => resolve_name(reader, NamespaceClass::Module, None, name),
            Self::Target(name) => resolve_name(reader, NamespaceClass::Target, None, name),
            Self::Declaration { module, name } => {
                let module = resolve_name(reader, NamespaceClass::Module, None, module)?;
                resolve_name(reader, NamespaceClass::Declaration, Some(module), name)
            }
        }
    }
}

fn resolve_name(
    reader: &mut Reader<'_>,
    class: NamespaceClass,
    parent: Option<OwnerKey>,
    name: &Name,
) -> Result<OwnerKey, Diagnostic> {
    reader.check()?;
    let key = NamespaceKey {
        class,
        parent,
        name: name.clone(),
    };
    let owner = reader.reader.namespace(&key)?.ok_or_else(|| {
        selection_error(format!(
            "local {} '{}' is absent under {:?} at revision '{}'",
            class.name(),
            name,
            parent,
            reader.view.revision()
        ))
    })?;
    // A namespace witness is an index, not authority. Recheck the canonical owner
    // with the same cumulative reader and immutable revision used by rendering.
    let record = reader.owner(owner)?;
    let Some(actual) = owner_namespace(&record) else {
        return Err(error(
            "draft name index selects an owner without a namespace",
        ));
    };
    if actual.class != class || actual.parent != parent || actual.name != name {
        return Err(error("draft name index disagrees with its canonical owner"));
    }
    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::execution::ExecutionControl;
    use crate::platform::kernel::OwnerRecord;
    use crate::platform::publication::{GraphRepository, RepositoryDefinitionAdmission};

    #[test]
    fn named_draft_snapshot_stays_pinned_after_a_concurrent_rename() {
        let temporary = tempfile::tempdir().unwrap();
        let initial = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
        let (owner, name) = initial
            .owners
            .iter()
            .find_map(|(owner, record)| {
                if let OwnerRecord::Module(module) = record {
                    Some((*owner, module.name.clone()))
                } else {
                    None
                }
            })
            .unwrap();
        let selection = NativeDraftSelection::Module(name);
        let view = created.repository.view_current().unwrap();
        let mut reader = Reader::new(&view, ExecutionControl::uncancelled());
        assert_eq!(selection.resolve(&mut reader).unwrap(), owner);
        let input = format!(
            "request base={}\nrename.owner owner={} name=renamed-for-draft\n",
            view.revision(),
            owner
        );
        let request = decode_compact_change_in_repository(
            "rename.lkjc",
            input.as_bytes(),
            &created.repository,
        )
        .unwrap();
        let prepared = created
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        created.repository.publish(&prepared.publication).unwrap();
        assert_eq!(selection.resolve(&mut reader).unwrap(), owner);
        let current = created.repository.view_current().unwrap();
        let mut fresh = Reader::new(&current, ExecutionControl::uncancelled());
        assert_eq!(
            selection.resolve(&mut fresh).unwrap_err().code,
            "change_draft_selection"
        );
        let renamed = NativeDraftSelection::parse("--module", "renamed-for-draft").unwrap();
        assert_eq!(renamed.resolve(&mut fresh).unwrap(), owner);
        let control = ExecutionControl::uncancelled();
        control.cancel();
        assert_eq!(
            renamed
                .resolve(&mut Reader::new(&current, control))
                .unwrap_err()
                .class,
            DiagnosticClass::Cancelled
        );
    }

    #[test]
    fn named_draft_lookups_share_admission_and_recheck_the_canonical_name() {
        let temporary = tempfile::tempdir().unwrap();
        let initial = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
        let (owner, record, name) = initial
            .owners
            .iter()
            .find_map(|(owner, record)| {
                if let OwnerRecord::Module(module) = record {
                    Some((*owner, record.clone(), module.name.clone()))
                } else {
                    None
                }
            })
            .unwrap();
        let selection = NativeDraftSelection::Module(name);
        let view = created.repository.view_current().unwrap();
        let mut reader = Reader::new(&view, ExecutionControl::uncancelled());
        let mut limits = RepositoryDefinitionAdmission::maximum();
        limits.ownership_records = 1;
        limits.summary_records = 0;
        reader.reader = view.definition_reader_with_admission(limits);
        assert_eq!(selection.resolve(&mut reader).unwrap(), owner);
        assert_eq!(
            selection.resolve(&mut reader).unwrap_err().code,
            "definition_admission_witness_records"
        );
        // Inject a mismatching cached canonical record, not an alternate name index.
        // Resolution must refuse it instead of trusting the correct-looking witness.
        let mut reader = Reader::new(&view, ExecutionControl::uncancelled());
        let mut wrong = record;
        *wrong.name_mut().unwrap() = Name::new("wrong-canonical-name").unwrap();
        reader.owners.insert(owner, wrong);
        assert_eq!(
            selection.resolve(&mut reader).unwrap_err().code,
            "change_draft_definition"
        );
    }
}
