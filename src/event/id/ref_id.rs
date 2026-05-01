use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use crate::event::id::ID;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefID<'r> {
    pub inner: RefIDInner<'r>,
}
impl std::fmt::Display for RefID<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            RefIDInner::Int(v) => write!(f, "{}", **v),
            RefIDInner::String(v) => write!(f, "{}", *v),
        }
    }
}

impl RefID<'_> {
    pub fn new<'s, T: ParseRefId + ?Sized>(inner: &'s T) -> RefID<'s> {
        let inner = inner.as_ref_id();
        RefID { inner }
    }
}

impl PartialEq<super::ID> for RefID<'_> {
    fn eq(&self, other: &super::ID) -> bool {
        self.inner == other.inner
    }
}

impl PartialOrd for RefID<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for RefID<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl Ord for RefID<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl Ord for RefIDInner<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (RefIDInner::Int(a), RefIDInner::Int(b)) => a.cmp(b),
            (RefIDInner::String(a), RefIDInner::String(b)) => a.cmp(b),
            (RefIDInner::Int(_), RefIDInner::String(_)) => Ordering::Less,
            (RefIDInner::String(_), RefIDInner::Int(_)) => Ordering::Greater,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RefIDInner<'r> {
    Int(&'r i64),
    String(&'r str),
}

impl PartialEq<super::IDInner> for RefIDInner<'_> {
    fn eq(&self, other: &super::IDInner) -> bool {
        match (self, other) {
            (RefIDInner::Int(a), super::IDInner::Int(b)) => *a == b,
            (RefIDInner::String(a), super::IDInner::String(b)) => *a == b.as_str(),
            _ => false,
        }
    }
}

impl PartialEq for RefIDInner<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RefIDInner::Int(a), RefIDInner::Int(b)) => *a == *b,
            (RefIDInner::String(a), RefIDInner::String(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for RefIDInner<'_> {
}

impl PartialOrd for RefIDInner<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for RefIDInner<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            RefIDInner::Int(v) => {
                0u8.hash(state);
                v.hash(state);
            }
            RefIDInner::String(v) => {
                1u8.hash(state);
                v.hash(state);
            }
        }
    }
}

pub trait ParseRefId {
    fn as_ref_id<'s>(&'s self) -> RefIDInner<'s>;
}
impl ParseRefId for ID {
    fn as_ref_id<'s>(&'s self) -> RefIDInner<'s> {
        match &self.inner {
            super::IDInner::Int(v) => v.as_ref_id(),
            super::IDInner::String(v) => v.as_ref_id(),
        }
    }
}
impl ParseRefId for i64 {
    fn as_ref_id<'s>(&'s self) -> RefIDInner<'s> {
        RefIDInner::Int(self)
    }
}
impl ParseRefId for String {
    fn as_ref_id<'s>(&'s self) -> RefIDInner<'s> {
        RefIDInner::String(self)
    }
}

impl ParseRefId for str {
    fn as_ref_id<'s>(&'s self) -> RefIDInner<'s> {
        RefIDInner::String(self)
    }
}

impl<T: ParseRefId> ParseRefId for &T {
    fn as_ref_id(&self) -> RefIDInner<'_> {
        (*self).as_ref_id()
    }
}

impl From<RefID<'_>> for toml::Value {
    fn from(value: RefID) -> Self {
        match value.inner {
            RefIDInner::Int(v) => (*v).into(),
            RefIDInner::String(v) => v.into(),
        }
    }
}
impl From<RefID<'_>> for toml_edit::Value {
    fn from(value: RefID) -> Self {
        match value.inner {
            RefIDInner::Int(v) => (*v).into(),
            RefIDInner::String(v) => v.into(),
        }
    }
}
impl From<RefID<'_>> for serde_json::Value {
    fn from(value: RefID) -> Self {
        match value.inner {
            RefIDInner::Int(v) => (*v).into(),
            RefIDInner::String(v) => v.into(),
        }
    }
}
