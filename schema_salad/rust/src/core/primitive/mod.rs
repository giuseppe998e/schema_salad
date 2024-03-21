use crate::core::SaladType;

/// A binary value.
pub type Bool = bool;

impl SaladType for Bool {}

/// 32-bit signed integer.
pub type Int = i32;

impl SaladType for Int {}

/// 64-bit signed integer.
pub type Long = i64;

impl SaladType for Long {}

/// Single precision (32-bit) IEEE 754 floating-point number.
pub type Float = f32;

impl SaladType for Float {}

/// Double precision (64-bit) IEEE 754 floating-point number.
pub type Double = f64;

impl SaladType for Double {}
