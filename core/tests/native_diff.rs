//! Level 1 of the differential harness (docs/native/adr/0005): every
//! program of a corpus runs under the interpreter and under generated
//! code, in this one process, from the same data, and everything it left
//! must be the same: DSPLY lines, printed lines, parameters, the error,
//! its global storage (the PSDS's clock positions aside), its indicators
//! and the records of the files it uses.
//!
//! Generated code exists only in a build made with AS4TAINR_NATIVE_DIR
//! (scripts/native-diff.sh). In a build without any, the test says it is
//! skipped; the script requires that it is not.
//!
//! Needs a PostgreSQL database (AS4TAINR_TEST_DATABASE_URL).

use as4tainr_core::rpg::native::{self, Engine, Observation};
use as4tainr_core::{catalog, dds, rla, rpg, runtime};
use std::sync::{Arc, Mutex};
use tokio_postgres::Client;

/// The corpus library: a fixed name, so nothing random enters the
/// comparison.
const LIB: &str = "NDIFF1";

struct Case {
    name: &'static str,
    source: &'static str,
    args: &'static [&'static str],
}

/// The batch samples that need only CUSTMAST and ORDLIN (OWING needs
/// logical files; demo.sh covers it, level 3), and the native unit tests.
const CORPUS: &[Case] = &[
    Case {
        name: "CUSTLIST",
        source: include_str!("../../samples/qrpglesrc/CUSTLIST.rpgle"),
        args: &["3"],
    },
    Case {
        name: "ORDTOTAL",
        source: include_str!("../../samples/qrpglesrc/ORDTOTAL.rpgle"),
        args: &["1002"],
    },
    Case {
        name: "CUSTPRT",
        source: include_str!("../../samples/qrpglesrc/CUSTPRT.rpgle"),
        args: &[],
    },
    Case {
        name: "CUSTUPD",
        source: include_str!("../../samples/qrpglesrc/CUSTUPD.rpgle"),
        args: &["5"],
    },
    Case {
        name: "NBYREF",
        source: include_str!("native/BYREF.rpgle"),
        args: &[],
    },
    Case {
        name: "NVALUE",
        source: include_str!("native/VALUE.rpgle"),
        args: &[],
    },
    Case {
        name: "NCALLS",
        source: include_str!("native/CALLS.rpgle"),
        args: &[],
    },
    Case {
        name: "NOVERLAY",
        source: include_str!("native/OVERLAY.rpgle"),
        args: &[],
    },
    Case {
        name: "NERRORS",
        source: include_str!("native/ERRORS.rpgle"),
        args: &[],
    },
    Case {
        name: "NPRECIS",
        source: include_str!("native/PRECIS.rpgle"),
        args: &[],
    },
    Case {
        name: "NONEXIT",
        source: include_str!("native/ONEXIT.rpgle"),
        args: &[],
    },
    Case {
        name: "NFOREACH",
        source: include_str!("native/FOREACH.rpgle"),
        args: &[],
    },
    Case {
        name: "NGOTOTAG",
        source: include_str!("native/GOTOTAG.rpgle"),
        args: &[],
    },
    Case {
        name: "NLOOP",
        source: include_str!("native/NLOOP.rpgle"),
        args: &[],
    },
    Case {
        name: "NALIAS",
        source: include_str!("native/ALIAS.rpgle"),
        args: &[],
    },
    Case {
        name: "NEVAL",
        source: include_str!("native/NEVAL.rpgle"),
        args: &[],
    },
    Case {
        name: "NARITH",
        source: include_str!("native/NARITH.rpgle"),
        args: &[],
    },
    Case {
        name: "NCHAR",
        source: include_str!("native/NCHAR.rpgle"),
        args: &[],
    },
    Case {
        name: "NBIF",
        source: include_str!("native/NBIF.rpgle"),
        args: &[],
    },
    Case {
        name: "NSUB",
        source: include_str!("native/NSUB.rpgle"),
        args: &[],
    },
    Case {
        name: "NMOVE",
        source: include_str!("native/NMOVE.rpgle"),
        args: &[],
    },
    Case {
        name: "NSCAN",
        source: include_str!("native/NSCAN.rpgle"),
        args: &[],
    },
    Case {
        name: "NCOND",
        source: include_str!("native/NCOND.rpgle"),
        args: &[],
    },
    Case {
        name: "NFUN",
        source: include_str!("native/NFUN.rpgle"),
        args: &[],
    },
    Case {
        name: "NFUN2",
        source: include_str!("native/NFUN2.rpgle"),
        args: &[],
    },
    Case {
        name: "NARR",
        source: include_str!("native/NARR.rpgle"),
        args: &[],
    },
    Case {
        name: "NDSQ",
        source: include_str!("native/NDSQ.rpgle"),
        args: &[],
    },
    Case {
        name: "NDIVE",
        source: include_str!("native/NDIVE.rpgle"),
        args: &[],
    },
    Case {
        name: "NREM",
        source: include_str!("native/NREM.rpgle"),
        args: &[],
    },
    Case {
        name: "NVAR",
        source: include_str!("native/NVAR.rpgle"),
        args: &[],
    },
    Case {
        name: "NVLEN",
        source: include_str!("native/NVLEN.rpgle"),
        args: &[],
    },
    Case {
        name: "NARRV",
        source: include_str!("native/NARRV.rpgle"),
        args: &[],
    },
    Case {
        name: "NFIG",
        source: include_str!("native/NFIG.rpgle"),
        args: &[],
    },
    Case {
        name: "NREPL",
        source: include_str!("native/NREPL.rpgle"),
        args: &[],
    },
    Case {
        name: "NDSW",
        source: include_str!("native/NDSW.rpgle"),
        args: &[],
    },
    Case {
        name: "NSIZE",
        source: include_str!("native/NSIZE.rpgle"),
        args: &[],
    },
    Case {
        name: "NEDITC",
        source: include_str!("native/NEDITC.rpgle"),
        args: &[],
    },
    Case {
        name: "NLOOK",
        source: include_str!("native/NLOOK.rpgle"),
        args: &[],
    },
    Case {
        name: "NSRPL",
        source: include_str!("native/NSRPL.rpgle"),
        args: &[],
    },
    Case {
        name: "NDIV",
        source: include_str!("native/NDIV.rpgle"),
        args: &[],
    },
    Case {
        name: "NPROC",
        source: include_str!("native/NPROC.rpgle"),
        args: &[],
    },
];

