use std::path::Path;
use std::fs::{File, metadata};
use std::io::{BufRead, BufReader, BufWriter, Write};
use regex::Regex;

pub struct Bundler<'a> {
    crate_name: &'a Path, 
    bin_file: &'a Path,
    file_ptr: Box<dyn Write>,
    file_buf: String,
}

impl<'a> Bundler<'a> {
    pub fn new(crate_name: &'a str, bin_file: &'a str, target_file: &'a str) -> Self {
        let crate_name = Path::new(crate_name);
        let bin_file = Path::new(bin_file);
        let target_file = Path::new(target_file);

        File::create(target_file).unwrap();

        let file_ptr = Box::new(BufWriter::new(File::create(target_file).unwrap()));

        Self {
            crate_name,
            bin_file,
            file_ptr,
            file_buf: String::new(),
        }
    }

    fn write_to_file(&mut self, content: String, level: u32) {
        let mut indent = String::new();
        for _ in 0..level{
            indent.push_str("\t");
        }

        let comment_re = Regex::new(r"^\s*//.*$").unwrap();
        if comment_re.is_match(&content) { return; }

        // println!("{}{}", indent, content);
        // writeln!(self.file_ptr, "{}{}", indent, content).unwrap();
        self.file_buf += format!("{}{}\n", indent, content).as_str();
    }

    fn query_mod_block(vec: &Vec<&str>, lineno: usize) -> (usize, usize) {
        let mut start = lineno;
        let mut end = lineno;
        let attr_re = Regex::new(r"^\s*#\[.+\]$").unwrap();
        let blank_re = Regex::new(r"^\s*$").unwrap();
        while start >= 1 {
            let line = &vec[start - 1];
            if attr_re.is_match(line) || blank_re.is_match(line) {
                start -= 1;
            }
            else { break; }
        }

        let mut bracket_cnt = 0;
        while end < vec.len() {
            let line = &vec[end];
            bracket_cnt += line.chars().filter(|&c| c == '{').count();
            bracket_cnt -= line.chars().filter(|&c| c == '}').count();

            if bracket_cnt == 0 { break; }
            end += 1;
        }

        (start, end)
    }

    fn clean_inline_test_mod(&mut self) {
        let mut vec = vec![];
        for line in self.file_buf.lines() {
            vec.push(line);
        }

        let inline_mod_re = Regex::new(r"^\s*(pub\s+)?mod\s+\w+(\s+\{)?\s*$").unwrap();
        
        let mut del: Vec<(usize, usize)> = vec![];

        let mut i = 0;
        loop {
            if inline_mod_re.is_match(vec[i]) && vec[i].contains("tests") {
                let (start, end) = Self::query_mod_block(&vec, i);
                del.push((start, end));
                i = end;
            }
            i += 1;
            if i >= vec.len() { break; }
        }
        for i in (0..del.len()).rev() {
            let (start, end) = del[i];
            vec.drain(start..=end);
        }
        self.file_buf = vec.join("\n");
    }

    fn flush(&mut self) {
        self.clean_inline_test_mod();
        writeln!(self.file_ptr, "{}", self.file_buf).unwrap();
    }


    // Bundle all library files recursively
    fn bundle_lib(&mut self, path: &str, name: &str, level: u32) {
        // println!("[DEBUG]path = {}, name = {}", path, name);
        let lib_paths = vec![
            path.to_string() + name + ".rs", 
            path.to_string() + name + "/mod.rs"
        ];

        let mut tmp: i32 = -1;
        for i in 0..2{
            if let Ok(_) = metadata(&lib_paths[i]) {
                tmp = i as i32;
            }
        }
        assert!(tmp != -1, "Cannot find library file: {:?}", lib_paths);

        let mut new_path: &str = &(path.to_string() + name + "/");
        if tmp == 0 {
            new_path = path;
        }

        // 1. (pub) mod xxx;
        // 2. (pub) mod xxx { }
        let mod_import = Regex::new(r"^\s*(pub\s+)?mod\s+(?P<modname>\w+)\s*;\s*$").unwrap();

        let lib_path = &lib_paths[tmp as usize];
        let file = File::open(lib_path).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            if let Some(cap) = mod_import.captures(&line) {
                let mut tmp_line = line.clone();
                loop {
                    let c = tmp_line.pop().unwrap();
                    if c == ';' { break; }
                }
                tmp_line.push_str(" {");
                self.write_to_file(tmp_line, level);
                let modname = cap.name("modname").unwrap().as_str();

                self.bundle_lib(new_path, modname, level + 1);

                self.write_to_file("}".to_string(), level);
            }
            else {
                self.write_to_file(line, level);
            }
        }
    }

    pub fn run(&mut self){
        self.write_to_file(format!("pub mod {} {{", self.crate_name.to_str().unwrap()), 0);
        self.bundle_lib("src/", "lib", 1);
        self.write_to_file("}".to_string(), 0);

        let file = File::open(self.bin_file).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            self.write_to_file(line.unwrap(), 0);
        }

        self.flush();
    }
}
