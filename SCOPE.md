# xlsmith: scope decision

## The gap, with numbers (measured 25 October 2026)

| Evidence | Value | Source |
|---|---|---|
| `calamine` downloads in 90 days | 5,974,390 | crates.io API |
| `xlsx2csv` (Python) downloads | 812,807 a month | pypistats |
| `csvkit` downloads | 227,411 a month | pypistats |
| Best Rust xlsx CLI found | 14,381 downloads **total** (`xlsx-to-usv`) | crates.io search |
| Next best Rust xlsx CLI | 62 downloads (`zavora-xlsx-cli`) | crates.io search |

Same shape as imgsmith: the library layer is solved and heavily used, the command
line layer has no adopted tool. Over a million times a month someone converts a
spreadsheet from a shell, and almost none of them are using Rust to do it.

## In scope

- Read `.xlsx`, `.xls`, `.xlsb`, `.ods` (one dependency, `calamine`).
- `sheets`: name, rows, columns.
- `csv` and `json` out, one sheet or every sheet, one file each.
- A block of cells by `A1:D100`, a sheet by name or position.
- Values only: strings, numbers, booleans, dates, Excel error codes.
- Batch: `--all-sheets <dir>`.

## Out of scope, and why

| Refused | Why |
|---|---|
| Writing spreadsheets | `umya-spreadsheet` would double the dependency tree for a need the pitch does not cover |
| Evaluating formulas | a formula engine is a different product, and the file already stores its last value |
| Styles, fonts, colours, merged ranges | invisible in CSV and JSON; carrying them means carrying layout |
| Images, charts, pivot tables | not data |
| Macros | `.xlsm` is read as data; running VBA is never a thing this tool does |
| Streaming large sheets | the known weakness. `calamine` builds the range in memory; a streaming path is the next version, not this one |
| CSV to xlsx | the reverse direction, and not the demand that pays |

## Kill tests

Stop and rethink if any of these hold after a quarter:

1. Fewer than 500 crate downloads in 90 days and no issue from a stranger: the
   gap was not real, or nobody found it.
2. `calamine` gains an official CLI: we are then duplicating a project with more
   maintainers.
3. The streaming weakness turns out to matter more than speed: `xlsx2csv` already
   wins on memory, and a slow-but-tiny tool has no pitch.

## What it rests on

The whole tool is 470 lines of our own code around one mature dependency. If the
dependency is wrong, the tool is wrong, so the value is in the interface, the
verification and the honesty about the memory ceiling, not in novel parsing.
