pub trait Parse<T> {
    type Item;
    type Error;
    /// Parse a valid toml str into to a toml token.
    /// 
    /// Item is the type returned and T is the input 
    /// # Examples
    /// 
    /// ```
    /// use toml_tokenizer::TomlTokenizer;
    /// 
    /// let toml = r#"[dependencies]
    /// a="0"
    /// f="0"
    /// c="0"
    /// 
    /// "#;
    /// 
    /// let mut tt = TomlTokenizer::parse(toml).unwrap();
    /// assert_eq!(tt.to_string(), toml);
    /// ```
    /// 
    fn parse(s: T) -> Result<Self::Item, Self::Error>;
}
