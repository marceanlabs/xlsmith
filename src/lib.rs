//! Read spreadsheets and write what is in them, without a runtime.
//!
//! The whole tool is this module: open a workbook, name its sheets, pull one
//! range out of one sheet, and render the cells as text. Nothing here evaluates
//! a formula, keeps a style, or touches a chart, because none of those change
//! what a cell says.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use calamine::{Data, Reader, Sheets};

pub use calamine;

/// One sheet, as the workbook describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetInfo {
    pub name: String,
    pub rows: usize,
    pub cols: usize,
}

/// Anything that can go wrong, with the file named.
#[derive(Debug)]
pub enum Error {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Read {
        path: PathBuf,
        source: calamine::Error,
    },
    SheetNotFound {
        path: PathBuf,
        wanted: String,
        available: Vec<String>,
    },
    BadRange {
        spec: String,
        reason: &'static str,
    },
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The reader panicked. A file that does this is malformed, and the fault is
    /// in the dependency, not here: say so rather than dying on a signal.
    ReaderPanic {
        path: PathBuf,
        detail: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Error::Read { path, source } => write!(f, "{}: {source}", path.display()),
            Error::SheetNotFound {
                path,
                wanted,
                available,
            } => write!(
                f,
                "{}: no sheet called {wanted:?}; it has {}",
                path.display(),
                list(available)
            ),
            Error::BadRange { spec, reason } => write!(f, "range {spec:?}: {reason}"),
            Error::Write { path, source } => write!(f, "{}: {source}", path.display()),
            Error::ReaderPanic { path, detail } => write!(
                f,
                "{}: the spreadsheet reader panicked on this file ({detail}). The file is \
                 malformed and the fault is in calamine, not in xlsmith",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {}

fn list(names: &[String]) -> String {
    match names.len() {
        0 => "no sheets".to_string(),
        1 => format!("{:?}", names[0]),
        _ => names
            .iter()
            .map(|n| format!("{n:?}"))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

type Result<T> = std::result::Result<T, Error>;

/// A range of cells, addressed the way a spreadsheet addresses them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRange {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: Option<usize>,
    pub end_col: Option<usize>,
}

impl CellRange {
    /// Parse `A1:D100`, `B2`, or `C:C` / `3:9` style edges.
    ///
    /// Columns and rows are zero-based here; the parser does the conversion so
    /// no other code has to think about letters.
    pub fn parse(spec: &str) -> Result<Self> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Err(Error::BadRange {
                spec: spec.to_string(),
                reason: "empty",
            });
        }
        let upper = spec.to_ascii_uppercase();
        let (left, right) = match upper.split_once(':') {
            Some((a, b)) => (a, Some(b)),
            None => (upper.as_str(), None),
        };
        if right == Some("") {
            return Err(Error::BadRange {
                spec: spec.to_string(),
                reason: "a colon needs a cell on both sides, as in A1:D10",
            });
        }
        let (start_row, start_col) = parse_cell(left, spec)?;
        let (end_row, end_col) = match right {
            Some(r) => {
                let (er, ec) = parse_cell(r, spec)?;
                (Some(er), Some(ec))
            }
            None => (None, None),
        };
        if let Some(er) = end_row {
            if er < start_row {
                return Err(Error::BadRange {
                    spec: spec.to_string(),
                    reason: "the end row is above the start row",
                });
            }
        }
        if let Some(ec) = end_col {
            if ec < start_col {
                return Err(Error::BadRange {
                    spec: spec.to_string(),
                    reason: "the end column is left of the start column",
                });
            }
        }
        Ok(CellRange {
            start_row,
            start_col,
            end_row,
            end_col,
        })
    }

    fn contains(&self, row: usize, col: usize) -> bool {
        if row < self.start_row || col < self.start_col {
            return false;
        }
        if let Some(er) = self.end_row {
            if row > er {
                return false;
            }
        }
        if let Some(ec) = self.end_col {
            if col > ec {
                return false;
            }
        }
        true
    }
}

fn parse_cell(text: &str, whole: &str) -> Result<(usize, usize)> {
    let letters: String = text
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let digits: String = text
        .chars()
        .skip_while(|c| c.is_ascii_alphabetic())
        .collect();
    if letters.is_empty() || digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::BadRange {
            spec: whole.to_string(),
            reason: "a cell looks like A1, letters then a row number",
        });
    }
    let mut col = 0usize;
    for c in letters.chars() {
        col = col * 26 + (c as u8 - b'A' + 1) as usize;
    }
    let row: usize = digits.parse().map_err(|_| Error::BadRange {
        spec: whole.to_string(),
        reason: "row number is too large",
    })?;
    if row == 0 {
        return Err(Error::BadRange {
            spec: whole.to_string(),
            reason: "rows start at 1",
        });
    }
    Ok((row - 1, col - 1))
}

