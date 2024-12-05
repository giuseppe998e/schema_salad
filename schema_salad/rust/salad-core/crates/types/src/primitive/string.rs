use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    str::FromStr,
};

use compact_str::CompactString;
use serde::{de, ser};

use crate::SaladType;

/// Unicode character sequence, a string.
#[repr(transparent)]
#[derive(Clone, Default)]
pub struct SaladString(CompactString);

impl SaladString {
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl SaladType for SaladString {}

impl From<String> for SaladString {
    fn from(value: String) -> Self {
        Self(CompactString::from_string_buffer(value))
    }
}

impl<'a> From<&'a String> for SaladString {
    fn from(value: &'a String) -> Self {
        Self(CompactString::new(value))
    }
}

impl From<SaladString> for String {
    fn from(value: SaladString) -> Self {
        String::from(value.0)
    }
}

impl<'a> From<&'a str> for SaladString {
    fn from(value: &'a str) -> Self {
        Self(CompactString::new(value))
    }
}

impl From<Box<str>> for SaladString {
    fn from(value: Box<str>) -> Self {
        Self(CompactString::from(value))
    }
}

impl From<SaladString> for Box<str> {
    fn from(value: SaladString) -> Self {
        Box::<str>::from(value.0)
    }
}

impl FromStr for SaladString {
    type Err = std::convert::Infallible;

    #[inline]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(value))
    }
}

impl FromIterator<char> for SaladString {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        Self(CompactString::from_iter(iter))
    }
}

impl<'a> FromIterator<&'a char> for SaladString {
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> Self {
        Self(CompactString::from_iter(iter))
    }
}

impl Extend<char> for SaladString {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl<'a> Extend<&'a char> for SaladString {
    fn extend<T: IntoIterator<Item = &'a char>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl AsRef<str> for SaladString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for SaladString {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<str> for SaladString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Eq for SaladString {}

impl<T: AsRef<str> + ?Sized> PartialEq<T> for SaladString {
    #[inline]
    fn eq(&self, other: &T) -> bool {
        self.0.as_str() == other.as_ref()
    }
}

impl PartialEq<SaladString> for &SaladString {
    fn eq(&self, other: &SaladString) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl PartialEq<SaladString> for String {
    fn eq(&self, other: &SaladString) -> bool {
        self.as_str() == other.0.as_str()
    }
}

impl<'a> PartialEq<&'a SaladString> for String {
    fn eq(&self, other: &&'a SaladString) -> bool {
        self.as_str() == other.0.as_str()
    }
}

impl PartialEq<SaladString> for &String {
    fn eq(&self, other: &SaladString) -> bool {
        self.as_str() == other.0.as_str()
    }
}

impl PartialEq<String> for &SaladString {
    fn eq(&self, other: &String) -> bool {
        self.0.as_str() == other.as_str()
    }
}

impl PartialEq<SaladString> for str {
    fn eq(&self, other: &SaladString) -> bool {
        self == other.0.as_str()
    }
}

impl<'a> PartialEq<&'a SaladString> for str {
    fn eq(&self, other: &&'a SaladString) -> bool {
        self == other.0.as_str()
    }
}

impl PartialEq<SaladString> for &str {
    fn eq(&self, other: &SaladString) -> bool {
        *self == other.0.as_str()
    }
}

impl PartialEq<SaladString> for &&str {
    fn eq(&self, other: &SaladString) -> bool {
        **self == other.0.as_str()
    }
}

impl Ord for SaladString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for SaladString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for SaladString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Deref for SaladString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl fmt::Debug for SaladString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for SaladString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl ser::Serialize for SaladString {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self)
    }
}

impl<'de> de::Deserialize<'de> for SaladString {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SaladStringVisitor;

        impl de::Visitor<'_> for SaladStringVisitor {
            type Value = SaladString;

            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str("a string")
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(SaladString(CompactString::from_string_buffer(v)))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(SaladString(CompactString::new(v)))
            }

            fn visit_borrowed_str<E: de::Error>(self, v: &'_ str) -> Result<Self::Value, E> {
                Ok(SaladString(CompactString::new(v)))
            }

            fn visit_byte_buf<E: de::Error>(self, v: Vec<u8>) -> Result<Self::Value, E> {
                String::from_utf8(v)
                    .map(|v| SaladString(CompactString::from_string_buffer(v)))
                    .map_err(|e| {
                        de::Error::invalid_value(de::Unexpected::Bytes(&e.into_bytes()), &self)
                    })
            }

            fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                core::str::from_utf8(v)
                    .map(|v| SaladString(CompactString::new(v)))
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(v), &self))
            }

            fn visit_borrowed_bytes<E: de::Error>(self, v: &'_ [u8]) -> Result<Self::Value, E> {
                core::str::from_utf8(v)
                    .map(|v| SaladString(CompactString::new(v)))
                    .map_err(|_| de::Error::invalid_value(de::Unexpected::Bytes(v), &self))
            }
        }

        deserializer.deserialize_str(SaladStringVisitor)
    }
}
