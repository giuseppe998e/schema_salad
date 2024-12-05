mod string;

use std::fmt;

pub use self::string::SaladString;
use crate::{SaladType, SaladTypeDowncastError};

/// A binary value.
pub type SaladBool = bool;
impl SaladType for SaladBool {}

/// 32-bit signed integer.
pub type SaladInt = i32;
impl SaladType for SaladInt {}

/// 64-bit signed integer.
pub type SaladLong = i64;
impl SaladType for SaladLong {}

/// Single precision (32-bit) IEEE 754 floating-point number.
pub type SaladFloat = f32;
impl SaladType for SaladFloat {}

/// Double precision (64-bit) IEEE 754 floating-point number.
pub type SaladDouble = f64;
impl SaladType for SaladDouble {}

/// Schema Salad primitives, except `null`.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SaladPrimitive {
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
    /// Unicode character sequence, a string.
    String(SaladString),
}

impl SaladType for SaladPrimitive {}

crate::util::impl_from_traits! {
    (SaladPrimitive, SaladTypeDowncastError)

    Bool => SaladBool,
    Int => SaladInt,
    Long => SaladLong,
    Float => SaladFloat,
    Double => SaladDouble,
    String => SaladString,
}

impl fmt::Display for SaladPrimitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(b) => fmt::Display::fmt(b, f),
            Self::Int(i) => fmt::Display::fmt(i, f),
            Self::Long(l) => fmt::Display::fmt(l, f),
            Self::Float(fl) => fmt::Display::fmt(fl, f),
            Self::Double(d) => fmt::Display::fmt(d, f),
            Self::String(s) => fmt::Display::fmt(s, f),
        }
    }
}
