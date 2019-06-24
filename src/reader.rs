use std::collections::HashMap;
use std::io::Read;

pub struct TomlReader {
    inner: String,
    temp_c: String,
    slices: HashMap<String, Vec<String>>,
    eo_table: Vec<u8>,
}

impl<'s> TomlReader {
    pub fn new(s: &mut String) -> Self {
        // TODO cp
        // is_char_boundary panics in std lib string.rs if there is only
        // one eol at eof? hack fix
        if !s.ends_with("\n\n") {
            if s.ends_with("\n") {
                s.push('\n');
            } else {
                s.push_str("\n\n");
            }
        }
        TomlReader {
            inner: s.to_owned(),
            temp_c: s.to_owned(),
            slices: HashMap::default(),
            //TODO
            eo_table: b"\n\n".to_vec(),
        }
    }

    fn has_ended(&self, end: &'s str, buf: &[u8]) -> bool {
        if end == "\n[" {
            if (buf == self.eo_table.as_slice()) | (buf == end.as_bytes()) {
                return true;
            }
        }

        if (buf == self.eo_table.as_slice()) | (std::str::from_utf8(&buf).unwrap().contains(end)) {
            true
        } else {
            false
        }
    }

    fn unsorted_len(&mut self, after_header: usize, end: &'s str) -> Option<usize> {
        // TODO cross platform
        let mut window_buf = [0u8; 2];

        let mut curse = std::io::Cursor::new(self.temp_c.clone());
        curse.set_position(after_header as u64);

        let mut pos = after_header;
        loop {
            // read to eol number of bytes
            curse.read_exact(&mut window_buf).expect("read_exact");

            // if we find double eol or "[" return cursor position
            if self.has_ended(end, &window_buf) {
                return Some(pos);
            }
            // make sure we dont split and not read the right bytes in a row
            pos += window_buf.len() - 1;
            curse.set_position((pos - 1) as u64);
        }
    }

    fn slice_range(&mut self, pos: usize, end: &'s str, key: String) {
        let end_pos = self.unsorted_len(pos, end).expect("unsorted_len() failed");
        match self.slices.get(&key) {
            Some(_) => {
                let s = String::from(&self.temp_c[pos..end_pos]);
                self.slices.get_mut(&key).expect("get mut push").push(s);

                // cuts just read chunk out of toml
                let start = pos - key.len();
                self.temp_c.drain(start..end_pos);
            }
            None => {
                let s = String::from(&self.temp_c[pos..end_pos]);
                self.slices.insert(key.clone(), Vec::default());

                self.slices.get_mut(&key).expect("insert push").push(s);

                let start = pos - key.len();
                self.temp_c.drain(start..end_pos);
            }
        }
    }

    /// Slices all toml tables with seek_to as header
    /// also resets the temp_c
    pub fn slice_table(&mut self, seek_to: String, end: &'s str) -> std::io::Result<bool> {
        // refresh the string that we cut up so if we get
        // any items out of order they are still found
        self.temp_c = self.inner.clone();
        match self
            .temp_c
            .as_bytes()
            .windows(seek_to.len())
            .position(|win| win == seek_to.as_bytes())
        {
            Some(pos) => {
                let cursor_pos = pos + seek_to.len();

                self.slice_range(cursor_pos, end, seek_to);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Slices all expanded headers deps.foo
    pub fn slice_header(&mut self, seek_to: String, end: &'s str) -> std::io::Result<bool> {
        match self
            .temp_c
            .as_bytes()
            .windows(seek_to.len())
            .position(|win| win == seek_to.as_bytes())
        {
            Some(pos) => {
                let cursor_pos = pos + seek_to.len();

                self.slice_range(cursor_pos, end, seek_to);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Sorts and checks if sorted
    pub fn is_sorted(&mut self) -> bool {
        let unsorted = self.collect();
        let mut sorted = unsorted.clone();

        for (i, table) in sorted.iter_mut().enumerate() {
            table.sort_unstable();
            //println!("{:#?} = {:#?}", table, unsorted[i]);
            if table != &unsorted[i] {
                return false;
            }
        }
        true
    }

    fn collect(&mut self) -> Vec<Vec<&str>> {
        let mut all_of_key: Vec<Vec<&str>> = Vec::new();
        for (_, v) in self.slices.iter() {
            let mut to_flatten: Vec<Vec<&str>> = Vec::new();
            for s in v.iter() {
                let v = s.lines().filter(|l| *l != "").collect();

                to_flatten.push(v);
            }
            all_of_key.push(to_flatten.into_iter().flatten().collect());
        }
        all_of_key
    }

    #[allow(dead_code)]
    pub fn print(&self) {
        println!("{:#?}", self.slices);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static HEADERS: &'static [&'static str] = &[
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "workspace.members",
        "workspace.exclude",
    ];

    #[test]
    fn create_reader_fail() {
        let mut toml = r#"
        [dependencies]
        a="0"
        d="0"
        c="0""#
            .to_owned();

        let mut tr = TomlReader::new(&mut toml);
        for header in HEADERS.iter() {
            let full_header = format!("[{}]", header);
            tr.slice_table(full_header, "\n[")
                .expect("create_reader pass");

            if header.contains("dependencies") {
                while tr
                    .slice_header(format!("[{}.", header), "]")
                    .expect("create_reader pass")
                {}
            }
        }
        // fails
        assert!(!tr.is_sorted());
    }

    #[test]
    fn create_reader_pass() {
        let mut toml = r#"
        [dependencies]
        a="0"
        b="0"
        c="0""#
            .to_owned();

        let mut tr = TomlReader::new(&mut toml);
        for header in HEADERS.iter() {
            let full_header = format!("[{}]", header);
            tr.slice_table(full_header, "\n[")
                .expect("create_reader pass");

            if header.contains("dependencies") {
                while tr
                    .slice_header(format!("[{}.", header), "]")
                    .expect("create_reader pass")
                {}
            }
        }
        // pass
        assert!(tr.is_sorted());
    }

    #[test]
    fn create_reader_dup() {
        let mut toml = r#"
        [dependencies]
        a="0"
        d="0"
        a="0""#
            .to_owned();

        let mut tr = TomlReader::new(&mut toml);
        for header in HEADERS.iter() {
            let full_header = format!("[{}]", header);
            tr.slice_table(full_header, "\n[")
                .expect("create_reader pass");

            if header.contains("dependencies") {
                while tr
                    .slice_header(format!("[{}.", header), "]")
                    .expect("create_reader pass")
                {}
            }
        }
        // fails
        assert!(!tr.is_sorted());
    }

    #[test]
    fn complicated_deps_fail() {
        let mut toml = r#"
        [dependencies]
        a="0"
        b="0"
        c="0"

        [workspace.members]
        a="0"
        b="0"
        c="0"

        [build-dependencies.bar]
        version="10"

        [build-dependencies.foo]
        version="10"

        [dev-dependencies]
        a="0"
        f="0"
        c="0"

        "#
        .to_owned();

        let mut tr = TomlReader::new(&mut toml);
        for header in HEADERS.iter() {
            let full_header = format!("[{}]", header);
            tr.slice_table(full_header, "\n[")
                .expect("create_reader pass");

            if header.contains("dependencies") {
                while tr
                    .slice_header(format!("[{}.", header), "]")
                    .expect("create_reader pass")
                {}
            }
        }
        // fail
        assert!(!tr.is_sorted());
    }

    #[test]
    fn complicated_deps_pass() {
        let mut toml = r#"
        [dependencies]
        a="0"
        b="0"
        c="0"

        [build-dependencies]
        a="1"
        b="1"
        c="1"

        [build-dependencies.bar]
        version="10"

        [build-dependencies.foo]
        version="10"

        [dev-dependencies]
        a="0"
        b="0"
        c="0"

        [workspace.members]
        a="0"
        b="0"
        c="0"

        "#
        .to_owned();

        let mut tr = TomlReader::new(&mut toml);
        for header in HEADERS.iter() {
            let full_header = format!("[{}]", header);
            tr.slice_table(full_header, "\n[")
                .expect("create_reader pass");

            if header.contains("dependencies") {
                while tr
                    .slice_header(format!("[{}.", header), "]")
                    .expect("create_reader pass")
                {}
            }
        }
        // fail
        assert!(tr.is_sorted());
    }

    #[test]
    fn out_of_order() {
        let mut toml = r#"
        [dependencies]
        a="0"
        b="0"
        c="0"

        [build-dependencies.bar]
        version="10"

        [build-dependencies.foo]
        version="10"

        [build-dependencies]
        a="1"
        b="1"
        c="1"

        [dev-dependencies]
        a="0"
        b="0"
        c="0"

        [workspace.members]
        a="0"
        b="0"
        c="0"

        "#
        .to_owned();

        let mut tr = TomlReader::new(&mut toml);
        for header in HEADERS.iter() {
            let full_header = format!("[{}]", header);
            tr.slice_table(full_header, "\n[")
                .expect("create_reader pass");

            if header.contains("dependencies") {
                while tr
                    .slice_header(format!("[{}.", header), "]")
                    .expect("create_reader pass")
                {}
            }
        }
        // fail
        assert_eq!(tr.slices.len(), 5);
    }
}
