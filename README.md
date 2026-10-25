# xlsmith

Read a spreadsheet and write what is in it, from one binary. `.xlsx`, `.xls`,
`.xlsb` and `.ods` in; CSV or JSON out.

```console
$ xlsmith csv sales.xlsx > sales.csv
$ xlsmith json sales.xlsx --header > sales.json
$ xlsmith sheets sales.xlsx
```

## Install

```console
$ cargo install xlsmith
```

Or a prebuilt binary for Linux, macOS or Windows from the
[releases page](https://github.com/marceanlabs/xlsmith/releases).

## What it does

| Command | What it writes |
|---|---|
| `sheets <file>` | every sheet, with its rows and columns |
| `csv <file>` | one sheet as CSV, or every sheet with `--all-sheets <dir>` |
| `json <file>` | one sheet as JSON; `--header` keys objects by the first row |
| `--sheet` | a sheet by name, or by position counting from 1 |
| `--range` | one block, as in `A1:D100` |
| `--delimiter` | comma (default), tab or semicolon |

```console
$ xlsmith csv report.xlsx --sheet "Q3" --range A2:F500 -o q3.csv
```

## Speed

200,000 rows and four columns, 4.6 MB, on four cores. `scripts/bench.sh` builds
the workbook and repeats the run.

| Tool | Time | Peak memory | Output |
|---|---|---|---|
| xlsmith | 0.89 s | 84 MB | 7,103,724 bytes |
| xlsx2csv (Python) | 9.89 s | 19 MB | 7,103,724 bytes, identical |
| pandas `read_excel` | 21.20 s | 179 MB | 7,303,724 bytes |

The output is byte for byte what `xlsx2csv` writes, and it arrives eleven times
sooner. What it is not: streaming. The whole sheet is in memory, so a 500 MB
workbook needs about 500 MB of RAM, and `xlsx2csv` will beat it there.

## What it refuses

No formula evaluation: you get the value the file last saved, and a workbook
written by a program that never calculated has no value to give. It does not
write spreadsheets, so there is no CSV to xlsx direction. No styles, images,
charts, pivot tables or macros: `.xlsm` is read as data and its macros are not
carried anywhere. Errors come out as Excel spells them (`#DIV/0!`), text as text,
numbers as numbers, and dates as `YYYY-MM-DD`.

## As a library

```rust
use xlsmith::Book;

let mut book = Book::open("report.xlsx")?;
let rows = book.rows(Some("Q3"), None)?;   // Vec<Vec<String>>
```

`CellRange::parse` takes the same `A1:D100` text the command line takes, and
`write_csv` / `write_json` are public so a caller can render the same way the
binary does. 18 tests, including the same file read from `.xlsx`, `.xls` and
`.ods`, and every value cross-checked against openpyxl.

## Licence

Apache-2.0.
