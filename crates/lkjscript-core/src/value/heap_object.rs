use super::model::Value;
use crate::ProductId;

#[derive(Debug, Clone, PartialEq)]
pub enum HeapObj {
    Int(i64),
    Float(f64),
    Str(String),
    Symbol(String),
    Pair {
        car: Value,
        cdr: Value,
    },
    Closure {
        proto: u32,
        captures: Vec<Value>,
    },
    Builtin(u16),
    Buf(Vec<u8>),
    Path(Vec<u8>),
    Product {
        product: ProductId,
        fields: Vec<Value>,
    },
    Enum {
        layout: crate::RuntimeLayoutId,
        physical_tag: u16,
        active_payload: Vec<Value>,
    },
}

impl HeapObj {
    pub fn trace(&self, mark: &mut dyn FnMut(Value)) {
        match self {
            HeapObj::Pair { car, cdr } => {
                mark(*car);
                mark(*cdr);
            }
            HeapObj::Closure { captures, .. } => {
                for c in captures {
                    mark(*c);
                }
            }
            HeapObj::Product { fields, .. } => {
                for field in fields {
                    mark(*field);
                }
            }
            HeapObj::Enum { active_payload, .. } => {
                for field in active_payload {
                    mark(*field);
                }
            }
            _ => {}
        }
    }
}
