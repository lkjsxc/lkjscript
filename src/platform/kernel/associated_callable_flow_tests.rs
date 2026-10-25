//! An alias carrying witness-path provenance must not acquire constructor weight.
#![allow(clippy::expect_used)]
use super::*;
use crate::platform::semantic_id::{DeclarationId, TypeParameterId};

struct Empty;
impl CallableClosureRead for Empty {
    fn visit_callable_owners(
        &self,
        _: &mut dyn FnMut(DeclarationReference) -> Result<(), Diagnostic>,
    ) -> Result<(), Diagnostic> {
        Ok(())
    }
    fn owner(&self, _: PackageId, _: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(None)
    }
    fn type_object(
        &self,
        _: PackageId,
        _: TypeObjectDigest,
    ) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(None)
    }
}

#[test]
fn associated_alias_provenance_does_not_make_a_constructor_cycle() {
    let seed = b"associated-flow-explicit-weight";
    let owner = DeclarationReference {
        package: PackageId::migrate(seed, 1),
        declaration: DeclarationId::migrate(seed, 1),
    };
    // Both variants have identical slot topology. Only the second has a real
    // constructor dependency; the nonempty witness path is diagnostic provenance.
    for expanding in [false, true] {
        let mut work = ValidationWork::default();
        let mut analysis = Analysis {
            read: &Empty,
            work: &mut work,
            maximum_work: Some(10_000),
            maximum_metadata_bytes: Some(MAXIMUM_ANALYSIS_BYTES),
            observation: CallableFlowWork::default(),
            contexts: vec![ContextInfo {
                key: Context {
                    owner,
                    method: None,
                    implementations: Vec::new(),
                },
                parameters: vec![TypeParameterId::migrate(seed, 1)],
                witnesses: Vec::new(),
                slots: vec![0, 1],
            }],
            indexes: BTreeMap::new(),
            selections: Vec::new(),
            selection_indexes: BTreeMap::new(),
            pending: Vec::new(),
            calls: Vec::new(),
            slot_owners: vec![(0, 0), (0, 1)],
            edges: Vec::new(),
            associated: State::default(),
            completed_view_queries: BTreeMap::new(),
            diagnostic_context: WorkContext {
                phase: "fixture initialization",
                owner: None,
                context: None,
                expression: None,
                query: None,
            },
        };
        analysis
            .push_edge(Edge {
                from: 0,
                to: 1,
                origin: owner,
                expression: None,
                argument: 0,
                path: vec![2, 0],
                expanding: false,
            })
            .expect("alias edge");
        analysis
            .push_edge(Edge {
                from: 1,
                to: 0,
                origin: owner,
                expression: None,
                argument: 0,
                path: if expanding { vec![0] } else { Vec::new() },
                expanding,
            })
            .expect("return edge");
        let result = analysis.admit();
        if expanding {
            let error = result.expect_err("a constructor inside the slot SCC must refuse");
            assert_eq!(error.class, DiagnosticClass::Semantic);
            assert_eq!(error.code, "kernel_callable_expansion");
        } else {
            result.expect("zero aliases remain finite despite nonempty provenance");
        }
    }
}
