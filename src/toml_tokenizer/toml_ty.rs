#[derive(Debug, Clone)]
pub struct TomlItems {
    pub items: Vec<String>,
}

impl TomlItems {
    pub fn new(items: Vec<String>) -> TomlItems {
        TomlItems { items }
    }
}

impl PartialEq for TomlItems {
    fn eq(&self, other: &TomlItems) -> bool {
        self.items == other.items
    }
}

#[derive(Debug, Clone)]
pub struct TomlHeader {
    pub extended: bool,
    pub inner: String,
    pub seg: Vec<String>,
}

impl PartialEq for TomlHeader {
    fn eq(&self, other: &TomlHeader) -> bool {
        self.inner == other.inner
    }
}

#[derive(Debug, Clone,)]
pub struct TomlTable {
    pub header: TomlHeader,
    pub items: TomlItems,
}

impl TomlTable {
    pub fn sort_items(&mut self) {
        self.items.items.sort_unstable()
    }
}

impl PartialEq for TomlTable {
    fn eq(&self, other: &TomlTable) -> bool {
        self.header == other.header && self.items == other.items
    }
}

impl Eq for TomlTable {}

impl PartialOrd for TomlTable  {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TomlTable {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.header.inner.cmp(&other.header.inner)
    }
}