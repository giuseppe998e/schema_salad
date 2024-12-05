use std::{collections::HashMap, fmt};

use compact_str::CompactString;
use fxhash::FxBuildHasher;

use super::SaladAny;
use crate::{SaladType, SaladTypeDowncastError};

/// A key-value map representing an untyped Schema Salad object.
///
/// `SaladObject` is a container that maps string keys to heterogeneous
/// values of type [`SaladAny`].
/// It provides a flexible way to represent arbitrary Schema Salad objects
/// before they are parsed into their specific types.
///
/// # Examples
/// ```
/// use salad_core::SaladAny;
/// use salad_core::any::SaladObject;
///
/// let obj = SaladObject::default();
/// // Given some entries in the object
/// obj.get(key); // Returns Option<&SaladAny>
///
/// // Downcast to a specific type
/// let typed_obj: Result<MyType, _> = obj.downcast();
/// ```
#[derive(Clone, Default)]
pub struct SaladObject {
    pub(super) map: HashMap<CompactString, SaladAny, FxBuildHasher>,
}

impl SaladObject {
    /// Retrieves a reference to a value in the object by its key.
    ///
    /// Returns an `Option` containing a reference to the value if found,
    /// or `None` if the key does not exist.
    pub fn get<S: AsRef<str>>(&self, key: S) -> Option<&SaladAny> {
        let key = key.as_ref();
        self.map.get(key)
    }

    /// Attempts to downcast to type `T` from a borrowed `SaladObject`.
    ///
    /// Returns a `Result` containing the downcasted value of type `T` if successful,
    /// or a `SaladTypeDowncastError` if the downcast fails.
    pub fn downcast<'de, T>(&'de self) -> Result<T, SaladTypeDowncastError>
    where
        T: SaladType + serde::de::Deserialize<'de>,
    {
        let deserializer = super::de::SaladObjectMapAccess::new(self);
        T::deserialize(deserializer)
    }

    /// Attempts to downcast from a consumed `SaladObject` to type `T`.
    ///
    /// Returns a `Result` containing the downcasted value of type `T` if successful,
    /// or a `SaladTypeDowncastError` if the downcast fails.
    #[inline]
    pub fn downcast_into<T>(self) -> Result<T, SaladTypeDowncastError>
    where
        for<'de> T: SaladType + serde::de::Deserialize<'de>,
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