thread_local! {
    /// Set while a dependency is parsing, so the panic hook can stay quiet about
    /// panics this module is going to turn into an ordinary error.
    static IN_READER: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Keep the default panic message for our own bugs, and drop it for the reader's.
///
/// Without this a malformed file prints a Rust panic trace and then our error
/// line, which reads like a crash that happened to carry on.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if IN_READER.with(|flag| flag.get()) {
            return;
        }
        default(info);
    }));
}

struct QuietReader;

impl QuietReader {
    fn enter() -> Self {
        IN_READER.with(|flag| flag.set(true));
        QuietReader
    }
}

impl Drop for QuietReader {
    fn drop(&mut self) {
        IN_READER.with(|flag| flag.set(false));
    }
}

/// Run a reader call, turning a panic into an error.
///
/// A parser is a dependency and a malformed file can make it panic; a command
/// line tool that dies on a signal has no way to say which file did it, and no
/// way to keep going with the next one.
fn guarded<T>(
    path: &Path,
    f: impl FnOnce() -> std::result::Result<T, calamine::Error>,
) -> Result<T> {
    let _quiet = QuietReader::enter();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(source)) => Err(Error::Read {
            path: path.to_path_buf(),
            source,
        }),
        Err(payload) => Err(Error::ReaderPanic {
            path: path.to_path_buf(),
            detail: panic_message(&payload),
        }),
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "no message".to_string()
    }
}

/// An open workbook. Held once and read from many times, because parsing the
/// file is the expensive part and every sheet is already inside it.
pub struct Book {
    path: PathBuf,
    inner: Sheets<BufReader<File>>,
    names: Vec<String>,
}

impl Book {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let inner = guarded(&path, || calamine::open_workbook_auto(&path))?;
        let names = inner.sheet_names().to_vec();
        Ok(Book { path, inner, names })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Every sheet with the size of the block of cells it actually holds.
    pub fn sheets(&mut self) -> Result<Vec<SheetInfo>> {
        let names = self.names.clone();
        let mut out = Vec::with_capacity(names.len());
        for name in names {
            let range = guarded(&self.path, || self.inner.worksheet_range(&name))?;
            let (rows, cols) = range.get_size();
            out.push(SheetInfo { name, rows, cols });
        }
        Ok(out)
    }

    /// Pick a sheet by name, or by position when the name is a number.
    fn resolve(&self, wanted: Option<&str>) -> Result<String> {
        match wanted {
            None => self
                .names
                .first()
                .cloned()
                .ok_or_else(|| Error::SheetNotFound {
                    path: self.path.clone(),
                    wanted: "(first sheet)".to_string(),
                    available: vec![],
                }),
            Some(w) => {
                if self.names.iter().any(|n| n == w) {
                    return Ok(w.to_string());
                }
                // A bare number means "the nth sheet", counting from one, which is
                // how a person reading `sheets` output thinks about it.
                if let Ok(idx) = w.parse::<usize>() {
                    if idx >= 1 && idx <= self.names.len() {
                        return Ok(self.names[idx - 1].clone());
                    }
                }
                Err(Error::SheetNotFound {
                    path: self.path.clone(),
                    wanted: w.to_string(),
                    available: self.names.clone(),
                })
            }
        }
    }

    /// The cells of one sheet, optionally cropped to a range.
    pub fn rows(
        &mut self,
        sheet: Option<&str>,
        range: Option<CellRange>,
    ) -> Result<Vec<Vec<String>>> {
        let name = self.resolve(sheet)?;
        let cells = guarded(&self.path, || self.inner.worksheet_range(&name))?;
        let (height, width) = cells.get_size();
        let mut out = Vec::new();
        for (row_idx, row) in cells.rows().enumerate() {
            if row_idx >= height {
                break;
            }
            if let Some(r) = range {
                if !r.contains(row_idx, r.start_col) {
                    continue;
                }
            }
            let mut rendered = Vec::with_capacity(width);
            for (col_idx, cell) in row.iter().enumerate() {
                if let Some(r) = range {
                    if !r.contains(row_idx, col_idx) {
                        continue;
                    }
                }
                rendered.push(text(cell));
            }
            // A cropped row that starts right of the first column needs no padding:
            // the columns asked for are the columns written.
            out.push(rendered);
        }
        Ok(out)
    }
}

/// How a cell is written out. Numbers keep the shape a person typed; dates
/// become ISO text so a CSV is readable without a spreadsheet.
pub fn text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => number(*f),
        Data::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Data::DateTime(dt) => match dt.as_datetime() {
            Some(naive) => {
                if naive.time() == chrono::NaiveTime::MIN {
                    naive.date().format("%Y-%m-%d").to_string()
                } else {
                    naive.format("%Y-%m-%d %H:%M:%S").to_string()
                }
            }
            None => number(dt.as_f64()),
        },
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => error_text(e),
    }
}

