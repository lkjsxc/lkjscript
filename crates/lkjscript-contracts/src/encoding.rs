use std::collections::BTreeSet;

use crate::model::{
    ContractDependency, ContractDescriptor, ContractError, ContractFact, ContractItem,
    FactOrdering, NameIdentity,
};

const MAX_TEXT_BYTES: usize = 32 * 1024;
const MAX_ITEMS: usize = 4_096;
const MAX_FACTS: usize = 4_096;

pub fn canonical_bytes(descriptor: &ContractDescriptor) -> Result<Vec<u8>, ContractError> {
    if descriptor.items.len() > MAX_ITEMS || descriptor.dependencies.len() > MAX_ITEMS {
        return Err(ContractError::LengthOverflow);
    }
    let mut output = Encoder::default();
    output.frame(b"lkjscript.contract-descriptor")?;
    output.frame(b"sha256")?;
    output.text(descriptor.name.as_str())?;

    let mut dependencies: Vec<_> = descriptor.dependencies.iter().collect();
    dependencies.sort_by(|left, right| left.name.cmp(&right.name));
    reject_duplicate_dependencies(&dependencies)?;
    output.count(dependencies.len())?;
    for dependency in dependencies {
        output.text(dependency.name.as_str())?;
        output.frame(&dependency.digest)?;
    }

    let mut items: Vec<_> = descriptor.items.iter().collect();
    items.sort_by(|left, right| left.stable_id.cmp(&right.stable_id));
    reject_duplicate_items(&items)?;
    output.count(items.len())?;
    for item in items {
        encode_item(&mut output, item)?;
    }
    Ok(output.bytes)
}

fn reject_duplicate_dependencies(values: &[&ContractDependency]) -> Result<(), ContractError> {
    for pair in values.windows(2) {
        if pair[0].name == pair[1].name {
            return Err(ContractError::DuplicateDependency(
                pair[0].name.as_str().to_owned(),
            ));
        }
    }
    Ok(())
}

fn reject_duplicate_items(values: &[&ContractItem]) -> Result<(), ContractError> {
    for item in values {
        validate_stable_id(&item.stable_id)?;
    }
    for pair in values.windows(2) {
        if pair[0].stable_id == pair[1].stable_id {
            return Err(ContractError::DuplicateItem(pair[0].stable_id.clone()));
        }
    }
    Ok(())
}

fn encode_item(output: &mut Encoder, item: &ContractItem) -> Result<(), ContractError> {
    if item.facts.len() > MAX_FACTS {
        return Err(ContractError::LengthOverflow);
    }
    output.byte(item.kind.tag());
    output.text(&item.stable_id)?;
    output.byte(match item.fact_ordering {
        FactOrdering::StableIdentity => 1,
        FactOrdering::Semantic => 2,
    });
    let mut facts = Vec::with_capacity(item.facts.len());
    let mut identities = BTreeSet::new();
    for fact in &item.facts {
        validate_stable_id(&fact.stable_id)?;
        if !identities.insert(&fact.stable_id) {
            return Err(ContractError::DuplicateFact(fact.stable_id.clone()));
        }
        facts.push(encode_fact(fact)?);
    }
    if item.fact_ordering == FactOrdering::StableIdentity {
        facts.sort();
    }
    output.count(facts.len())?;
    for fact in facts {
        output.frame(&fact)?;
    }
    Ok(())
}

fn encode_fact(fact: &ContractFact) -> Result<Vec<u8>, ContractError> {
    let mut output = Encoder::default();
    output.text(&fact.stable_id)?;
    output.byte(u8::from(fact.required));
    output.byte(u8::from(fact.closed));
    output.byte(match fact.name_identity {
        NameIdentity::Included => 1,
        NameIdentity::Metadata => 2,
    });
    if fact.name_identity == NameIdentity::Included {
        output.text(&fact.name)?;
    }
    output.text(&fact.value)?;
    Ok(output.bytes)
}

fn validate_stable_id(value: &str) -> Result<(), ContractError> {
    let valid = !value.is_empty()
        && value.len() <= 240
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'-' | b'_' | b'/' | b':')
        });
    if valid {
        Ok(())
    } else {
        Err(ContractError::InvalidStableId(value.to_owned()))
    }
}

#[derive(Default)]
struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    fn byte(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn count(&mut self, value: usize) -> Result<(), ContractError> {
        let value = u64::try_from(value).map_err(|_| ContractError::LengthOverflow)?;
        self.bytes.extend_from_slice(&value.to_be_bytes());
        Ok(())
    }

    fn text(&mut self, value: &str) -> Result<(), ContractError> {
        self.frame(value.as_bytes())
    }

    fn frame(&mut self, value: &[u8]) -> Result<(), ContractError> {
        if value.len() > MAX_TEXT_BYTES {
            return Err(ContractError::LengthOverflow);
        }
        let length = u64::try_from(value.len()).map_err(|_| ContractError::LengthOverflow)?;
        let required = 8_usize
            .checked_add(value.len())
            .and_then(|size| self.bytes.len().checked_add(size))
            .ok_or(ContractError::LengthOverflow)?;
        self.bytes
            .try_reserve(required.saturating_sub(self.bytes.len()))
            .map_err(|_| ContractError::LengthOverflow)?;
        self.bytes.extend_from_slice(&length.to_be_bytes());
        self.bytes.extend_from_slice(value);
        Ok(())
    }
}
