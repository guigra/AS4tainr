//! as4tainr-native-emit: generate the native code of the RPG programs of
//! a set of libraries (CLAUDE.native.md section 3, ADR 0003; PLAN.md N5).
//!
//! ```text
//! as4tainr-native-emit --out DIR [--database-url URL] [--libl A,B,...] LIB...
//! ```
//!
//! Each RPG program of each library is compiled as its activation compiles
//! it, with the library list it will run with: the system libraries, then
//! `--libl` (default: the libraries named, then QGPL). Its routines are
//! written to DIR as `<fingerprint>.rs` and `.map.json`. A production
//! build then takes them with `AS4TAINR_NATIVE_DIR=DIR cargo build
//! --release`. A line per program goes to standard output. The exit
//! status is 1 when a program does not compile or a routine is not
//! generated, 2 for a usage error.

#![forbid(unsafe_code, unused_must_use)]
#![forbid(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects
)]

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

/// The command line.
struct Args {
    out: PathBuf,
    url: String,
    libl: Option<Vec<String>>,
    libs: Vec<String>,
}

fn usage() -> String {
    "usage: as4tainr-native-emit --out DIR [--database-url URL] [--libl A,B,...] LIB...".to_string()
}

fn parse(mut it: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut out = None;
    let mut url = std::env::var("AS4TAINR_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://as4tainr:as4tainr@127.0.0.1/as4tainr".to_string());
    let mut libl = None;
    let mut libs = Vec::new();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = Some(PathBuf::from(it.next().ok_or_else(usage)?)),
            "--database-url" => url = it.next().ok_or_else(usage)?,
            "--libl" => {
                libl = Some(
                    it.next()
                        .ok_or_else(usage)?
                        .split(',')
                        .filter(|l| !l.is_empty())
                        .map(|l| l.to_ascii_uppercase())
                        .collect(),
                )
            }
            "-h" | "--help" => return Err(usage()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}\n{}", usage())),
            s => libs.push(s.to_ascii_uppercase()),
        }
    }
    let out = out.ok_or_else(usage)?;
    if libs.is_empty() {
        return Err(usage());
    }
    Ok(Args {
        out,
        url,
        libl,
        libs,
    })
}

// The runtime is built by hand: `#[tokio::main]` expands to an
// `expect`, which the lints above forbid.
fn main() -> ExitCode {
    let args = match parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "{e}");
            return ExitCode::from(2);
        }
    };
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "as4tainr-native-emit: {e}");
            return ExitCode::from(1);
        }
    };
    match rt.block_on(run(&args)) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "as4tainr-native-emit: {e:#}");
            ExitCode::from(1)
        }
    }
}

/// Generate every library's programs; false when anything was not
/// generated.
async fn run(args: &Args) -> anyhow::Result<bool> {
    let client = as4tainr_core::runtime::connect_db(&args.url).await?;
    let user = args.libl.clone().unwrap_or_else(|| {
        let mut l = args.libs.clone();
        if !l.iter().any(|x| x == "QGPL") {
            l.push("QGPL".to_string());
        }
        l
    });
    let libl: Vec<String> = as4tainr_core::commands::SYSTEM_LIBRARY_LIST
        .iter()
        .map(|l| l.to_string())
        .chain(user)
        .collect();
    let mut stdout = std::io::stdout();
    let mut ok = true;
    let (mut written, mut present) = (0usize, 0usize);
    for lib in &args.libs {
        let programs =
            as4tainr_core::rpg::native::emit_library(&client, &libl, lib, &args.out).await?;
        for p in programs {
            match p.result {
                Ok(r) => {
                    written = written.saturating_add(r.written);
                    present = present.saturating_add(r.present);
                    writeln!(
                        stdout,
                        "{lib}/{}: {} routines written, {} already there",
                        p.name, r.written, r.present
                    )?;
                    for e in &r.errors {
                        ok = false;
                        writeln!(stdout, "{lib}/{}: not generated: {e}", p.name)?;
                    }
                }
                Err(e) => {
                    ok = false;
                    writeln!(stdout, "{lib}/{}: not compiled: {e}", p.name)?;
                }
            }
        }
    }
    writeln!(
        stdout,
        "as4tainr-native-emit: {written} routines written, {present} already there, in {}",
        args.out.display()
    )?;
    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Result<Args, String> {
        parse(v.iter().map(|s| s.to_string()))
    }

    #[test]
    fn command_line() {
        let a = args(&["--out", "d", "--libl", "app,qgpl", "app"]);
        assert!(matches!(&a, Ok(a) if a.out == std::path::Path::new("d")
            && a.libl.as_deref() == Some(&["APP".to_string(), "QGPL".to_string()][..])
            && a.libs == ["APP"]));
        assert!(args(&["app"]).is_err(), "--out is needed");
        assert!(args(&["--out", "d"]).is_err(), "a library is needed");
        assert!(args(&["--out", "d", "--bogus", "x"]).is_err());
    }
}
