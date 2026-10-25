# xlsmith

Read a spreadsheet and get the data out as CSV or JSON, without opening Excel.

```console
$ xlsmith csv sales.xlsx > sales.csv
$ xlsmith json sales.xlsx --header > sales.json
$ xlsmith sheets sales.xlsx
```

`.xlsx`, `.xls`, `.xlsb` and `.ods` go in. CSV or JSON comes out. It is one
program with nothing else attached, so it drops into a script, a container or a
scheduled job without a Python environment to carry along.

## Install

```console
$ cargo install xlsmith
```

Prebuilt binaries for Linux, macOS and Windows are on the
[releases page](https://github.com/marceanlabs/xlsmith/releases).

## Commands

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

## How fast it is

200,000 rows and four columns, 4.6 MB, four cores. `scripts/bench.sh` builds the
workbook and repeats the run.

| Tool | Time | Peak memory | Output |
|---|---|---|---|
| xlsmith | 0.89 s | 84 MB | 7,103,724 bytes |
| xlsx2csv (Python) | 9.89 s | 19 MB | 7,103,724 bytes, identical |
| pandas `read_excel` | 21.20 s | 179 MB | 7,303,724 bytes |

The output is byte for byte what `xlsx2csv` writes, and it arrives about eleven
times sooner.

The trade is memory. The whole sheet is held in memory at once, so a 500 MB
workbook needs about 500 MB of RAM, and `xlsx2csv` will win on files that size.

## What it will not do

**Calculate formulas.** You get the value Excel last saved into the file. If a
workbook was written by a program that never calculated its formulas, there is no
value in the file, and xlsmith will not invent one.

**Write spreadsheets.** There is no CSV to xlsx direction.

**Carry anything but data.** No styles, images, charts, pivot tables or macros. An
`.xlsm` file is read for its data and its macros are ignored.

Errors come out as Excel writes them (`#DIV/0!`), text stays text, numbers stay
numbers, and dates come out as `YYYY-MM-DD`.

## Using it from Rust

```rust
use xlsmith::Book;

let mut book = Book::open("report.xlsx")?;
let rows = book.rows(Some("Q3"), None)?;   // Vec<Vec<String>>
```

`CellRange::parse` takes the same `A1:D100` text the command line takes, and
`write_csv` and `write_json` are public, so your own code can print output in the
same shape as the program.

The test suite reads the same workbook from `.xlsx`, `.xls` and `.ods`, checks
that all three agree, and compares every value against openpyxl.

## Licence

Apache-2.0.
