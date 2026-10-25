#!/bin/bash
# Rebuild the workbook the README quotes, then time every tool on it.
#
# Needs: openpyxl (builds the file), xlsx2csv and pandas (the comparison columns).
# A missing comparison tool is fatal: a table with a silently absent column is a
# lie, and a failed run that prints 0.01 s reads as fast.
set -euo pipefail
cd "$(dirname "$0")/.." || exit 1

PY=${PYTHON:-/usr/bin/python3}
ROWS=${ROWS:-200000}
FILE=tests/fixtures/large.xlsx
OUT=$(mktemp -d /tmp/xlsmith-bench.XXXXXX)
BIN=target/release/xlsmith

echo "document: $ROWS rows, 4 columns"
echo "host:     $(nproc) cores, $(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs)"
echo "date:     $(date +%F)"
echo

cargo build --release --quiet

"$PY" - "$FILE" "$ROWS" <<'PY'
import datetime as dt
import sys
from openpyxl import Workbook

path, rows = sys.argv[1], int(sys.argv[2])
wb = Workbook(write_only=True)
ws = wb.create_sheet("Data")
ws.append(["id", "name", "value", "date"])
for i in range(rows):
    ws.append([i, f"row {i}", i * 1.5, dt.date(2026, 1, 1)])
wb.save(path)
PY
echo "built:    $FILE ($(du -h "$FILE" | cut -f1))"
echo

timeit() {
  local name=$1; shift
  local measured secs peak
  measured=$(/usr/bin/time -f '%e %M' "$@" 2>&1 >/dev/null | tail -1)
  secs=${measured%% *}
  peak=${measured##* }
  printf '| %-28s | %6.2f s | %s MB |\n' "$name" "$secs" "$((peak / 1024))"
}

echo "| Tool | Time | Peak memory |"
echo "|---|---|---|"
timeit "xlsmith" "$BIN" csv "$FILE" -o "$OUT/ours.csv"
timeit "xlsx2csv" xlsx2csv "$FILE" "$OUT/theirs.csv"
timeit "pandas read_excel" "$PY" -c "import pandas as pd; pd.read_excel('$FILE').to_csv('$OUT/pandas.csv', index=False)"
echo

echo "sizes:   xlsmith $(stat -c %s "$OUT/ours.csv")  xlsx2csv $(stat -c %s "$OUT/theirs.csv")  pandas $(stat -c %s "$OUT/pandas.csv")"
if cmp -s "$OUT/ours.csv" "$OUT/theirs.csv"; then
  echo "verdict: xlsmith and xlsx2csv wrote byte identical CSV"
else
  echo "verdict: xlsmith and xlsx2csv differ, which is a finding, not a footnote"
  diff <(head -5 "$OUT/ours.csv") <(head -5 "$OUT/theirs.csv") | head -10
  exit 1
fi
rm -rf "$OUT"
