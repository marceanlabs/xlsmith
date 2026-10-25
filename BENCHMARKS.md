# xlsmith benchmarks

200,000 rows and four columns, 4.6 MB, four cores. `scripts/bench.sh` builds the
workbook and repeats the run.

| Tool | Time | Peak memory | Output |
|---|---|---|---|
| xlsmith | 0.89 s | 84 MB | 7,103,724 bytes |
| xlsx2csv (Python) | 9.89 s | 19 MB | 7,103,724 bytes, identical |
| pandas `read_excel` | 21.20 s | 179 MB | 7,303,724 bytes |

The output is byte for byte what `xlsx2csv` writes, and it arrives about eleven
times sooner.

## Where it loses

The trade is memory. The whole sheet is held in memory at once, so a 500 MB
workbook needs about 500 MB of RAM, and `xlsx2csv` will win on files that size.

## Reproducing it

```console
$ scripts/bench.sh
```
