use std::fs;
use std::path;

use colored::Colorize;

pub fn check_save<P: AsRef<path::Path>>(path: &P) {
    
    let fd = fs::File::open(path).unwrap_or_else(|_| {
        let msg = format!("{} No file found at: {:?}", "ERROR:".red(), path.as_ref().as_os_str());
        eprintln!("{}", msg);
        std::process::exit(1);
    });

    println!("{:#?}", fd.metadata());
}