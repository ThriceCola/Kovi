pub mod ref_id;

#[cfg(test)]
pub mod test;

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ID {
    pub inner: IDInner,
}
impl std::fmt::Display for ID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            IDInner::Int(v) => write!(f, "{}", v),
            IDInner::String(v) => write!(f, "{}", v),
        }
    }
}
impl ID {
    pub fn new<T: ParseUserId>(inner: T) -> ID {
        let inner = inner.into_id_inner();
        ID { inner }
    }
}

impl PartialEq<ref_id::RefID<'_>> for ID {
    fn eq(&self, other: &ref_id::RefID<'_>) -> bool {
        self.inner == other.inner
    }
}

impl PartialOrd for ID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ID {
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl Hash for ID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IDInner {
    Int(i64),
    String(String),
}

impl PartialEq<ref_id::RefIDInner<'_>> for IDInner {
    fn eq(&self, other: &ref_id::RefIDInner<'_>) -> bool {
        match (self, other) {
            (IDInner::Int(a), ref_id::RefIDInner::Int(b)) => a == *b,
            (IDInner::String(a), ref_id::RefIDInner::String(b)) => a == b,

            _ => false,
        }
    }
}

impl PartialOrd for IDInner {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IDInner {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (IDInner::Int(a), IDInner::Int(b)) => a.cmp(b),
            (IDInner::String(a), IDInner::String(b)) => a.cmp(b),
            (IDInner::Int(_), IDInner::String(_)) => Ordering::Less,
            (IDInner::String(_), IDInner::Int(_)) => Ordering::Greater,
        }
    }
}

impl Hash for IDInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            IDInner::Int(v) => {
                0u8.hash(state);
                v.hash(state);
            }
            IDInner::String(v) => {
                1u8.hash(state);
                v.hash(state);
            }
        }
    }
}

pub trait ParseUserId: Sized {
    fn into_id_inner(self) -> IDInner;
}

impl ParseUserId for i64 {
    fn into_id_inner(self) -> IDInner {
        IDInner::Int(self)
    }
}
impl ParseUserId for String {
    fn into_id_inner(self) -> IDInner {
        IDInner::String(self)
    }
}
impl ParseUserId for &str {
    fn into_id_inner(self) -> IDInner {
        IDInner::String(self.to_string())
    }
}

// Serde implementations:
impl Serialize for IDInner {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            IDInner::Int(v) => serializer.serialize_i64(*v),
            IDInner::String(s) => serializer.serialize_str(s),
        }
    }
}

struct IDInnerVisitor;

impl<'de> Visitor<'de> for IDInnerVisitor {
    type Value = IDInner;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an integer or a string for ID")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(IDInner::Int(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(IDInner::String(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(IDInner::String(v))
    }
}

impl<'de> Deserialize<'de> for IDInner {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(IDInnerVisitor)
    }
}

impl Serialize for ID {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let inner = IDInner::deserialize(deserializer)?;
        Ok(ID { inner })
    }
}

impl From<ID> for toml::Value {
    fn from(value: ID) -> Self {
        match value.inner {
            IDInner::Int(v) => v.into(),
            IDInner::String(v) => v.into(),
        }
    }
}
impl From<ID> for toml_edit::Value {
    fn from(value: ID) -> Self {
        match value.inner {
            IDInner::Int(v) => v.into(),
            IDInner::String(v) => v.into(),
        }
    }
}
impl From<ID> for serde_json::Value {
    fn from(value: ID) -> Self {
        match value.inner {
            IDInner::Int(v) => v.into(),
            IDInner::String(v) => v.into(),
        }
    }
}
