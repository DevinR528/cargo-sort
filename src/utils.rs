
pub fn commas(v: &mut Vec<&str>) -> String {
    if let Some(el) = v.last() {
        if *el == "" {
            v.remove(v.len() - 1);
        }
    }
    v.join(", ")
}

pub fn expand_table(pair: (&String, &toml::value::Value), res: &mut String) -> () {
    let (k, v) = pair;
    match v {
        toml::value::Value::Table(t) => {
            let mut s = toml::ser::to_string(&t).unwrap();
            // TODO cp
            let mut v: Vec<&str> = s.lines()
                .filter(|l| *l != "").collect();
            
            res.push_str(&format!("{} = {{ {} }}\n", k, commas(&mut v)));
        },
        _ => res.push_str(&format!("{} = {}\n", k, v)),
    };
}