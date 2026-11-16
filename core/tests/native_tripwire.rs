//! A tripwire for the native engine (docs/native/adr/0005): the generated
//! code's runtime (core/src/rpg/native_rt.rs) copies what
//! `Machine::run_code_at` does for each instruction and how it routes
//! errors. When upstream changes that code, or the error functions it
//! calls, this test fails: review native_rt.rs against the new code,
//! then record the new hash here, saying why in the commit.
//!
//! This is not a fidelity test; it compares the source text with the one
//! native_rt.rs was last reviewed against.

const INTERP: &str = include_str!("../src/rpg/interp.rs");
const ERRORS: &str = include_str!("../src/rpg/errors.rs");
const BIFS: &str = include_str!("../src/rpg/bifs.rs");
const PTRS: &str = include_str!("../src/rpg/ptrs.rs");

/// The text of a function, from its signature to its closing brace at
/// the same indentation.
fn function(src: &str, signature: &str, indent: &str) -> String {
    let Some((_, rest)) = src.split_once(signature) else {
        panic!("{signature} is gone: review native_rt.rs");
    };
    let close = format!("\n{indent}}}\n");
    let Some((body, _)) = rest.split_once(&close) else {
        panic!("no end for {signature}");
    };
    format!("{signature}{body}{close}")
}

