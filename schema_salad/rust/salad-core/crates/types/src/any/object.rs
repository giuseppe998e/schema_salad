use std::{borrow::Borrow, collections::HashMap, fmt};

use compact_str::CompactString;
use fxhash::FxBuildHasher;
use serde::{
    de,
    ser::{self, SerializeMap as _},
};

use super::SaladAny;
use crate::{SaladType, SaladDowncastError};

/// A key-value map representing an untyped Schema Salad object.
///
/// `SaladObject` is a container that maps string keys to heterogeneous
/// values of type [`SaladAny`].
/// It provides a flexible way to represent arbitrary Schema Salad objects
/// before they are parsed into their specific types.
///
/// # Examples
/// ```ignore
/// use salad_core::SaladAny;
/// use salad_core::any::SaladObject;
///
/// let obj = SaladObject::default();
/// // Given some entries in the object
/// obj.get(key); // Returns Option<&SaladAny>
///
/// // Downcast to a specific type
/// let typed_obj: Result<SomeSaladType, _> = obj.downcast();
/// ```
#[derive(Clone, Default, PartialEq)]
pub struct SaladObject {
    pub(super) map: HashMap<CompactString, SaladAny, FxBuildHasher>,
}

impl SaladObject {
    /// Retrieves a reference to a value in the object by its key.
    ///
    /// Returns an `Option` containing a reference to the value if found,
    /// or `None` if the key does not exist.
    pub fn get<S: Borrow<str> + ?Sized>(&self, key: &S) -> Option<&SaladAny> {
        let key = key.borrow();
        self.map.get(key)
    }

    /// Attempts to downcast to type `T` from a borrowed `SaladObject`.
    ///
    /// Returns a `Result` containing the downcasted value of type `T` if successful,
    /// or a `SaladTypeDowncastError` if the downcast fails.
    pub fn downcast<'de, T>(&'de self) -> Result<T, SaladDowncastError>
    where
        T: SaladType + de::Deserialize<'de>,
    {
        let deserializer = super::deser::SaladObjectMapAccess::new(self);
        T::deserialize(deserializer)
    }

    /// Attempts to downcast from a consumed `SaladObject` to type `T`.
    ///
    /// Returns a `Result` containing the downcasted value of type `T` if successful,
    /// or a `SaladTypeDowncastError` if the downcast fails.
    #[inline]
    pub fn downcast_into<T>(self) -> Result<T, SaladDowncastError>
    where
        for<'de> T: SaladType + de::Deserialize<'de>,
    {
        Self::downcast(&self)
    }
}

impl SaladType for SaladObject {}

impl fmt::Debug for SaladObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("SaladObject");
        for (k, v) in self.map.iter() {
            debug_struct.field(k.as_str(), v);
        }
        debug_struct.finish()
    }
}

impl ser::Serialize for SaladObject {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map_serializer = serializer.serialize_map(Some(self.map.len()))?;
        self.map
            .iter()
            .try_for_each(|(k, v)| map_serializer.serialize_entry(k.as_str(), v))?;
        map_serializer.end()
    }
}

impl<'de> de::Deserialize<'de> for SaladObject {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SaladObjectVisitor;

        impl<'de> de::Visitor<'de> for SaladObjectVisitor {
            type Value = SaladObject;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a generic key-value object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let capacity = map.size_hint().unwrap_or(0);
                let mut this = SaladObject {
                    map: HashMap::with_capacity_and_hasher(capacity, FxBuildHasher::default()),
                };

                while let Some(key) = map.next_key::<CompactString>()? {
                    if this.map.contains_key(&key) {
                        return Err(de::Error::custom(format_args!(
                            "duplicate field `{}`",
                            &key
                        )));
                    }

                    let value = map.next_value::<SaladAny>()?;
                    this.map.insert(key, value);
                }

                Ok(this)
            }
        }

        deserializer.deserialize_map(SaladObjectVisitor)
    }
}