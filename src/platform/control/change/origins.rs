//! Ephemeral source locations for one proposal. They are excluded from authored intent and all
//! persistent meaning. Exact owner allocation is delegated to the existing authored owner.

use super::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct InputOrigins {
    pub symbols: BTreeMap<String, SourceLocation>,
    pub tokens: BTreeMap<String, SourceLocation>,
    pub mutations: Vec<(String, SourceLocation)>,
    pub private: BTreeSet<String>,
}

impl InputOrigins {
    pub fn locate(&self, error: &mut Diagnostic, owners: &BTreeMap<String, OwnerKey>) {
        if error.location.is_none() {
            for note in &error.notes {
                if let Some(expression) = note.strip_prefix("expression owner: ") {
                    for (symbol, owner) in owners {
                        if owner.to_string() == expression
                            && let Some(location) = self.symbols.get(symbol)
                        {
                            error.location = Some(location.clone());
                            break;
                        }
                    }
                }
            }
        }
        if error.location.is_none() {
            let text = std::iter::once(error.message.as_str())
                .chain(error.notes.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join("\n");
            let mut sites = self.mutations.iter().filter_map(|(selector, location)| {
                let target = owners
                    .get(selector)
                    .map_or_else(|| selector.clone(), ToString::to_string);
                token_in(&text, &target).then_some(location)
            });
            // Ordered lowering stops at its first invalid target. Final-candidate validation
            // instead observes the last type assigned to an existing parameter.
            error.location = if matches!(
                error.code.as_str(),
                "change_mutation_owner_kind"
                    | "change_authored_owner_missing"
                    | "change_authored_owner_deleted"
            ) {
                sites.next()
            } else {
                sites.next_back()
            }
            .cloned();
            if error.location.is_none() {
                for (symbol, owner) in owners {
                    if token_in(&text, &owner.to_string())
                        && let Some(location) = self.symbols.get(symbol)
                    {
                        error.location = Some(location.clone());
                        break;
                    }
                }
            }
            if error.location.is_none() {
                for (token, location) in self.tokens.iter().chain(self.symbols.iter()) {
                    if token.starts_with('$') && token_in(&text, token) {
                        error.location = Some(location.clone());
                        break;
                    }
                }
            }
        }
        for symbol in &self.private {
            if error.message.contains(symbol) {
                error.message = error.message.replace(symbol, "<lexical occurrence>");
            }
        }
    }
}

fn token_in(text: &str, token: &str) -> bool {
    text.match_indices(token).any(|(offset, _)| {
        text.as_bytes()
            .get(offset + token.len())
            .is_none_or(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'_' | b'-'))
    })
}
