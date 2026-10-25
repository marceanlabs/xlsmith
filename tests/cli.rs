//! Drive the binary the way a person does, and check what lands on stdout.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_xlsmith")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

struct Run {
    code: i32,
    out: String,
    err: String,
}

fn run(args: &[&str]) -> Run {
    let out = Command::new(bin())
        .args(args)
        .output()
        .expect("run xlsmith");
    Run {
        code: out.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        err: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn sheets_lists_every_sheet_with_its_size() {
    let r = run(&["sheets", fixture("simple.xlsx").to_str().unwrap()]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(r.out.contains("3 sheets"), "{}", r.out);
    assert!(r.out.contains("Sales"), "{}", r.out);
    assert!(r.out.contains("6 rows x 5 columns"), "{}", r.out);
}

#[test]
fn csv_writes_the_first_sheet_by_default() {
    let r = run(&["csv", fixture("simple.xlsx").to_str().unwrap()]);
    assert_eq!(r.code, 0, "{}", r.err);
    let first: Vec<&str> = r.out.lines().take(3).collect();
    assert_eq!(first[0], "region,units,price,date,in stock");
    assert_eq!(first[1], "North,12,9.99,2026-03-01,true");
    // A whole number keeps the shape a person typed, not 1200.0
    assert_eq!(first[2], "South,0,4.5,2026-03-02,false");
}

#[test]
fn csv_range_takes_only_the_block_asked_for() {
    let r = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--sheet",
        "Sales",
        "--range",
        "A2:B3",
    ]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(r.out, "North,12\nSouth,0\n");
}

#[test]
fn csv_takes_a_sheet_by_position() {
    let by_name = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--sheet",
        "Notes",
    ]);
    let by_number = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--sheet",
        "2",
    ]);
    assert_eq!(by_name.code, 0, "{}", by_name.err);
    assert_eq!(by_name.out, by_number.out);
}

#[test]
fn an_unknown_sheet_exits_2_and_says_what_is_there() {
    let r = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--sheet",
        "Nope",
    ]);
    assert_eq!(r.code, 2, "{}", r.err);
    assert!(r.err.contains("Nope"), "{}", r.err);
    assert!(
        r.err.contains("Sales"),
        "the message must name the sheets that exist: {}",
        r.err
    );
}

#[test]
fn a_bad_range_is_a_usage_error() {
    let r = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--range",
        "D9:A1",
    ]);
    assert_eq!(r.code, 2, "{}", r.err);
    assert!(
        r.err.contains("above") || r.err.contains("left"),
        "{}",
        r.err
    );
}

#[test]
fn quotes_commas_newlines_and_accents_survive() {
    let r = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--sheet",
        "Notes",
    ]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(
        r.out.contains("\"comma, inside \"\"quotes\"\"\""),
        "{}",
        r.out
    );
    assert!(r.out.contains("café — naïve"), "{}", r.out);
    assert!(r.out.contains("\"line\nbreak\""), "{}", r.out);
}

#[test]
fn json_with_a_header_makes_objects() {
    let r = run(&["json", fixture("simple.xlsx").to_str().unwrap(), "--header"]);
    assert_eq!(r.code, 0, "{}", r.err);
    let value: serde_json::Value = serde_json::from_str(&r.out).expect("valid json");
    let first = &value[0];
    assert_eq!(first["region"], "North");
    assert_eq!(first["units"], 12);
    assert_eq!(first["in stock"], true);
    assert_eq!(first["date"], "2026-03-01");
}

#[test]
fn json_without_a_header_makes_arrays() {
    let r = run(&[
        "json",
        fixture("simple.xlsx").to_str().unwrap(),
        "--range",
        "A1:B2",
    ]);
    assert_eq!(r.code, 0, "{}", r.err);
    let value: serde_json::Value = serde_json::from_str(&r.out).expect("valid json");
    assert_eq!(value[0][0], "region");
    assert_eq!(value[1][1], 12);
}

#[test]
fn every_sheet_goes_to_its_own_file() {
    let dir = std::env::temp_dir().join("xlsmith-all-sheets");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let r = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--all-sheets",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(r.code, 0, "{}", r.err);
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, vec!["Notes.csv", "Sales.csv", "Total.csv"]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ods_and_xls_read_the_same_values_as_xlsx() {
    let xlsx = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--sheet",
        "Sales",
    ]);
    for other in ["simple.ods", "simple.xls"] {
        let r = run(&["csv", fixture(other).to_str().unwrap(), "--sheet", "Sales"]);
        assert_eq!(r.code, 0, "{other}: {}", r.err);
        assert_eq!(
            r.out, xlsx.out,
            "{other} must read the same values as the xlsx"
        );
    }
}

#[test]
fn a_file_that_is_not_a_spreadsheet_exits_1() {
    let path = std::env::temp_dir().join("xlsmith-not-a-sheet.txt");
    fs::write(&path, b"this is not a workbook").unwrap();
    let r = run(&["csv", path.to_str().unwrap()]);
    assert_eq!(r.code, 1, "{}", r.err);
    assert!(r.err.contains("xlsmith:"), "{}", r.err);
    let _ = fs::remove_file(&path);
}

#[test]
fn tab_delimiter_replaces_the_separator() {
    let r = run(&[
        "csv",
        fixture("simple.xlsx").to_str().unwrap(),
        "--delimiter",
        "tab",
    ]);
    assert_eq!(r.code, 0, "{}", r.err);
    assert!(r.out.lines().next().unwrap().contains('\t'), "{}", r.out);
}
