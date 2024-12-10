mod data;
mod list;
mod map;

use std::marker::PhantomData;

use salad_types::SaladType;
use serde::de;

use self::list::ListDeserializeSeed;
pub use self::{data::SeedData, map::MapDeserializeSeed};

/// TODO ...
pub trait IntoDeserializeSeed<'de, 'sd> {
    type DeserializeSeed: de::DeserializeSeed<'de, Value = Self>;

    /// TODO ...
    fn deserialize_seed(data: &'sd SeedData) -> Self::DeserializeSeed;
}

// ///////////////////////////////////////////////////////////////////////////// //

impl<'de, 'sd, T> IntoDeserializeSeed<'de, 'sd> for Box<[T]>
where
    T: SaladType + IntoDeserializeSeed<'de, 'sd>,
{
    type DeserializeSeed = ListDeserializeSeed<'sd, T>;

    #[inline]
    fn deserialize_seed(data: &'sd SeedData) -> Self::DeserializeSeed {
        ListDeserializeSeed {
            data,
            _phant: PhantomData,
        }
    }
}

macro_rules! impl_default_intoseed {
    ( $( $ty:path ),* $(,)? ) => {
        $(
            impl<'sd> IntoDeserializeSeed<'_, 'sd> for $ty {
                type DeserializeSeed = std::marker::PhantomData<Self>;

                #[inline]
                fn deserialize_seed(_: &'sd SeedData) -> Self::DeserializeSeed {
                    std::marker::PhantomData
                }
            }
        )*
    };
}

impl_default_intoseed! {
    // Any & Object
    salad_types::SaladAny,
    salad_types::SaladObject,

    // Primitives
    salad_types::primitive::SaladBool,
    salad_types::primitive::SaladInt,
    salad_types::primitive::SaladLong,
    salad_types::primitive::SaladFloat,
    salad_types::primitive::SaladDouble,
    salad_types::primitive::SaladString,
    salad_types::primitive::SaladPrimitive,

    // Common
    salad_types::common::ArrayName,
    salad_types::common::EnumName,
    salad_types::common::RecordName,
    salad_types::common::PrimitiveType,
}
