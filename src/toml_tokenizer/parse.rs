pub trait Parse<T> {
    type Item;
    type Error;
    fn parse(s: T) -> Result<Self::Item, Self::Error>;
}
