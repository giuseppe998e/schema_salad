mod any;
pub mod common;
mod error;
pub mod primitive;
mod util;

pub use self::{
    any::{SaladAny, SaladObject},
    error::SaladTypeDowncastError,
};

/// A marker trait for Schema Salad data types.
///
/// This trait is implemented by all types that represent valid Schema Salad data,
/// including primitives (boolean, int, float, string), objects, and collections.
pub trait SaladType {}
