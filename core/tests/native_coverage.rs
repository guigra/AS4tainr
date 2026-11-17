//! How much of each native unit runs natively (ADR 0007, phase A). Every
//! unit is compiled, its routines generated into a temporary directory,
//! and the instructions whose source map says "interp" counted. A count
//! above the unit's ceiling fails: a lowering that stops applying shows
//! here, which a parity test cannot see (the interpreter then runs the
//! instruction with the same result).
//!
//! The ceilings are what the generator reaches now: lowering more lowers
//! them, and they may only go down. They are not IBM i behaviour; the
//! units' expected output is tested in native_units.rs.
//!
//! Needs a PostgreSQL database (AS4TAINR_TEST_DATABASE_URL).

use as4tainr_core::{rpg, runtime};

const UNITS: &[(&str, &str, usize)] = &[
    ("ALIAS", include_str!("native/ALIAS.rpgle"), 2),
    ("BYREF", include_str!("native/BYREF.rpgle"), 0),
    ("CALLS", include_str!("native/CALLS.rpgle"), 1),
    ("ERRORS", include_str!("native/ERRORS.rpgle"), 0),
    ("FOREACH", include_str!("native/FOREACH.rpgle"), 2),
    ("GOTOTAG", include_str!("native/GOTOTAG.rpgle"), 2),
    ("NABS", include_str!("native/NABS.rpgle"), 0),
    ("NARITH", include_str!("native/NARITH.rpgle"), 4),
    ("NARR", include_str!("native/NARR.rpgle"), 0),
    ("NARRV", include_str!("native/NARRV.rpgle"), 0),
    ("NBIF", include_str!("native/NBIF.rpgle"), 0),
    ("NCALL", include_str!("native/NCALL.rpgle"), 2),
    ("NCHAR", include_str!("native/NCHAR.rpgle"), 0),
    ("NCHAIN", include_str!("native/NCHAIN.rpgle"), 0),
    ("NCHECK", include_str!("native/NCHECK.rpgle"), 0),
    ("NCONSTA", include_str!("native/NCONSTA.rpgle"), 0),
    ("NCONSTB", include_str!("native/NCONSTB.rpgle"), 0),
    ("NCOND", include_str!("native/NCOND.rpgle"), 0),
    ("NDIV", include_str!("native/NDIV.rpgle"), 1),
    ("NDIVE", include_str!("native/NDIVE.rpgle"), 0),
    ("NDSA", include_str!("native/NDSA.rpgle"), 0),
    ("NDSQ", include_str!("native/NDSQ.rpgle"), 0),
    ("NDSW", include_str!("native/NDSW.rpgle"), 0),
    ("NEDITC", include_str!("native/NEDITC.rpgle"), 0),
    ("NEDITW", include_str!("native/NEDITW.rpgle"), 0),
    ("NEVAL", include_str!("native/NEVAL.rpgle"), 0),
    ("NFIG", include_str!("native/NFIG.rpgle"), 0),
    ("NFUN", include_str!("native/NFUN.rpgle"), 0),
    ("NFUN2", include_str!("native/NFUN2.rpgle"), 0),
    ("NIND", include_str!("native/NIND.rpgle"), 0),
    ("NLOOK", include_str!("native/NLOOK.rpgle"), 0),
    ("NLOOP", include_str!("native/NLOOP.rpgle"), 0),
    ("NMINMAX", include_str!("native/NMINMAX.rpgle"), 0),
    ("NMOVE", include_str!("native/NMOVE.rpgle"), 0),
    ("NPROC", include_str!("native/NPROC.rpgle"), 1),
    ("NPTR", include_str!("native/NPTR.rpgle"), 0),
    ("NBASED", include_str!("native/NBASED.rpgle"), 1),
    ("NPARITH", include_str!("native/NPARITH.rpgle"), 1),
    ("NPNULL", include_str!("native/NPNULL.rpgle"), 0),
    ("NHEAP", include_str!("native/NHEAP.rpgle"), 0),
    ("NPARM", include_str!("native/NPARM.rpgle"), 1),
    ("NDATE", include_str!("native/NDATE.rpgle"), 0),
    ("NDATEFIG", include_str!("native/NDATEFIG.rpgle"), 0),
    ("NTCHAR", include_str!("native/NTCHAR.rpgle"), 0),
    ("NTDSP", include_str!("native/NTDSP.rpgle"), 0),
    ("NTBIF", include_str!("native/NTBIF.rpgle"), 0),
    ("NTDUR", include_str!("native/NTDUR.rpgle"), 0),
    ("NTDURF", include_str!("native/NTDURF.rpgle"), 0),
    ("NCFUNC", include_str!("native/NCFUNC.rpgle"), 2),
    ("NREM", include_str!("native/NREM.rpgle"), 0),
    ("NREPL", include_str!("native/NREPL.rpgle"), 0),
    ("NSCAN", include_str!("native/NSCAN.rpgle"), 0),
    ("NSIZE", include_str!("native/NSIZE.rpgle"), 0),
    ("NSRPL", include_str!("native/NSRPL.rpgle"), 0),
    ("NSTAT", include_str!("native/NSTAT.rpgle"), 0),
    ("NSUB", include_str!("native/NSUB.rpgle"), 0),
    ("NSUBT", include_str!("native/NSUBT.rpgle"), 0),
    ("NVAR", include_str!("native/NVAR.rpgle"), 0),
    ("NVARDIM", include_str!("native/NVARDIM.rpgle"), 1),
    ("NSUBT2", include_str!("native/NSUBT2.rpgle"), 0),
    ("NVLEN", include_str!("native/NVLEN.rpgle"), 0),
    ("ONEXIT", include_str!("native/ONEXIT.rpgle"), 1),
    ("OVERLAY", include_str!("native/OVERLAY.rpgle"), 0),
    ("PRECIS", include_str!("native/PRECIS.rpgle"), 0),
    ("VALUE", include_str!("native/VALUE.rpgle"), 0),
];

