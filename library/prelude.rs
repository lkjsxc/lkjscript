//! The source prelude. These enums own their payloads and work with affine values.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Option<T> {
    None,
    Some(T),
}

impl<T> Option<T> {
    pub fn is_some(&self) -> bool {
        match self {
            Option::None => false,
            Option::Some(_) => true,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    pub fn unwrap(self) -> T {
        match self {
            Option::Some(value) => return value,
            Option::None => panic!("called Option::unwrap on None"),
        }
    }

    pub fn take(&mut self) -> Option<T> {
        std::mem::replace(self, Option::None)
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Option::Some(value) => return value,
            Option::None => return default,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E> {
    pub fn is_ok(&self) -> bool {
        match self {
            Result::Ok(_) => true,
            Result::Err(_) => false,
        }
    }

    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }

    pub fn unwrap(self) -> T {
        match self {
            Result::Ok(value) => return value,
            Result::Err(_) => panic!("called Result::unwrap on Err"),
        }
    }

    pub fn unwrap_err(self) -> E {
        match self {
            Result::Err(value) => return value,
            Result::Ok(_) => panic!("called Result::unwrap_err on Ok"),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Result::Ok(value) => return value,
            Result::Err(_) => return default,
        }
    }

    pub fn ok(self) -> Option<T> {
        match self {
            Result::Ok(value) => return Option::Some(value),
            Result::Err(_) => return Option::None,
        }
    }

    pub fn err(self) -> Option<E> {
        match self {
            Result::Err(value) => return Option::Some(value),
            Result::Ok(_) => return Option::None,
        }
    }
}
