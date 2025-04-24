use compact_str::CompactString;

/// Extends the string type with helper methods for string manipulation.
pub trait ToSnakeCase {
    /// Converts a string to snake_case.
    ///
    /// This converts strings in various cases (like camelCase or PascalCase)
    /// to snake_case by inserting underscores before uppercase letters and
    /// converting all letters to lowercase.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate::ToSnakeCase;
    ///
    /// assert_eq!("HelloWorld".to_snake_case(), "hello_world");
    /// assert_eq!("camelCase".to_snake_case(), "camel_case");
    /// ```
    fn to_snake_case(&self) -> CompactString;
}

impl ToSnakeCase for &str {
    fn to_snake_case(&self) -> CompactString {
        let mut buf = CompactString::const_new(""); // Capacity = 24
        let mut chars = self.chars();

        if let Some(first_ch) = chars.next() {
            buf.push(first_ch.to_ascii_lowercase());

            for ch in chars {
                if ch.is_ascii_uppercase() {
                    buf.push('_');
                    buf.push(ch.to_ascii_lowercase());
                } else {
                    buf.push(ch);
                }
            }
        }

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        assert_eq!("".to_snake_case(), "");
    }

    #[test]
    fn test_lowercase() {
        assert_eq!("hello".to_snake_case(), "hello");
    }

    #[test]
    fn test_camel_case() {
        assert_eq!("helloWorld".to_snake_case(), "hello_world");
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!("HelloWorld".to_snake_case(), "hello_world");
    }

    #[test]
    fn test_complex_case() {
        assert_eq!("ThisIsATest".to_snake_case(), "this_is_a_test");
    }

    #[test]
    fn test_with_numbers() {
        assert_eq!("Hello123World".to_snake_case(), "hello123_world");
    }

    #[test]
    fn test_already_snake_case() {
        assert_eq!("hello_world".to_snake_case(), "hello_world");
    }
}
