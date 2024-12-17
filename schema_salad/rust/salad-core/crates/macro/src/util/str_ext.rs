pub trait StrExt {
    fn to_snake_case(&self) -> String;
}

impl StrExt for String {
    fn to_snake_case(&self) -> String {
        self.as_str().to_snake_case()
    }
}

impl StrExt for &str {
    fn to_snake_case(&self) -> String {
        let mut buf = String::with_capacity(self.len() * 2);
        let mut iter = self.chars().peekable();

        while let Some(ch) = iter.next() {
            buf.push(ch.to_ascii_lowercase());

            if let Some(ch_next) = iter.peek() {
                if ch.is_ascii_lowercase() && ch_next.is_ascii_uppercase() {
                    buf.push('_')
                }
            }
        }

        buf
    }
}
