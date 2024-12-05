mod de;
mod object;

pub use self::object::SaladObject;
use crate::{
    primitive::{SaladBool, SaladDouble, SaladFloat, SaladInt, SaladLong, SaladString},
    SaladType, SaladTypeDowncastError,
};

/// The `SaladAny` type validates for any non-null value.
#[derive(Debug, Clone)]
pub enum SaladAny {
    /// A binary value.
    Bool(SaladBool),
    /// 32-bit signed integer.
    Int(SaladInt),
    /// 64-bit signed integer.
    Long(SaladLong),
    /// Single precision (32-bit) IEEE 754 floating-point number.
    Float(SaladFloat),
    /// Double precision (64-bit) IEEE 754 floating-point number.
    Double(SaladDouble),
    /// Unicode character sequence.
    String(SaladString),
    /// Unknown object.
    Object(SaladObject),
    /// List of any values.
    List(Box<[SaladAny]>),
}

impl SaladAny {
    /// Attempts to downcast to type `T` from a borrowed `SaladAny`.
    /// N.B. When downcasting to a primitive type, consider using the
    /// `TryFrom::try_from` method.
    ///
    /// Returns a `Result` containing the downcasted value of type `T` if successful,
    /// or a `SaladTypeDowncastError` if the downcast fails.
    pub fn downcast<'de, T>(&'de self) -> Result<T, SaladTypeDowncastError>
    where
        T: SaladType + serde::de::Deserialize<'de>,
    {
        let deserializer = self::de::SaladAnyDeserializer(self);
        T::deserialize(deserializer)
    }

    /// Attempts to downcast from a consumed `SaladAny` to type `T`.
    /// N.B. When downcasting to a primitive type, consider using the
    /// `TryFrom::try_from` method.
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

impl SaladType for SaladAny {}

crate::util::impl_from_traits! {
    (SaladAny, SaladTypeDowncastError)

    Bool => SaladBool,
    Int => SaladInt,
    Long => SaladLong,
    Float => SaladFloat,
    Double => SaladDouble,
    String => SaladString,
}
