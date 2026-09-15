use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{BufRead, BufReader},
};

fn main() {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("file.txt")
        .expect("Failed to open file");
    let reader = BufReader::new(&file);
    let mut count = HashMap::new();
    for line in reader.lines() {
        if let Ok(res) = line {
            for ch in res.split_whitespace() {
                *count.entry(ch.to_string()).or_insert(0) += 1;
            }
        }
    }

    for (k, v) in count {
        println!("{} {}", k, v)
    }
}
