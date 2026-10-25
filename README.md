# xlsmith

[![crates.io](https://img.shields.io/crates/v/xlsmith.svg)](https://crates.io/crates/xlsmith)
[![ci](https://github.com/marceanlabs/xlsmith/actions/workflows/ci.yml/badge.svg)](https://github.com/marceanlabs/xlsmith/actions/workflows/ci.yml)

Read a spreadsheet and get the data out as CSV or JSON, without opening Excel.

```console
$ xlsmith csv sales.xlsx > sales.csv
$ xlsmith json sales.xlsx --header > sales.json
$ xlsmith sheets sales.xlsx
```

`.xlsx`, `.xls`, `.xlsb` and `.ods` go in. CSV or JSON comes out. It is one
program with nothing else attached, so it drops into a script, a container or a
scheduled job without a Python environment to carry along.

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [What it will not do](#what-it-will-not-do)
- [Performance](#performance)
- [Using it from Rust](#using-it-from-rust)
- [Contributing](#contributing)
- [Licence](#licence)

## Install

```console
$ cargo install xlsmith
```

Building from source needs Rust 1.85 or newer. Prebuilt binaries are on the
[releases page](https://github.com/marceanlabs/xlsmith/releases):

| Platform | Target |
|---|---|
| Linux x86_64 | `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl` |
| Linux arm64 | `aarch64-unknown-linux-gnu` |
| macOS, Apple silicon | `aarch64-apple-darwin` |
| macOS, Intel | `x86_64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |

The musl build is a fully static file, so it runs on any Linux distribution
whatever its C library.

## Usage

```console
$ xlsmith csv report.xlsx --sheet "Q3" --range A2:F500 -o q3.csv
```

| Command or flag | What it does |
|---|---|
| `sheets <file>` | lists every sheet with its row and column counts |
| `csv <file>` | writes one sheet as CSV, or every sheet with `--all-sheets <dir>` |
| `json <file>` | writes one sheet as JSON; `--header` uses the first row as the keys |
| `--sheet` | picks a sheet by name, or by its position counting from 1 |
| `--range` | takes one block, as in `A1:D100` |
| `--delimiter` | comma (the default), tab or semicolon |

Output goes to standard output unless you pass `-o`, so it pipes.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | the data was written |
| 1 | the file could not be read or written |
| 2 | the sheet or the range does not exist |

## What it will not do

**Calculate formulas.** You get the value Excel last saved into the file. If a
workbook was written by a program that never calculated its formulas, there is no
value in the file, and xlsmith will not invent one.

**Write spreadsheets.** There is no CSV to xlsx direction.

**Carry anything but data.** No styles, images, charts, pivot tables or macros. An
`.xlsm` file is read for its data and its macros are ignored.

Errors come out as Excel writes them (`#DIV/0!`), text stays text, numbers stay
numbers, and dates come out as `YYYY-MM-DD`.

## Performance

A 200,000 row workbook converts in 0.89 s, and the output is byte for byte what
`xlsx2csv` writes. The cost is memory: the whole sheet is held at once, so a
500 MB workbook needs about 500 MB of RAM.

The full table is in [BENCHMARKS.md](BENCHMARKS.md).

## Using it from Rust

```rust
use xlsmith::Book;

let mut book = Book::open("report.xlsx")?;
let rows = book.rows(Some("Q3"), None)?;   // Vec<Vec<String>>
```

`CellRange::parse` takes the same `A1:D100` text the command line takes, and
`write_csv` and `write_json` are public, so your own code can print output in the
same shape as the program.

## Contributing

Questions and bug reports go in the [issues](https://github.com/marceanlabs/xlsmith/issues).
Please open one before writing a patch, since the scope here is narrow on purpose
and a feature that falls outside it will be turned down whatever its quality.

Security problems should go through [private reporting](https://github.com/marceanlabs/xlsmith/security/advisories/new)
rather than a public issue.

## Licence

Apache-2.0.
