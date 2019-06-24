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
            if (buf == self.eo_table.as_slice())
            | (buf == end.as_bytes())
            {
                return true
            }
        }

        if (buf == self.eo_table.as_slice())
        | (std::str::from_utf8(&buf).unwrap().contains(end))
        {
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
                return Some(pos)
            }
            // make sure we dont split and not read the right bytes in a row
            pos += window_buf.len() - 1;
            curse.set_position((pos - 1) as u64);
        }
    }

    fn slice_range(
        &mut self,
        pos: usize,
        end: &'s str,
        key: String
    ) {
        let end_pos = self.unsorted_len(pos, end).expect("unsorted_len() failed");
        match self.slices.get(&key) {
            Some(_) => {
                let s = String::from(&self.temp_c[pos..end_pos]);
                self.slices.get_mut(&key)
                    .expect("get mut push").push(s);
                self.temp_c = self.temp_c[end_pos..].to_owned();
            },
            None => {
                let s = String::from(&self.temp_c[pos..end_pos]);
                self.slices.insert(key.clone(), Vec::default());
                self.slices.get_mut(&key)
                    .expect("insert push").push(s);
                self.temp_c = self.temp_c[end_pos..].to_owned();
            },
        }

    }

    pub fn slice_table(
        &mut self,
        seek_to: String,
        end: &'s str,
    ) -> std::io::Result<bool> {
        // refresh the string that we cut up so if we get
        // any items out of order they are still found
        self.temp_c = self.inner.clone();
        match self.temp_c
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

    pub fn slice_header(
        &mut self,
        seek_to: String,
        end: &'s str,
    ) -> std::io::Result<bool> {
        match self.temp_c
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

    pub fn is_sorted (&mut self) -> bool {
        let unsorted = self.collect();
        let mut sorted = unsorted.clone();
        
        //println!("{:#?}", self.slices);
        for (i, table) in sorted.iter_mut().enumerate() {
            table.sort_unstable();
            if table != &unsorted[i] {
                return false;
            }
        }
        println!("{:#?}", sorted);
        true
    }
    
    pub fn collect(&mut self) -> Vec<Vec<&str>> {
        let mut all_of_key: Vec<Vec<&str>> = Vec::new();
        for (_, v) in self.slices.iter() {
            let mut to_flatten: Vec<Vec<&str>> = Vec::new();
            for s in v.iter() {
                let v = s.lines()
                    .filter(|l| *l != "")
                    .collect();
                
                to_flatten.push(v);
            }
            all_of_key.push(to_flatten.into_iter().flatten().collect());
        }
        println!("{:#?}", all_of_key);
        all_of_key
    }

}