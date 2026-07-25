use std::fmt;

use super::BudgetAuthority;

pub const MAX_BUDGET_PATH_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BudgetPath {
    entries: [BudgetAuthority; MAX_BUDGET_PATH_DEPTH],
    len: u8,
}

impl BudgetPath {
    pub const fn root(authority: BudgetAuthority) -> Self {
        Self {
            entries: [authority; MAX_BUDGET_PATH_DEPTH],
            len: 1,
        }
    }

    pub const fn len(self) -> usize {
        self.len as usize
    }

    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    pub const fn authority(self) -> Option<BudgetAuthority> {
        if self.len == 0 {
            None
        } else {
            Some(self.entries[self.len as usize - 1])
        }
    }

    pub fn entries(&self) -> &[BudgetAuthority] {
        &self.entries[..self.len()]
    }

    pub(crate) const fn empty() -> Self {
        Self {
            entries: [BudgetAuthority::CompileRequest; MAX_BUDGET_PATH_DEPTH],
            len: 0,
        }
    }

    pub(crate) fn pushed(self, authority: BudgetAuthority) -> Option<Self> {
        if self.len() == MAX_BUDGET_PATH_DEPTH {
            return None;
        }
        let mut path = self;
        path.entries[path.len()] = authority;
        path.len += 1;
        Some(path)
    }
}

impl fmt::Display for BudgetPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return formatter.write_str("<missing>");
        }
        for (index, authority) in self.entries().iter().enumerate() {
            if index != 0 {
                formatter.write_str("/")?;
            }
            formatter.write_str(authority.as_str())?;
        }
        Ok(())
    }
}
