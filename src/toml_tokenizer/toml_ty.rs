
#[derive(Debug, Clone)]
pub struct TomlItems {
    pub items: Vec<String>,
}

impl TomlItems {
    pub fn new(items: Vec<String>) -> TomlItems {
        TomlItems {
            items,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TomlHeader {
    pub extended: bool,
    pub inner: String,
    pub seg: Vec<String>,
}


#[derive(Debug, Clone)]
pub struct TomlTable {
    pub header: TomlHeader,
    pub items: TomlItems,
}

impl TomlTable {

    pub fn sort_items(&mut self) {
        self.items.items.sort_unstable()
    }
}
