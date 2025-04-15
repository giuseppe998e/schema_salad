// Core trait for all Schema Salad types
pub use salad_types::SaladType;

// Primitive data types in Schema Salad
pub use salad_types::{
    SaladBool, SaladDouble, SaladFloat, SaladInt, SaladLong, SaladPrimitive, SaladString,
};

// Dynamic typing support types for Schema Salad
pub use salad_types::{SaladAny, SaladObject};

// Common predefined Schema Salad types
pub use salad_types::common;

// Macro for defining Schema Salad types
pub use salad_macro::salad_type;

/// Used by generated code.
/// Not public API.
#[doc(hidden)]
pub mod __private {
    pub mod de {
        pub use salad_serde::de::{IntoDeserializeSeed, MapToListSeed, SeedData};
        pub use serde::__private::de::{Content, ContentDeserializer, ContentRefDeserializer};
        pub use serde::de::{
            Deserialize, DeserializeSeed, Deserializer, Error, Unexpected, Visitor,
        };
    }
}
