# xlsmith: build notes and status

Third tool in the Marcean Labs family, after imgsmith and pdfsurgeon. Same shape:
one narrow job, pure Rust, one static binary, refusals stated up front.

## Why this one

`calamine` is downloaded 5.97M times in 90 days, `xlsx2csv` is run 812k times a
month, and the best Rust CLI in the space has 14k downloads in total. The library
layer is solved; the command line layer is not. Full evidence in `SCOPE.md`.

## What it is

- `sheets`, `csv`, `json`. One sheet by name or position, one block by `A1:D100`,
  or every sheet into its own file.
- Values only: strings, numbers, booleans, dates, Excel error codes.
- 18 tests: 5 unit, 13 against the binary. Every value cross-checked against
  openpyxl, and the same workbook read from `.xlsx`, `.xls` and `.ods`.

## Decisions

- **calamine, not a parser of our own.** Four formats, 16M downloads, and the
  alternative is owning xlsx, xlsb, ods and BIFF parsing. Same reasoning as
  imgsmith's codecs: depend on the codec, own the tool.
- **Whole numbers print without `.0`**, which is what `xlsx2csv` does and what a
  spreadsheet shows. This is why our CSV is byte identical to theirs on a 200k
  row file, while pandas differs in every float.
- **Dates render as `YYYY-MM-DD`**, datetimes as `YYYY-MM-DD HH:MM:SS`. A cell
  with a date format but no time prints as a date.
- **JSON guesses the type from the text**: a number becomes a number, `true` and
  `false` become booleans, empty becomes null. The alternative is strings for
  everything, which makes the output useless to a program.
- **A sheet name becomes a file name** with anything non-alphanumeric replaced,
  so `Q3 Report/Sales` cannot escape the output directory.
- **`--all-sheets` reopens the workbook per sheet** and runs one worker per
  sheet. Reading is cheap, and rayon over sheets beats one pass over a workbook
  the reader cannot share across threads.

## Known weakness

The whole sheet is in memory: 84 MB peak for 200,000 rows, against 19 MB for
`xlsx2csv`. A 500 MB workbook needs about 500 MB of RAM. A streaming path is the
next version, not this one.

## Status

- 0.1.0, published to crates.io, six release targets, CI on three operating
  systems plus a job that compares every cell against openpyxl.
- Fuzz target `parse` with committed seeds; nothing found in a three minute run.

## Where things are

- `scripts/bench.sh` rebuilds the 200,000 row workbook and times xlsmith,
  xlsx2csv and pandas, failing if our CSV and theirs differ.
- `scripts/verify_against_openpyxl.py --quick` is the CI check; without `--quick`
  it also builds and checks the large workbook.
- `tests/fixtures/` holds a 6 KB workbook in three formats. The large one is
  generated, not committed.