/// The instructions of a directory's source maps that run through the
/// interpreter, as "program line kind what" lines.
fn interp_lines(dir: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if !p.to_string_lossy().ends_with(".map.json") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap_or_default();
        // Each instruction is one JSON object on the line; "via" says how.
        for ins in text.split("{\"pc\":").skip(1) {
            if ins.contains("\"via\":\"interp\"") {
                let field = |k: &str| {
                    ins.split(&format!("\"{k}\":"))
                        .nth(1)
                        .and_then(|r| r.split(',').next())
                        .unwrap_or("")
                        .to_string()
                };
                out.push(format!(
                    "line {} {} {}",
                    field("line"),
                    field("kind"),
                    field("what")
                ));
            }
        }
    }
    out.sort();
    out
}

#[tokio::test]
async fn native_units_are_lowered() {
    let Ok(url) = std::env::var("AS4TAINR_TEST_DATABASE_URL") else {
        eprintln!("skipped: AS4TAINR_TEST_DATABASE_URL not set");
        return;
    };
    let client = runtime::connect_db(&url).await.unwrap();
    as4tainr_core::catalog::init(&client).await.unwrap();
    let base = std::env::temp_dir().join(format!("native-coverage-{}", std::process::id()));
    let mut over = Vec::new();
    let mut counts = Vec::new();
    for (name, source, ceiling) in UNITS {
        let prog = rpg::compile_with(&client, &["QGPL".to_string()], name, source, "")
            .await
            .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let report = rpg::native::emit_into(&prog, &dir);
        assert!(report.errors.is_empty(), "{name}: {:?}", report.errors);
        let lines = interp_lines(&dir);
        counts.push(format!("{name} {}", lines.len()));
        if lines.len() > *ceiling {
            over.push(format!("{name}: {} > {ceiling}: {lines:?}", lines.len()));
        }
    }
    let _ = std::fs::remove_dir_all(&base);
    eprintln!("interp instructions per unit: {}", counts.join(", "));
    assert!(
        over.is_empty(),
        "units above their ceiling:\n{}",
        over.join("\n")
    );
}

/// An expression nested past IBM i's limit of 100 levels (ILE RPG
/// Reference, Appendix A. RPG IV Restrictions, IBM i 7.5,
/// https://www.ibm.com/docs/ssw_ibm_i_75/rzasd/rpgrest.htm, read
/// 2026-11-15) is not generated: its routine is refused with a diagnostic
/// that names the line, instead of being run by the interpreter (ADR 0007,
/// S2.41). One level fewer is generated.
///
/// It runs with the stack the as4tainr binary gives its runtime (64 MiB,
/// main.rs): compiling a hundred nested calls recurses deeply.
#[test]
fn expressions_past_ibm_nesting_are_refused() {
    let Ok(url) = std::env::var("AS4TAINR_TEST_DATABASE_URL") else {
        eprintln!("skipped: AS4TAINR_TEST_DATABASE_URL not set");
        return;
    };
    let t = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .thread_stack_size(64 * 1024 * 1024)
                .build()
                .unwrap()
                .block_on(nesting(url));
        })
        .unwrap();
    t.join().unwrap();
}

async fn nesting(url: String) {
    let client = runtime::connect_db(&url).await.unwrap();
    as4tainr_core::catalog::init(&client).await.unwrap();
    // %ABS nested n times around a field: n + 1 levels with the field.
    // Source: golden capture tools/golden/native49.txt n49_crt_a100 and
    // n49_crt_a101 (PUB400, IBM i 7.5, 2026-11-16): 99 compile, 100 give
    // RNF0640 at severity 30.
    let source = |n: usize| {
        format!(
            "**FREE\ndcl-s k int(10) inz(-3);\nk = {}k{};\ndsply ('K ' + %char(k));\n*inlr = *on;\n",
            "%abs(".repeat(n),
            ")".repeat(n)
        )
    };
    for (n, refused) in [(99, false), (100, true)] {
        let name = format!("NEST{n}");
        let prog = rpg::compile_with(&client, &["QGPL".to_string()], &name, &source(n), "")
            .await
            .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let dir = std::env::temp_dir().join(format!("native-nest-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let report = rpg::native::emit_into(&prog, &dir);
        let _ = std::fs::remove_dir_all(&dir);
        let says = report
            .errors
            .iter()
            .any(|e| e.contains("line 3") && e.contains("nested more than 100 levels"));
        assert_eq!(says, refused, "{name}: {:?}", report.errors);
    }
}