fn sha256(text: &str) -> String {
    let d = ring::digest::digest(&ring::digest::SHA256, text.as_bytes());
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn native_runtime_matches_the_interpreter_it_copies() {
    let watched = [
        function(
            INTERP,
            "    fn run_code_at(&mut self, code: &Code, start: usize) -> Result<Flow> {",
            "    ",
        ),
        function(
            ERRORS,
            "pub(super) fn is_signal(err: &anyhow::Error) -> bool {",
            "",
        ),
        function(
            ERRORS,
            "pub(super) fn on_error_takes(entry: &str, code: u32) -> bool {",
            "",
        ),
        // Stage 2, ADR 0006: what lowered code reads, writes and compares
        // as the interpreter does.
        function(
            INTERP,
            "    fn load_var(&self, v: &VarInfo, index: Option<i128>) -> Result<Value> {",
            "    ",
        ),
        function(INTERP, "    fn store_var(\n", "    "),
        function(
            INTERP,
            "    pub fn eval(&mut self, ex: &Expr) -> Result<Value> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn assign(&mut self, target: &Expr, val: &Value, fit: Fit, right: bool) -> Result<()> {",
            "    ",
        ),
        // S2.2: EVAL and the precision rules lowered code asks for.
        function(INTERP, "    fn stmt_inner(&mut self, s: &Stmt) -> Result<()> {", "    "),
        function(
            INTERP,
            "    fn to_rule_scale(&self, ex: &Expr, v: Dec) -> Dec {",
            "    ",
        ),
        function(
            INTERP,
            "    fn num_precision(&self, ex: &Expr) -> Option<(u32, u32)> {",
            "    ",
        ),
        function(ERRORS, "    pub(super) fn stmt_e(&mut self, s: &Stmt) -> Result<()> {", "    "),
        // S2.8: where a procedure local is (Runtime::local copies sym's
        // place rule).
        function(INTERP, "    fn sym(&self, name: &str) -> Option<Sym> {", "    "),
        // S2.10-S2.20: what rpgrt/src/native.rs writes again (%CHAR's
        // DECEDIT, array elements, division, MOVE and MOVEL).
        function(
            INTERP,
            "pub fn char_of_num_edit(d: &Dec, point: char, zero: bool) -> String {",
            "",
        ),
        function(
            INTERP,
            "    fn table_index(&self, name: &str, v: &VarInfo, i: Option<i128>) -> Option<i128> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn elem_off(&self, v: &VarInfo, index: Option<i128>) -> Result<usize> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn divide(&self, l: &Expr, r: &Expr, a: &Dec, b: &Dec) -> Result<Dec> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn move_image(&mut self, ex: &Expr) -> Result<Vec<u8>> {",
            "    ",
        ),
        function(INTERP, "    fn do_move(\n", "    "),
        // S2.26: %EDITC.
        function(INTERP, "pub fn edit_code_with(\n", ""),
        // S2.29: %EDITW.
        function(
            INTERP,
            "pub fn edit_word(v: &Dec, digits: u32, decs: u32, word: &str) -> Result<String> {",
            "",
        ),
        // S2.27: %LOOKUP's search.
        function(
            BIFS,
            "    fn search(&self, x: &Value, all: &[Value], start: usize, n: usize, how: &str) -> Result<usize> {",
            "    ",
        ),
        // S2.35 (ADR 0007): native procedure calls (native_rt.rs `call`,
        // `call_frame`, `call_body`).
        function(
            INTERP,
            "    fn call_named(&mut self, name: &str, args: &[Expr]) -> Result<Option<Value>> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn call_proc(&mut self, pi: &'a ProcInfo, args: Vec<Arg>) -> Result<Option<Value>> {",
            "    ",
        ),
        function(INTERP, "    fn args_for(\n", "    "),
        // S2.46: the prototype path of a call.
        function(
            INTERP,
            "    fn call_named_inner(&mut self, name: &str, args: &[Expr]) -> Result<Option<Value>> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn same_type_variable(&mut self, a: &Expr, t: &DType) -> bool {",
            "    ",
        ),
        // S2.51: a CONST *VARSIZE argument by reference.
        function(
            INTERP,
            "    fn char_variable(&mut self, a: &Expr) -> bool {",
            "    ",
        ),
        function(
            ERRORS,
            "    pub(super) fn guarded(&mut self, code: &Code) -> Result<Flow> {",
            "    ",
        ),
        function(
            ERRORS,
            "    pub(super) fn recover(&mut self, err: anyhow::Error) -> Result<Recovery> {",
            "    ",
        ),
        // S2.36: DSPLY's text; S2.13's %CHAR of a number.
        function(
            INTERP,
            "    fn text_of(&self, v: &Value) -> Result<String> {",
            "    ",
        ),
        function(
            INTERP,
            "pub fn char_of_num_edit(d: &Dec, point: char, zero: bool) -> String {",
            "",
        ),
        // S2.39: indicators.
        function(INTERP, "    pub fn ind(&self, n: &str) -> bool {", "    "),
        function(INTERP, "    pub fn set_ind(&mut self, n: &str, v: bool) {", "    "),
        function(INTERP, "fn ind_name(n: &str) -> String {", ""),
        // S2.40: EXSR runs a routine as run_code does.
        function(
            INTERP,
            "    fn run_code(&mut self, code: &Code) -> Result<Flow> {",
            "    ",
        ),
        // S2.43: the error path.
        function(
            ERRORS,
            "    pub(super) fn note_error(&mut self, err: &anyhow::Error) -> u32 {",
            "    ",
        ),
        function(
            ERRORS,
            "    pub(super) fn psds_error(&mut self, code: u32, err: &anyhow::Error) {",
            "    ",
        ),
        function(
            ERRORS,
            "    pub(super) fn passes_by(&self, err: &anyhow::Error) -> bool {",
            "    ",
        ),
        function(
            ERRORS,
            "    fn ds_put(&mut self, ds: &str, off: usize, bytes: &[u8]) {",
            "    ",
        ),
        function(
            ERRORS,
            "    fn ds_get(&self, ds: &str, off: usize, len: usize) -> Vec<u8> {",
            "    ",
        ),
        function(
            ERRORS,
            "    fn ds_text(&mut self, ds: &str, from: usize, len: usize, text: &str) {",
            "    ",
        ),
        function(
            ERRORS,
            "    pub(super) fn ds_zoned(&mut self, ds: &str, from: usize, digits: u32, v: i64) {",
            "    ",
        ),
        function(ERRORS, "    fn statement(&self) -> String {", "    "),
        // S2.47: %ADDR's pointer.
        function(
            PTRS,
            "    pub(super) fn ptr_to(&self, buf: usize, off: usize) -> Value {",
            "    ",
        ),
        function(
            PTRS,
            "    pub(super) fn addr(&self, v: &Value, size: usize) -> Result<(usize, usize)> {",
            "    ",
        ),
        // S2.53: %CHAR of dates, times and timestamps.
        function(
            BIFS,
            "    pub(super) fn bif_override(&mut self, b: &str, args: &[Expr]) -> Result<Option<Value>> {",
            "    ",
        ),
        function(BIFS, "pub(super) fn temporal(v: &Value) -> Option<Temporal> {", ""),
        // S2.50: heap storage.
        function(
            PTRS,
            "    pub(super) fn alloc(&mut self, size: &Value) -> Result<Value> {",
            "    ",
        ),
        function(
            PTRS,
            "    pub(super) fn realloc(&mut self, p: &Value, size: &Value) -> Result<Value> {",
            "    ",
        ),
        function(
            PTRS,
            "    pub(super) fn dealloc(&mut self, p: &Value) -> Result<()> {",
            "    ",
        ),
        function(PTRS, "    pub(super) fn storage_op(", "    "),
        // S2.49: pointer arithmetic.
        function(
            PTRS,
            "    pub(super) fn ptr_arith(&self, op: &BinOp, x: &Value, y: &Value) -> Option<Result<Value>> {",
            "    ",
        ),
        // S2.48: a based field's place.
        function(
            PTRS,
            "    pub(super) fn based_place(&self, ptr: &str, off: usize, size: usize) -> (usize, usize) {",
            "    ",
        ),
        function(
            INTERP,
            "    fn room_for_based(&mut self, name: &str, p: &super::mem::Ptr) {",
            "    ",
        ),
        function(
            INTERP,
            "    fn assign(&mut self, target: &Expr, val: &Value, fit: Fit, right: bool) -> Result<()> {",
            "    ",
        ),
        function(
            INTERP,
            "    pub(super) fn sender(&self) -> crate::pgmmsg::Sender {",
            "    ",
        ),
    ];
    let hash = sha256(&watched.concat());
    // Reviewed on 2026-11-15 against native_rt.rs and the stage 2
    // helpers of rpgrt/src/native.rs (upstream dd47e28, with #9's %SUBST
    // and %SCAN starts and #10's MONITOR routing, plus the native hook at
    // the start of run_code_at). This adds the functions S2.35-S2.43 write
    // again: calls, DSPLY's text, indicators, run_code and the error path;
    // S2.46 adds call_named_inner (prototypes), S2.47 ptrs.rs ptr_to and
    // addr, assign and room_for_based, S2.48 based_place, S2.49 ptr_arith,
    // S2.50 alloc, realloc, dealloc and storage_op, S2.51 char_variable,
    // S2.53 bif_override and temporal;
    // upstream still dd47e28.
    let reviewed = "4821eccc7789af78b12965c7a1c6ae7bf72225c55bb308ef2a09af924a9e4800";
    assert_eq!(
        hash, reviewed,
        "run_code_at or the error functions changed: review native_rt.rs, then record the new hash"
    );
}