fn number(f: f64) -> String {
    if !f.is_finite() {
        return String::new();
    }
    // A whole number prints without a decimal point, the way a spreadsheet shows
    // it, unless it is too large for an i64 to hold.
    if f.fract() == 0.0 && f.abs() < 9.0e15 {
        return format!("{}", f as i64);
    }
    let mut s = f.to_string();
    if s.contains('e') && !s.contains("e-") {
        // 1e20 style output is not what a spreadsheet shows; expand it.
        s = format!("{f:.0}");
    }
    s
}

fn error_text(e: &calamine::CellErrorType) -> String {
    use calamine::CellErrorType as T;
    match e {
        T::Div0 => "#DIV/0!".into(),
        T::NA => "#N/A".into(),
        T::Name => "#NAME?".into(),
        T::Null => "#NULL!".into(),
        T::Num => "#NUM!".into(),
        T::Ref => "#REF!".into(),
        T::Value => "#VALUE!".into(),
        T::GettingData => "#GETTING_DATA".into(),
    }
}

/// Write rows as CSV, quoting only where a field needs it.
pub fn write_csv<W: Write>(rows: &[Vec<String>], out: W, delimiter: u8) -> std::io::Result<()> {
    let mut w = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .quote_style(csv::QuoteStyle::Necessary)
        .from_writer(out);
    for row in rows {
        w.write_record(row)?;
    }
    w.flush()
}

/// Write rows as JSON: objects when there is a header, arrays when there is not.
pub fn write_json<W: Write>(rows: &[Vec<String>], out: W, header: bool) -> std::io::Result<()> {
    let value = if header && !rows.is_empty() {
        let head = rows[0].clone();
        let body: Vec<serde_json::Value> = rows[1..]
            .iter()
            .map(|row| {
                let mut map = serde_json::Map::new();
                for (i, key) in head.iter().enumerate() {
                    if key.is_empty() {
                        continue;
                    }
                    let v = row.get(i).cloned().unwrap_or_default();
                    map.insert(key.clone(), guessed(&v));
                }
                serde_json::Value::Object(map)
            })
            .collect();
        serde_json::Value::Array(body)
    } else {
        serde_json::Value::Array(
            rows.iter()
                .map(|row| serde_json::Value::Array(row.iter().map(|v| guessed(v)).collect()))
                .collect(),
        )
    };
    let mut w = BufWriter::new(out);
    serde_json::to_writer_pretty(&mut w, &value)?;
    w.write_all(b"\n")
}

/// A cell that came out of a spreadsheet has no type any more, so JSON gets the
/// one the text looks like: a number, a boolean, null for empty, else a string.
fn guessed(v: &str) -> serde_json::Value {
    if v.is_empty() {
        return serde_json::Value::Null;
    }
    if v == "true" {
        return serde_json::Value::Bool(true);
    }
    if v == "false" {
        return serde_json::Value::Bool(false);
    }
    if let Ok(i) = v.parse::<i64>() {
        return serde_json::Value::Number(i.into());
    }
    if let Ok(f) = v.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return serde_json::Value::Number(n);
        }
    }
    serde_json::Value::String(v.to_string())
}

/// Write to `path`, or to stdout when there is no path. Callers pass `-`.
pub fn sink(path: Option<&Path>) -> std::io::Result<Box<dyn Write>> {
    match path {
        Some(p) => Ok(Box::new(BufWriter::new(File::create(p)?))),
        None => Ok(Box::new(std::io::stdout().lock())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_parse_columns_and_rows() {
        let r = CellRange::parse("A1:D10").unwrap();
        assert_eq!(
            (r.start_row, r.start_col, r.end_row, r.end_col),
            (0, 0, Some(9), Some(3))
        );
        let single = CellRange::parse("B2").unwrap();
        assert_eq!((single.start_row, single.start_col), (1, 1));
        assert!(single.end_row.is_none());
    }

    #[test]
    fn ranges_reject_nonsense() {
        assert!(CellRange::parse("").is_err());
        assert!(CellRange::parse("A").is_err());
        assert!(CellRange::parse("1").is_err());
        assert!(CellRange::parse("D10:A1").is_err());
        assert!(CellRange::parse("A0").is_err());
        assert!(CellRange::parse("A1:").is_err());
    }

    #[test]
    fn ranges_contain_only_their_block() {
        let r = CellRange::parse("B2:C3").unwrap();
        assert!(r.contains(1, 1));
        assert!(r.contains(2, 2));
        assert!(!r.contains(0, 1));
        assert!(!r.contains(1, 0));
        assert!(!r.contains(3, 2));
    }

    #[test]
    fn numbers_print_the_way_a_sheet_shows_them() {
        assert_eq!(number(1.0), "1");
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(-3.0), "-3");
        assert_eq!(number(f64::NAN), "");
    }

    #[test]
    fn json_guesses_types_from_text() {
        assert!(matches!(guessed("12"), serde_json::Value::Number(_)));
        assert!(matches!(guessed("true"), serde_json::Value::Bool(true)));
        assert!(matches!(guessed(""), serde_json::Value::Null));
        assert!(matches!(
            guessed("12 High Street"),
            serde_json::Value::String(_)
        ));
    }
}
