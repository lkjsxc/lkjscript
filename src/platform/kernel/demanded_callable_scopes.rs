//! Ordered lexical IDs shared only within one demanded proof over an immutable reader.
//! A declaration projection is not a witness selection or semantic-validity certificate.
use super::*;

#[derive(Default)]
pub(super) struct Scopes {
    entries: BTreeMap<DeclarationReference, Vec<ImplementationParameterId>>,
}

impl Scopes {
    pub(super) fn ordinal<R: CallableClosureRead + ?Sized>(
        &mut self,
        analysis: &mut Analysis<'_, R>,
        owner: DeclarationReference,
        parameter: ImplementationParameterId,
    ) -> Result<usize, Diagnostic> {
        analysis.tick()?;
        // The complete package/declaration pair is the key, not a matching shape
        // or an implementation parameter ID reused by another lexical owner.
        analysis.comparison_work(2, self.entries.len())?;
        if let Some(parameters) = self.entries.get(&owner) {
            for (ordinal, candidate) in parameters.iter().enumerate() {
                analysis.tick()?;
                if *candidate == parameter {
                    return Ok(ordinal);
                }
            }
            // Preserve the established missing-parameter diagnostic. This path
            // cannot succeed for the same immutable source admitted by this proof.
            let declaration = analysis.declaration(owner)?;
            return analysis.parameter_ordinal(&declaration.payload, parameter);
        }
        let declaration = analysis.declaration(owner)?;
        let ordinal = analysis.parameter_ordinal(&declaration.payload, parameter)?;
        let parameters = implementation_parameters(&declaration.payload);
        analysis.reserve::<(DeclarationReference, Vec<ImplementationParameterId>)>(1)?;
        analysis.reserve::<ImplementationParameterId>(parameters.len())?;
        analysis.steps(parameters.len())?;
        let parameters = parameters.iter().map(|parameter| parameter.id).collect();
        // Failure during loading, reservation, copying or this final checkpoint
        // leaves no partially populated entry available to a later request.
        analysis.tick()?;
        self.entries.insert(owner, parameters);
        Ok(ordinal)
    }
}

#[cfg(test)]
#[path = "demanded_callable_scope_tests.rs"]
mod tests;
