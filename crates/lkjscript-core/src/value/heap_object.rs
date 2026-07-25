use super::model::Value;
use crate::chunk::ProductId;

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
    ResultOk(Value),
    ResultErr(Value),
    OptionSome(Value),
    Product {
        product: ProductId,
        fields: Vec<Value>,
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
            HeapObj::ResultOk(v) | HeapObj::ResultErr(v) | HeapObj::OptionSome(v) => mark(*v),
            HeapObj::Product { fields, .. } => {
                for field in fields {
                    mark(*field);
                }
            }
            _ => {}
        }
    }
}
