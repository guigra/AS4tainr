//! The native backend's unit tests (CLAUDE.native.md section 6): RPG
//! programs in core/tests/native/ that each say OK or FAIL for what they
//! check. They run on whichever engine the process uses
//! (AS4TAINR_RPG_ENGINE); scripts/native-diff.sh runs them under the
//! interpreter and under generated code. Expected values come from IBM's
//! documentation, cited per test; none was taken from a run of either
//! engine.
//!
//! Needs a PostgreSQL database (AS4TAINR_TEST_DATABASE_URL); without it
//! the tests say they are skipped.

use as4tainr_core::{catalog, rpg, runtime};

fn db_url() -> Option<String> {
    std::env::var("AS4TAINR_TEST_DATABASE_URL").ok()
}

macro_rules! require_db {
    () => {
        match db_url() {
            Some(u) => u,
            None => {
                eprintln!("skipped: AS4TAINR_TEST_DATABASE_URL not set");
                return;
            }
        }
    };
}

/// Run a program and give its DSPLY lines (without the DSPLY prefix).
async fn shown(name: &str, source: &str) -> Vec<String> {
    let url = require_db_url();
    let client = runtime::connect_db(&url)
        .await
        .unwrap_or_else(|e| panic!("{e:#}"));
    catalog::init(&client)
        .await
        .unwrap_or_else(|e| panic!("{e:#}"));
    let r = rpg::call(
        &url,
        vec!["QGPL".to_string()],
        "QSECOFR".to_string(),
        None,
        None,
        None,
        name.to_string(),
        source.to_string(),
        vec![],
    )
    .await
    .unwrap_or_else(|e| panic!("{name}: {e:#}"));
    assert!(r.error.is_none(), "{name}: {:?}", r.error);
    r.messages
        .iter()
        .map(|m| m.strip_prefix("DSPLY").unwrap_or(m).trim().to_string())
        .collect()
}

fn require_db_url() -> String {
    db_url().unwrap_or_default()
}

async fn expect(name: &str, source: &str, want: &[&str]) {
    let got = shown(name, source).await;
    assert_eq!(got, want, "{name}");
}

// Source: IBM Docs, "Passing by Reference", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasc/hpsref.htm, read 2026-11-11
// Source: IBM Docs, "Parameter Passing Styles", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasc/hpssty.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_byref_and_forwarding() {
    let _ = require_db!();
    expect(
        "NBYREF",
        include_str!("native/BYREF.rpgle"),
        &["BYREF OK", "FORWARD OK"],
    )
    .await;
}

// Source: IBM Docs, "Passing by Value", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasc/hpsval.htm, read 2026-11-11
// Source: IBM Docs, "CONST{(constant)}", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/dconst.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_value_and_const() {
    let _ = require_db!();
    expect(
        "NVALUE",
        include_str!("native/VALUE.rpgle"),
        &["VALUE OK", "CONST OK"],
    )
    .await;
}

// Calls in expressions and return values, and recursion: the arithmetic
// is the expected value (3*2 + 4*2 = 14, 2*2*2 = 8, 10! = 3628800).
// Source: IBM Docs, "Passing by Value", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasc/hpsval.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_calls_in_expressions() {
    let _ = require_db!();
    expect(
        "NCALLS",
        include_str!("native/CALLS.rpgle"),
        &["ARITH OK", "NESTED OK", "RECURSION OK"],
    )
    .await;
}

// Source: IBM Docs, "OVERLAY(name{:start_pos | *NEXT})", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/doverly.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_data_structure_overlay() {
    let _ = require_db!();
    expect(
        "NOVERLAY",
        include_str!("native/OVERLAY.rpgle"),
        &["OVERLAY READ OK", "OVERLAY WRITE OK"],
    )
    .await;
}

// Status 00102 and 00103, ON-ERROR codes and *PROGRAM, and control after
// ENDMON.
// Source: IBM Docs, "Program Status Codes", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/ptacode.htm, read 2026-11-11
// Source: IBM Docs, "ON-ERROR (On Error)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzonerr.htm, read 2026-11-11
// Source: IBM Docs, "TRUNCNBR(*YES | *NO)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/htrunc.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_division_by_zero_and_overflow() {
    let _ = require_db!();
    expect(
        "NERRORS",
        include_str!("native/ERRORS.rpgle"),
        &["DIV0 OK", "OVERFLOW OK", "PROGRAM OK", "AFTER ENDMON"],
    )
    .await;
}

// The default precision rules leave the quotient with no decimal places
// (100 / 1.5 → 66); %DEC keeps them (66.6666 in a 15,4 field).
// Source: IBM Docs, "Example of Default Precision Rules", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/precxdf.htm, read 2026-11-11
// Source: IBM Docs, "Precision of Intermediate Results", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/prectdf.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_precision_rules() {
    let _ = require_db!();
    expect(
        "NPRECIS",
        include_str!("native/PRECIS.rpgle"),
        &["DEFAULT RULES OK", "DEC RULES OK"],
    )
    .await;
}

// Source: IBM Docs, "ON-EXIT (On Exit)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzonexit.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_on_exit_return_value() {
    let _ = require_db!();
    expect(
        "NONEXIT",
        include_str!("native/ONEXIT.rpgle"),
        &["ONEXIT OK"],
    )
    .await;
}

