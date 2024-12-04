//! ## Features
#![doc = document_features::document_features!()]

pub mod core;

// Used by generated code. Not public API.
#[doc(hidden)]
#[path = "private/mod.rs"]
pub(crate) mod __private;

// Preformed _TEMPORARY_ structures
schema_salad_macro::define_type! {
    #[doc = "Matches constant value `array`."]
    #[derive(Clone, Copy, Debug)]
    #[salad(as_str = "array")]
    pub struct TypeArray;
}

schema_salad_macro::define_type! {
    #[doc = "Matches constant value `enum`."]
    #[derive(Clone, Copy, Debug)]
    #[salad(as_str = "enum")]
    pub struct TypeEnum;
}

schema_salad_macro::define_type! {
    #[doc = "Matches constant value `record`."]
    #[derive(Clone, Copy, Debug)]
    #[salad(as_str = "record")]
    pub struct TypeRecord;
}

// Generated code