/// The library again, with the sample files and their data, as before
/// each run.
async fn fresh_library(url: &str) -> Client {
    let mut client = runtime::connect_db(url).await.unwrap();
    catalog::init(&client).await.unwrap();
    client
        .batch_execute(&format!(
            "DROP SCHEMA IF EXISTS {} CASCADE; DELETE FROM as4tainr.objects WHERE library='{LIB}' OR name='{LIB}';
             DELETE FROM as4tainr.files WHERE library='{LIB}'; DELETE FROM as4tainr.file_fields WHERE library='{LIB}';
             DELETE FROM as4tainr.file_keys WHERE library='{LIB}';",
            catalog::ident(LIB)
        ))
        .await
        .unwrap();
    catalog::create_library(&client, LIB, "native diff corpus")
        .await
        .unwrap();
    for (name, src, csv) in [
        (
            "CUSTMAST",
            include_str!("../../samples/qddssrc/CUSTMAST.pf"),
            include_str!("../../samples/data/CUSTMAST.csv"),
        ),
        (
            "ORDLIN",
            include_str!("../../samples/qddssrc/ORDLIN.pf"),
            include_str!("../../samples/data/ORDLIN.csv"),
        ),
    ] {
        let pf = dds::parse_pf(name, src).unwrap();
        catalog::create_physical_file(&mut client, LIB, &pf)
            .await
            .unwrap();
        let f = catalog::resolve_file(&client, &[LIB.to_string()], name)
            .await
            .unwrap();
        let mut of = rla::OpenFile::new(f);
        let mut lines = csv.lines();
        let header: Vec<String> = lines.next().unwrap().split(',').map(String::from).collect();
        for line in lines {
            let rec: rla::Record = header.iter().cloned().zip(split_csv(line)).collect();
            of.write(&client, &rec).await.unwrap();
        }
    }
    client
}