// Source: IBM Docs, "FOR-EACH (For Each)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzforeach.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_for_each() {
    let _ = require_db!();
    expect(
        "NFOREACH",
        include_str!("native/FOREACH.rpgle"),
        &["Open", "Active", "Closed"],
    )
    .await;
}

// A GOTO/TAG/CABxx loop: generated as a state machine.
// Source: IBM Docs, "CABxx (Compare and Branch)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzcabxx.htm, read 2026-11-11
// Source: IBM Docs, "GOTO (Go To)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzgoto.htm, read 2026-11-11
// Source: IBM Docs, "TAG (Tag)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zztag.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_goto_tag_state_machine() {
    let _ = require_db!();
    expect(
        "NGOTOTAG",
        include_str!("native/GOTOTAG.rpgle"),
        &["GOTO OK"],
    )
    .await;
}

// Stage 2 (docs/native/adr/0006): FOR/DO control and conditions on global
// numeric fields, lowered to Rust. FOR compares the index with the limit
// before each pass and steps it after the body (so 1 to 10 by 3 sees 1,
// 4, 7, 10 and ends at 13; 10 downto 1 by 4 ends at -2; 5 to 4 does not
// run), and the index must hold one more than the limit.
// Source: IBM Docs, "FOR (For)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzfor.htm, read 2026-11-12
// Source: IBM Docs, "DOW (Do While)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzdow.htm, read 2026-11-12
// Source: IBM Docs, "Program Status Codes", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/ptacode.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_lowered_loops_and_conditions() {
    let _ = require_db!();
    expect(
        "NLOOP",
        include_str!("native/NLOOP.rpgle"),
        &[
            "FOR TO OK",
            "FOR DOWNTO OK",
            "FOR ZERO OK",
            "DOW OK",
            "INDEX OVERFLOW OK",
        ],
    )
    .await;
}

// Two reference parameters that are one variable share its storage.
// Source: golden capture tools/golden/native1.txt n1_alias (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_aliased_reference_parameters() {
    let _ = require_db!();
    expect("NALIAS", include_str!("native/ALIAS.rpgle"), &["ALIAS OK"]).await;
}

// Stage 2, S2.2 (docs/native/adr/0006): numeric EVAL of +, - and *,
// lowered to Rust: truncation, EVAL(H) rounding, the decimals of the
// intermediate results, and overflow.
// Source: golden capture tools/golden/native2.txt n2_eval (PUB400, IBM i 7.5, 2026-11-12)
// Source: IBM Docs, "Precision of Intermediate Results", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/prectdf.htm, read 2026-11-11
// Source: IBM Docs, "Program Status Codes", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/ptacode.htm, read 2026-11-11
#[tokio::test]
async fn native_unit_lowered_eval() {
    let _ = require_db!();
    expect(
        "NEVAL",
        include_str!("native/NEVAL.rpgle"),
        &[
            "EVAL TRUNC OK",
            "EVAL HALF OK",
            "EVAL HALF NEG OK",
            "EVAL FULL OK",
            "EVAL ADDSUB OK",
            "EVAL EDGE OK",
            "EVAL OVERFLOW OK",
        ],
    )
    .await;
}

// Stage 2, S2.3: fixed-form ADD, SUB, MULT, Z-ADD and Z-SUB, lowered to
// Rust: results, high-order truncation, MULT(H) and resulting indicators.
// Source: golden capture tools/golden/native3.txt n3_arith (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_fixed_arithmetic() {
    let _ = require_db!();
    expect(
        "NARITH",
        include_str!("native/NARITH.rpgle"),
        &[
            "ARITH ADD 58.01",
            "ARITH SUB -33.33 010",
            "ARITH TRUNC 3",
            "ARITH MULT 563.5",
            "ARITH MULTH 563.6",
            "ARITH MULTFULL 563.567",
            "ARITH ZADD 45.6 100",
            "ARITH ZSUB -12.34 010",
            "ARITH ZERO .00 001",
            "ARITH SUB1 -1.00",
        ],
    )
    .await;
}

// Stage 2, S2.4: character EVAL, EVALR, concatenation of fixed-length
// fields and comparisons (blank padding, EBCDIC order), lowered to Rust.
// Source: golden capture tools/golden/native4.txt n4_char (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_characters() {
    let _ = require_db!();
    expect(
        "NCHAR",
        include_str!("native/NCHAR.rpgle"),
        &[
            "CHR CAT [AB   XYZ  ]",
            "CHR TRUNC [AB  ]",
            "CHR EVALR [       XYZ]",
            "CHR EVALR TRUNC [ XYZ]",
            "CHR PAD [Q         ]",
            "CHR CMP PADDED EQ",
            "CHR CMP LT",
            "CHR CMP LOWER FIRST",
            "CHR CMP DIGIT HIGH",
        ],
    )
    .await;
}

// Stage 2, S2.7: fixed-form DIV and MVR, lowered to Rust: the quotient's
// decimal places, DIV(H), the remainder and its sign, indicators, and
// status 00102.
// Source: golden capture tools/golden/native5.txt n5_div (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_div_mvr() {
    let _ = require_db!();
    expect(
        "NDIV",
        include_str!("native/NDIV.rpgle"),
        &[
            "DIV 3 MVR 2",
            "DIV2 3.40 MVR2 .00",
            "DIVNEG -3 MVR -2",
            "DIVH 3",
            "DIVRES 8 100",
            "DIV0 00102",
        ],
    )
    .await;
}
