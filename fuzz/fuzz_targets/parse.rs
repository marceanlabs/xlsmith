#![no_main]

//! Every workbook is a file from somewhere else, so the path the binary takes is
//! the path that gets fuzzed: open, list, read, render, write.

use std::io::Write;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // calamine reads from a path, so the bytes become a file first. The name
    // decides the parser, and xlsx is the one most people have.
    let dir = std::env::temp_dir().join("xlsmith-fuzz");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("input.xlsx");
    if std::fs::write(&path, data).is_err() {
        return;
    }

    let Ok(mut book) = xlsmith::Book::open(&path) else {
        return;
    };
    let names = book.names().to_vec();
    let _ = book.sheets();
    if let Some(first) = names.first() {
        if let Ok(rows) = book.rows(Some(first), None) {
            let mut sink = Vec::new();
            let _ = xlsmith::write_csv(&rows, &mut sink, b',');
            let mut sink = Vec::new();
            let _ = xlsmith::write_json(&rows, &mut sink, true);
            // A range that only touches the edge is where an off-by-one shows.
            if let Some(range) = xlsmith::CellRange::parse("B2:D4").ok() {
                let _ = book.rows(Some(first), Some(range));
            }
        }
    }
    let mut flush = std::io::sink();
    let _ = flush.write_all(b"");
});
