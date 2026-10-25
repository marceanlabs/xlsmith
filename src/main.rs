//! Command line for xlsmith. The work is in the library; this file decides what
//! to do with the answer.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use rayon::prelude::*;

use xlsmith::{Book, CellRange, Error, sink, write_csv, write_json};

#[derive(Parser)]
#[command(
    name = "xlsmith",
    version,
    about = "Read spreadsheets and write what is in them",
    long_about = "Read .xlsx, .xls, .xlsb and .ods and write a sheet as CSV or JSON.\n\
                  Formulas are not evaluated: the value the spreadsheet last saved is what you get."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List the sheets in a workbook
    Sheets {
        /// Workbook to read
        file: PathBuf,
    },
    /// Write a sheet, or every sheet, as CSV
    Csv {
        /// Workbook to read
        file: PathBuf,
        /// Sheet name, or its position counting from 1
        #[arg(short, long)]
        sheet: Option<String>,
        /// Cells to take, as in A1:D100
        #[arg(short, long)]
        range: Option<String>,
        /// Write every sheet to its own file in this directory
        #[arg(long, value_name = "DIR")]
        all_sheets: Option<PathBuf>,
        /// Where to write; defaults to standard output
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Field separator
        #[arg(long, value_enum, default_value = "comma")]
        delimiter: Delimiter,
    },
    /// Write a sheet, or every sheet, as JSON
    Json {
        /// Workbook to read
        file: PathBuf,
        /// Sheet name, or its position counting from 1
        #[arg(short, long)]
        sheet: Option<String>,
        /// Cells to take, as in A1:D100
        #[arg(short, long)]
        range: Option<String>,
        /// Use the first row as the keys of each object
        #[arg(long)]
        header: bool,
        /// Write every sheet to its own file in this directory
        #[arg(long, value_name = "DIR")]
        all_sheets: Option<PathBuf>,
        /// Where to write; defaults to standard output
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, ValueEnum)]
enum Delimiter {
    Comma,
    Tab,
    Semicolon,
}

impl Delimiter {
    fn byte(self) -> u8 {
        match self {
            Delimiter::Comma => b',',
            Delimiter::Tab => b'\t',
            Delimiter::Semicolon => b';',
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xlsmith: {e}");
            match e {
                Error::BadRange { .. } | Error::SheetNotFound { .. } => ExitCode::from(2),
                _ => ExitCode::from(1),
            }
        }
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    match cli.command {
        Command::Sheets { file } => list_sheets(&file),
        Command::Csv {
            file,
            sheet,
            range,
            all_sheets,
            output,
            delimiter,
        } => {
            let range = parse_range(range)?;
            if let Some(dir) = all_sheets {
                return all_sheets_to(&file, &dir, range, "csv", |rows, out| {
                    let w = sink(Some(out)).map_err(|source| write_error(out, source))?;
                    write_csv(rows, w, delimiter.byte()).map_err(|source| write_error(out, source))
                });
            }
            let mut book = Book::open(&file)?;
            let rows = book.rows(sheet.as_deref(), range)?;
            let out = output.as_deref();
            let w =
                sink(out).map_err(|source| write_error(out.unwrap_or(Path::new("-")), source))?;
            write_csv(&rows, w, delimiter.byte())
                .map_err(|source| write_error(out.unwrap_or(Path::new("-")), source))
        }
        Command::Json {
            file,
            sheet,
            range,
            header,
            all_sheets,
            output,
        } => {
            let range = parse_range(range)?;
            if let Some(dir) = all_sheets {
                return all_sheets_to(&file, &dir, range, "json", |rows, out| {
                    let w = sink(Some(out)).map_err(|source| write_error(out, source))?;
                    write_json(rows, w, header).map_err(|source| write_error(out, source))
                });
            }
            let mut book = Book::open(&file)?;
            let rows = book.rows(sheet.as_deref(), range)?;
            let out = output.as_deref();
            let w =
                sink(out).map_err(|source| write_error(out.unwrap_or(Path::new("-")), source))?;
            write_json(&rows, w, header)
                .map_err(|source| write_error(out.unwrap_or(Path::new("-")), source))
        }
    }
}

fn write_error(path: &Path, source: std::io::Error) -> Error {
    Error::Write {
        path: path.to_path_buf(),
        source,
    }
}

fn parse_range(spec: Option<String>) -> Result<Option<CellRange>, Error> {
    match spec {
        Some(s) => Ok(Some(CellRange::parse(&s)?)),
        None => Ok(None),
    }
}

fn list_sheets(file: &Path) -> Result<(), Error> {
    let mut book = Book::open(file)?;
    let list = book.sheets()?;
    println!(
        "{}: {} sheet{}",
        file.display(),
        list.len(),
        if list.len() == 1 { "" } else { "s" }
    );
    for s in list {
        println!("  {:<32} {} rows x {} columns", s.name, s.rows, s.cols);
    }
    Ok(())
}

/// Every sheet to its own file, named after the sheet, one worker per sheet.
fn all_sheets_to<F>(
    file: &Path,
    dir: &Path,
    range: Option<CellRange>,
    extension: &str,
    write_one: F,
) -> Result<(), Error>
where
    F: Fn(&[Vec<String>], &Path) -> Result<(), Error> + Sync + Send,
{
    let names = Book::open(file)?.names().to_vec();
    names.par_iter().try_for_each(|name| -> Result<(), Error> {
        let mut book = Book::open(file)?;
        let rows = book.rows(Some(name), range)?;
        let out = dir.join(format!("{}.{}", stem_for(name), extension));
        write_one(&rows, &out)
    })
}

/// A sheet name as a file name: letters, digits, dash and underscore survive,
/// everything else becomes a dash, because a sheet can be called anything.
fn stem_for(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else {
            out.push('-');
        }
    }
    let trimmed: String = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "sheet".to_string()
    } else {
        trimmed
    }
}
