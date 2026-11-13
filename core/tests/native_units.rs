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

// Stage 2, S2.8: lowered code inside procedures (locals, parameters).
// Source: IBM Docs, "Passing by Reference", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasc/hpsref.htm, read 2026-11-11
// Source: IBM Docs, "Passing by Value", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasc/hpsval.htm, read 2026-11-11
// Source: IBM Docs, "FOR (For)", IBM i 7.5,
// https://www.ibm.com/docs/en/ssw_ibm_i_75/rzasd/zzfor.htm, read 2026-11-12
#[tokio::test]
async fn native_unit_lowered_procedures() {
    let _ = require_db!();
    expect(
        "NPROC",
        include_str!("native/NPROC.rpgle"),
        &["PROC LOOP OK", "PROC BYREF OK", "PROC RECURSION OK"],
    )
    .await;
}

// Stage 2, S2.9: %TRIM, %TRIML, %TRIMR and %LEN of character fields,
// lowered to Rust (trimming in concatenation and EVALR, all blanks, %LEN
// in arithmetic).
// Source: golden capture tools/golden/native6.txt n6_bif (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_trim_len() {
    let _ = require_db!();
    expect(
        "NBIF",
        include_str!("native/NBIF.rpgle"),
        &[
            "TRIM [AB        ]",
            "TRIML [AB        ]",
            "TRIMR [  AB      ]",
            "TRIM CAT [XYAB      ]",
            "TRIM BLANKS [Q         ]",
            "TRIM EVALR [        AB]",
            "LEN 6",
            "LEN EXPR 12",
        ],
    )
    .await;
}

// Stage 2, S2.10: %SUBST of character expressions, lowered to Rust:
// literal and field arguments, the rest of the string, %SUBST of %TRIM,
// and status 00100 for a start or a length out of range.
// Source: golden capture tools/golden/native7.txt n7_subst (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_subst() {
    let _ = require_db!();
    expect(
        "NSUB",
        include_str!("native/NSUB.rpgle"),
        &[
            "SUBST [CDEF ]",
            "SUBST REST [HIJ  ]",
            "SUBST VARS [BCD  ]",
            "SUBST TRIM [YZQ  ]",
            "SUBST END ZERO 00100",
            "SUBST START 00100",
            "SUBST LEN 00100",
        ],
    )
    .await;
}

// Stage 2, S2.11: fixed-form MOVE and MOVEL lowered to Rust: from the
// right and the left, padded, character to numeric (digits from each
// byte, the sign from the last zone), numeric to numeric and to
// character, literals.
// Source: golden capture tools/golden/native8.txt n8_move (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_move() {
    let _ = require_db!();
    expect(
        "NMOVE",
        include_str!("native/NMOVE.rpgle"),
        &[
            "MOVE [ABXYZ]",
            "MOVEL [XYZDE]",
            "MOVEP [  XYZ]",
            "MOVELP [XYZ  ]",
            "MOVE NUM [AB123]",
            "MOVE CHR NUM 765",
            "MOVE ZONE SIGN -121",
            "MOVE NUM NUM 345",
            "MOVEL NUM NUM 12.3",
            "MOVE NEG [04K]",
            "MOVE LIT [AB7]",
            "MOVEL LIT [QRCDE]",
        ],
    )
    .await;
}

// Stage 2, S2.12: %SCAN and %XLATE lowered to Rust: with and without a
// start, a pattern with blanks, %SCAN in arithmetic and of a
// concatenation, %XLATE with a shorter `to`, and status 00100 for an
// %XLATE start past the end.
// Source: golden capture tools/golden/native9.txt n9_scan (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_scan_xlate() {
    let _ = require_db!();
    expect(
        "NSCAN",
        include_str!("native/NSCAN.rpgle"),
        &[
            "SCAN 2",
            "SCAN START 5",
            "SCAN NONE 0",
            "SCAN BLANK 7",
            "SCAN EXPR 11",
            "SCAN VAR 4",
            "SCAN CAT 8",
            "SCAN PAST ERR 100",
            "SCAN ZERO ERR 100",
            "XLATE [AXBYC     ]",
            "XLATE SHORT [xyCxyCXYZ ]",
            "XLATE START [ABCzBCXYZ ]",
            "XLATE PAST ERR 100",
        ],
    )
    .await;
}

// Stage 2, S2.13: conditions with built-in functions and concatenation on
// either side, lowered to Rust (%SCAN, %SUBST, %TRIM, %TRIML with blank
// padding, %LEN, %XLATE, AND and OR).
// Source: golden capture tools/golden/native11.txt n11_cond (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_conditions_with_functions() {
    let _ = require_db!();
    expect(
        "NCOND",
        include_str!("native/NCOND.rpgle"),
        &[
            "SCAN COND YES",
            "SCAN COND NONE",
            "SUBST COND EQ",
            "TRIM COND EQ",
            "CAT COND EQ",
            "TRIML COND PADDED EQ",
            "AND COND YES",
            "OR COND YES",
            "LEN COND YES",
            "XLATE COND EQ",
            "SUBST COND LT",
        ],
    )
    .await;
}

// Stage 2, S2.14: %CHAR of numbers, %INT, %INTH, %DEC and %DECH lowered to
// Rust, in expressions and conditions, and status 00103 when %DEC's digits
// are too few.
// Source: golden capture tools/golden/native12.txt n12_fun (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_char_int_dec() {
    let _ = require_db!();
    expect(
        "NFUN",
        include_str!("native/NFUN.rpgle"),
        &[
            "CHAR NEG [-1.50       ]",
            "CHAR INT [1234        ]",
            "CHAR FRACTION [.25         ]",
            "CHAR EXPR [2468        ]",
            "CHAR INT FN [-1          ]",
            "CHAR INTH FN [-2          ]",
            "CHAR LIT [X3          ]",
            "INT EXPR 2",
            "INTH EXPR 3",
            "DEC -1.5",
            "DECH 1.3",
            "DEC SMALL ERR 103",
            "INT COND YES",
        ],
    )
    .await;
}

