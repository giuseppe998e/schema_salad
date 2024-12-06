use std::{error::Error, fmt};

use serde::de;

/// Error type used when a type downcast operation fails.
///
/// This error occurs when attempting to convert a value from a more general type
/// to a more specific type (_downcast_) and the conversion is not possible. For example,
/// when trying to convert a `SaladAny` containing a string value into a numeric type.
///
/// The error may optionally contain a cause message explaining why the downcast failed.
///
/// # Examples
/// ```ignore
/// use salad_core::SaladAny;
/// use salad_core::primitive::SaladInt;
///
/// let any = SaladAny::String("hello".into());
/// let result = SaladInt::try_from(any);
/// assert!(result.is_err()); // Error, cannot downcast SaladString to SaladInt
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaladTypeDowncastError {
    cause: Option<String>,
}

impl SaladTypeDowncastError {
    pub const fn new() -> Self {
        Self { cause: None }
    }
}

impl fmt::Display for SaladTypeDowncastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Some(cause) => write!(f, "could not downcast, {cause}"),
            None => f.write_str("could not downcast to desired type"),
        }
    }
}

impl Error for SaladTypeDowncastError {}

impl de::Error for SaladTypeDowncastError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self {
            cause: Some(msg.to_string()),
        }
    }
}
