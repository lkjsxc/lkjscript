use super::{semantic_tags::*, SemanticContractError as E, *};

const DOMAIN: &[u8] = b"lkjscript.semantic-memory-contract\0canonical-platform-contract";
const TYPE_DOMAIN: &[u8] = b"lkjscript.semantic-type-closure\0canonical-platform-contract";

pub fn semantic_contract_hash(value: &SemanticDescriptor) -> Result<[u8; 32], E> {
    Ok(crate::sha256(&canonical_semantic_descriptor(value)?))
}

pub fn semantic_type_closure_hash(value: &SemanticDescriptor) -> Result<[u8; 32], E> {
    let descriptor = canonical_semantic_descriptor(value)?;
    let mut bytes = Vec::with_capacity(TYPE_DOMAIN.len() + 8 + descriptor.len());
    bytes.extend_from_slice(TYPE_DOMAIN);
    bytes.extend_from_slice(
        &u64::try_from(descriptor.len())
            .map_err(|_| E("semantic type closure length overflow"))?
            .to_be_bytes(),
    );
    bytes.extend_from_slice(&descriptor);
    Ok(crate::sha256(&bytes))
}

pub fn canonical_semantic_descriptor(value: &SemanticDescriptor) -> Result<Vec<u8>, E> {
    validate_semantic_descriptor(value)?;
    let mut out = Encoder(Vec::new());
    out.bytes(DOMAIN)?;
    out.ty(&value.root)?;
    out.len(value.declarations.len())?;
    for declaration in &value.declarations {
        out.declaration(declaration)?;
    }
    if out.0.len() > MAX_SEMANTIC_DESCRIPTOR_BYTES {
        return Err(E("semantic descriptor byte limit exceeded"));
    }
    Ok(out.0)
}

struct Encoder(Vec<u8>);
impl Encoder {
    fn byte(&mut self, value: u8) {
        self.0.push(value);
    }
    fn bool(&mut self, value: bool) {
        self.byte(u8::from(value));
    }
    fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_be_bytes());
    }
    fn len(&mut self, value: usize) -> Result<(), E> {
        self.0.extend_from_slice(
            &u64::try_from(value)
                .map_err(|_| E("semantic length overflow"))?
                .to_be_bytes(),
        );
        Ok(())
    }
    fn bytes(&mut self, value: &[u8]) -> Result<(), E> {
        self.len(value.len())?;
        self.0.extend_from_slice(value);
        Ok(())
    }
    fn string(&mut self, value: &str) -> Result<(), E> {
        self.bytes(value.as_bytes())
    }
    fn names(&mut self, values: &[String]) -> Result<(), E> {
        self.len(values.len())?;
        for value in values {
            self.string(value)?;
        }
        Ok(())
    }
    fn ty(&mut self, value: &SemanticType) -> Result<(), E> {
        match value {
            SemanticType::Primitive(kind) => {
                self.byte(0);
                self.byte(primitive(*kind));
            }
            SemanticType::Capability(kind) => {
                self.byte(1);
                self.byte(capability(*kind));
            }
            SemanticType::Resource(kind) => {
                self.byte(2);
                self.byte(resource(*kind));
            }
            SemanticType::Product(id) => {
                self.byte(3);
                self.bytes(id)?;
            }
            SemanticType::Enum {
                identity,
                arguments,
            } => {
                self.byte(4);
                self.bytes(identity)?;
                self.len(arguments.len())?;
                for ty in arguments {
                    self.ty(ty)?;
                }
            }
            SemanticType::Parameter(name) => {
                self.byte(5);
                self.string(name)?;
            }
            SemanticType::List(ty) => {
                self.byte(6);
                self.ty(ty)?;
            }
            SemanticType::Function { parameters, result } => {
                self.byte(7);
                self.len(parameters.len())?;
                for ty in parameters {
                    self.ty(ty)?;
                }
                self.ty(result)?;
            }
            SemanticType::ForAll { parameters, body } => {
                self.byte(8);
                self.names(parameters)?;
                self.ty(body)?;
            }
        }
        Ok(())
    }
    fn declaration(&mut self, value: &SemanticDeclaration) -> Result<(), E> {
        match value {
            SemanticDeclaration::Product(item) => {
                self.byte(0);
                self.bytes(&item.identity)?;
                self.len(item.fields.len())?;
                for field in &item.fields {
                    self.bytes(&field.identity)?;
                    self.u16(field.source_order);
                    self.ty(&field.ty)?;
                }
            }
            SemanticDeclaration::Enum(item) => {
                self.byte(1);
                self.bytes(&item.identity)?;
                self.names(&item.type_parameters)?;
                self.len(item.variants.len())?;
                for variant in &item.variants {
                    self.bytes(&variant.identity)?;
                    self.u16(variant.source_order);
                    self.len(variant.fields.len())?;
                    for field in &variant.fields {
                        self.bytes(&field.identity)?;
                        self.u16(field.source_order);
                        self.bool(field.indirect);
                        self.ty(&field.ty)?;
                    }
                }
            }
        }
        Ok(())
    }
}
