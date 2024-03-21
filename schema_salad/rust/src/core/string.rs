use std::{
    borrow::Borrow,
    fmt,
    hash::{self, Hash},
    ops::Deref,
};

use compact_str::CompactString;
use serde::{de, ser};

use crate::core::SaladType;

/// Unicode character sequence.
#[repr(transparent)]
#[derive(Clone, PartialOrd, Ord)]
pub struct StrValue(CompactString);

impl StrValue {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl SaladType for StrValue {}

impl From<&str> for StrValue {
    fn from(value: &str) -> Self {
        Self(CompactString::from(value))
    }
}

impl From<Box<str>> for StrValue {
    fn from(value: Box<str>) -> Self {
        Self(CompactString::from(value))
    }
}

impl From<String> for StrValue {
    fn from(value: String) -> Self {
        Self(CompactString::from(value))
    }
}

impl From<&String> for StrValue {
    fn from(value: &String) -> Self {
        Self(CompactString::from(value))
    }
}

impl AsRef<str> for StrValue {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Borrow<str> for StrValue {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl Deref for StrValue {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Hash for StrValue {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Eq for StrValue {}

impl PartialEq<StrValue> for StrValue {
    fn eq(&self, other: &StrValue) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<str> for StrValue {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<StrValue> for str {
    fn eq(&self, other: &StrValue) -> bool {
        other.0 == self
    }
}

impl<'a> PartialEq<&'a str> for StrValue {
    fn eq(&self, other: &&'a str) -> bool {
        self.0 == *other
    }
}

impl<'a> PartialEq<StrValue> for &'a str {
    fn eq(&self, other: &StrValue) -> bool {
        *self == other.0
    }
}

impl PartialEq<String> for StrValue {
    fn eq(&self, other: &String) -> bool {
        self.0 == other
    }
}

impl PartialEq<StrValue> for String {
    fn eq(&self, other: &StrValue) -> bool {
        other.0 == self
    }
}

impl<'a> PartialEq<&'a String> for StrValue {
    fn eq(&self, other: &&'a String) -> bool {
        self.0 == *other
    }
}

impl<'a> PartialEq<StrValue> for &'a String {
    fn eq(&self, other: &StrValue) -> bool {
        *self == &other.0
    }
}

impl fmt::Debug for StrValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for StrValue {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl ser::Serialize for StrValue {
    #[inline]
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        CompactString::serialize(&self.0, serializer)
    }
}

impl<'de> de::Deserialize<'de> for StrValue {
    #[inline]
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        CompactString::deserialize(deserializer).map(Self)
    }
}