/// The fields of a CSV line (double quotes around fields with commas).
fn split_csv(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// The files' records, in a fixed order.
async fn records(client: &Client) -> Vec<String> {
    let mut out = Vec::new();
    for t in ["custmast", "ordlin"] {
        let rows = client
            .query(
                &format!(
                    "SELECT row_to_json(t)::text FROM {}.{t} t ORDER BY 1",
                    catalog::ident(LIB)
                ),
                &[],
            )
            .await
            .unwrap();
        out.extend(rows.iter().map(|r| r.get::<_, String>(0)));
    }
    out
}

/// What one run left.
#[derive(Debug, PartialEq)]
struct Run {
    messages: Vec<String>,
    printed: Vec<String>,
    params: Vec<String>,
    error: Option<String>,
    observed: Vec<Seen>,
    records: Vec<String>,
}

/// An observation with its global buffers keyed by the names of what is
/// in them. The compiler may number the buffers of a program's external
/// fields differently from one compile to the next (each run here
/// compiles again, after the library is made again), so buffers are
/// matched by name, not by number.
#[derive(Debug, PartialEq)]
struct Seen {
    program: String,
    globals: Vec<(Vec<String>, Vec<u8>)>,
    indicators: Vec<(String, bool)>,
}

fn seen(o: Observation) -> Seen {
    let mut globals: Vec<(Vec<String>, Vec<u8>)> = o
        .globals
        .into_iter()
        .enumerate()
        .map(|(i, b)| (o.names.get(i).cloned().unwrap_or_default(), b))
        .collect();
    globals.sort();
    Seen {
        program: o.program,
        globals,
        indicators: o.indicators,
    }
}

async fn run(url: &str, case: &Case, engine: Engine) -> Run {
    let _ = fresh_library(url).await;
    let sink: native::Sink = Arc::new(Mutex::new(Vec::new()));
    let g = native::scope(engine);
    let s = native::observe_into(sink.clone());
    let r = rpg::call(
        url,
        vec![LIB.to_string()],
        "QSECOFR".to_string(),
        None,
        None,
        None,
        case.name.to_string(),
        case.source.to_string(),
        case.args.iter().map(|a| a.to_string()).collect(),
    )
    .await;
    drop(s);
    drop(g);
    let client = runtime::connect_db(url).await.unwrap();
    let r = r.unwrap_or_else(|e| panic!("{}: {e:#}", case.name));
    // The engine is the one difference expected; it is left out.
    let observed: Vec<Seen> = sink.lock().unwrap().drain(..).map(seen).collect();
    Run {
        messages: r.messages,
        printed: r.printed,
        params: r.params,
        error: r.error.map(|e| format!("{e:#}")),
        observed,
        records: records(&client).await,
    }
}

#[tokio::test]
async fn native_diff_corpus() {
    let Some(url) = std::env::var("AS4TAINR_TEST_DATABASE_URL").ok() else {
        eprintln!("skipped: AS4TAINR_TEST_DATABASE_URL not set");
        return;
    };
    if native::generated_routines() == 0 {
        eprintln!("skipped: no generated code in this build (scripts/native-diff.sh builds it)");
        return;
    }
    for case in CORPUS {
        let interp = run(&url, case, Engine::Interp).await;
        let native = run(&url, case, Engine::Rust).await;
        assert!(
            interp.error.is_none(),
            "{}: the interpreter failed: {:?}",
            case.name,
            interp.error
        );
        assert_eq!(
            interp.observed.len(),
            1,
            "{}: one observation per call",
            case.name
        );
        if let (Some(a), Some(b)) = (interp.observed.first(), native.observed.first()) {
            for ((names, x), (_, y)) in a.globals.iter().zip(&b.globals) {
                if let Some(at) = x.iter().zip(y).position(|(p, q)| p != q) {
                    let near = |v: &Vec<u8>| {
                        v.iter()
                            .skip(at.saturating_sub(4))
                            .take(16)
                            .copied()
                            .collect::<Vec<u8>>()
                    };
                    panic!(
                        "{}: storage of {names:?} differs at offset {at}: interpreter {:?}, native {:?}",
                        case.name,
                        near(x),
                        near(y)
                    );
                }
            }
        }
        assert_eq!(interp, native, "{}: the engines differ", case.name);
    }
}