// Stage 2, S2.14: %CHAR under DECEDIT('0,'), lowered to Rust.
// Source: golden capture tools/golden/native12.txt n12_decedit (PUB400, IBM i 7.5, 2026-11-12)
#[tokio::test]
async fn native_unit_lowered_char_decedit() {
    let _ = require_db!();
    expect(
        "NFUN2",
        include_str!("native/NFUN2.rpgle"),
        &[
            "DECEDIT FRACTION [0,25        ]",
            "DECEDIT NEG [-12,50      ]",
        ],
    )
    .await;
}

// Stage 2, S2.15: array elements with literal indexes lowered to Rust, in
// EVAL, expressions, conditions, %TRIM, and an array subfield of a data
// structure (the FOR loop with a variable index stays with the
// interpreter).
// Source: golden capture tools/golden/native16.txt n16_arr (PUB400, IBM i 7.5, 2026-11-13)
#[tokio::test]
async fn native_unit_lowered_array_elements() {
    let _ = require_db!();
    expect(
        "NARR",
        include_str!("native/NARR.rpgle"),
        &[
            "ARR FIRST LAST 1.25 2.50",
            "ARR CHAR [    |AB  |ABC ]",
            "ARR COND GT",
            "ARR SUM 5.00",
            "ARR DS [XYXY  ]",
        ],
    )
    .await;
}

// Stage 2, S2.16: qualified subfields (QUALIFIED, and LIKEDS of a
// template) lowered to Rust, in EVAL, arithmetic, %TRIM, %SUBST and
// conditions.
// Source: golden capture tools/golden/native17.txt n17_dsq (PUB400, IBM i 7.5, 2026-11-13)
#[tokio::test]
async fn native_unit_lowered_qualified_subfields() {
    let _ = require_db!();
    expect(
        "NDSQ",
        include_str!("native/NDSQ.rpgle"),
        &[
            "DSQ CUST 43 [ACME      ]",
            "DSQ LIKEDS 86 [ACM]",
            "DSQ COND YES",
            "DSQ BYTES [ACM]",
        ],
    )
    .await;
}

// Stage 2, S2.17: / in expressions lowered to Rust: the quotient's
// decimal places by the precision rules (literals, packed and integer
// operands), EVAL and EVAL(H), a nested expression, status 00102.
// Source: golden capture tools/golden/native18.txt n18_div (PUB400, IBM i 7.5, 2026-11-13)
#[tokio::test]
async fn native_unit_lowered_division() {
    let _ = require_db!();
    expect(
        "NDIVE",
        include_str!("native/NDIVE.rpgle"),
        &[
            "DIV LIT [3.3333333333333333333333333333          ]",
            "DIV LIT100 [33.333333333333333333333333333          ]",
            "DIV PACKED [3.3333333333333333333333333             ]",
            "DIV INT [3.33333333333333333333                  ]",
            "DIV ASSIGN 3.33",
            "DIV HALF .67",
            "DIV EXPR 7.50",
            "DIV ZERO 00102",
        ],
    )
    .await;
}

// Stage 2, S2.18: %REM and %DIV lowered to Rust: signs, in arithmetic and
// in a condition, and status 00102 for a zero divisor.
// Source: golden capture tools/golden/native19.txt n19_rem (PUB400, IBM i 7.5, 2026-11-13)
#[tokio::test]
async fn native_unit_lowered_rem_div() {
    let _ = require_db!();
    expect(
        "NREM",
        include_str!("native/NREM.rpgle"),
        &[
            "REM 2",
            "REM NEG -2",
            "DIVF 3",
            "DIVF NEG -3",
            "REM DIV BACK 123",
            "REM COND ODD",
            "REM ZERO ERR 102",
            "DIVF ZERO ERR 102",
        ],
    )
    .await;
}

// Stage 2, S2.19: varying-length character fields lowered to Rust:
// assignment, concatenation, truncation at the maximum, %TRIM, to and from
// fixed-length fields, a padded comparison, %SUBST and %SCAN.
// Source: golden capture tools/golden/native20.txt n20_var (PUB400, IBM i 7.5, 2026-11-13)
#[tokio::test]
async fn native_unit_lowered_varying_fields() {
    let _ = require_db!();
    expect(
        "NVAR",
        include_str!("native/NVAR.rpgle"),
        &[
            "VAR [HELLO] 5",
            "VAR CAT [HELLO W] 7",
            "VAR TRUNC [TOOLO] 5",
            "VAR TRIM [X] 1",
            "VAR TO CHAR [TOOLO ]",
            "VAR FROM CHAR [TOOLO ] 6",
            "VAR CMP PADDED EQ",
            "VAR SUBST [TOO] 3",
            "VAR SCAN 2",
        ],
    )
    .await;
}

// Stage 2, S2.20: %LEN of varying-length fields lowered to Rust: their
// current length, in arithmetic and conditions, of an empty field and
// after %TRIMR.
// Source: golden capture tools/golden/native21.txt n21_vlen (PUB400, IBM i 7.5, 2026-11-13)
#[tokio::test]
async fn native_unit_lowered_varying_length() {
    let _ = require_db!();
    expect(
        "NVLEN",
        include_str!("native/NVLEN.rpgle"),
        &[
            "VLEN 3",
            "VLEN EXPR 10",
            "VLEN EMPTY 0",
            "VLEN COND GT",
            "VLEN TRIMR 3",
        ],
    )
    .await;
}
