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
const CYCLE: &str = include_str!("../src/rpg/cycle.rs");
const EXEC_SQL: &str = include_str!("../src/rpg/exec_sql.rs");
const OPS: &str = include_str!("../src/rpg/ops.rs");
const PRINTER: &str = include_str!("../src/rpg/printer.rs");

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
        // S2.55: durations, %DIFF and %SUBDT (bif_more, above).
        function(
            BIFS,
            "    pub(super) fn date_arith(&mut self, op: BinOp, x: Value, y: Value) -> Result<Value> {",
            "    ",
        ),
        function(BIFS, "fn as_ts(t: Temporal) -> dates::Ts {", ""),
        // S2.54: %DATE, %TIME and %TIMESTAMP.
        function(
            BIFS,
            "    pub(super) fn bif_more(&mut self, b: &str, args: &[Expr]) -> Result<Option<Value>> {",
            "    ",
        ),
        function(
            BIFS,
            "    fn temporal_arg(&mut self, args: &[Expr]) -> Result<(Value, Option<String>)> {",
            "    ",
        ),
        function(
            BIFS,
            "    pub(super) fn special_arg(&self, args: &[Expr], i: usize) -> Option<String> {",
            "    ",
        ),
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
        // S2.58: reads of database files.
        function(ERRORS, "    pub(super) fn file_op(\n", "    "),
        function(
            ERRORS,
            "    pub(super) fn file_failed(&mut self, i: usize, err: &anyhow::Error) {",
            "    ",
        ),
        function(
            ERRORS,
            "    pub(super) fn file_idx(&self, name: &str) -> Option<usize> {",
            "    ",
        ),
        function(ERRORS, "fn opcode_name(op: FileOp) -> &'static str {", ""),
        function(INTERP, "    fn file_op_inner(\n", "    "),
        function(
            INTERP,
            "    fn key_values(&mut self, key: &[Expr]) -> Result<Vec<String>> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn fill_fields(&mut self, i: usize, rec: &Record) -> Result<()> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn file_status(&self, args: &[Expr]) -> Result<Status> {",
            "    ",
        ),
        function(
            CYCLE,
            "    pub(super) fn move_input(&mut self, i: usize, rec: &Record) -> Result<()> {",
            "    ",
        ),
        // S2.59: WRITE, UPDATE and DELETE.
        function(
            INTERP,
            "    fn record_of(&mut self, i: usize) -> Result<Record> {",
            "    ",
        ),
        function(INTERP, "    pub(super) fn record_change(\n", "    "),
        function(INTERP, "    fn rla_change(\n", "    "),
        function(
            INTERP,
            "    fn fref_of(&self, i: usize) -> Result<crate::catalog::FileRef> {",
            "    ",
        ),
        // S2.64: multiple-occurrence data structures.
        function(
            OPS,
            "    pub(super) fn occurrence(&self, e: &Expr) -> Result<u32> {",
            "    ",
        ),
        function(
            OPS,
            "    pub(super) fn set_occurrence(&mut self, e: &Expr, n: i128) -> Result<()> {",
            "    ",
        ),
        function(OPS, "    fn occur_op(\n", "    "),
        // S2.69: embedded SQL, step 1.
        function(EXEC_SQL, "fn sql_error(e: &anyhow::Error) -> Option<Outcome> {", ""),
        function(EXEC_SQL, "fn words(sql: &str) -> Vec<String> {", ""),
        function(EXEC_SQL, "fn find_word(sql: &str, word: &str) -> Option<usize> {", ""),
        function(
            EXEC_SQL,
            "    pub(super) fn exec_sql(&mut self, text: &str) -> Result<()> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn sql_statement(&mut self, sql: &str) -> Result<Outcome> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn sql_ddl(&mut self, sql: &str) -> Result<Option<Outcome>> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn sql_change(&mut self, text: &str) -> Result<Outcome> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn run_sql(&mut self, sql: &str) -> Result<Exec> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn sql_overrides(&mut self, sql: &str) -> Result<()> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn session(&mut self) -> Result<&Session> {",
            "    ",
        ),
        function(EXEC_SQL, "pub(crate) fn sqlstate(code: i64) -> &'static str {", ""),
        // S2.70: host variables.
        function(
            EXEC_SQL,
            "fn host_list(text: &str) -> Result<Vec<(String, Option<String>)>> {",
            "",
        ),
        function(EXEC_SQL, "fn literal(v: &Value) -> Result<String> {", ""),
        function(
            EXEC_SQL,
            "    fn host_literal(&mut self, name: &str, ind: Option<&str>) -> Result<String> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn host_targets(&mut self, name: &str) -> Result<Vec<Expr>> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn bind_host(&mut self, sql: &str) -> Result<String> {",
            "    ",
        ),
        function(EXEC_SQL, "    fn assign_hosts(\n", "    "),
        // S2.72: cursors.
        function(
            EXEC_SQL,
            "    fn sql_open(&mut self, sql: &str) -> Result<Outcome> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn sql_fetch(&mut self, sql: &str) -> Result<Outcome> {",
            "    ",
        ),
        function(EXEC_SQL, "fn strip_cursor_clauses(query: &str) -> String {", ""),
        function(EXEC_SQL, "fn with_rrn(query: &str) -> Option<String> {", ""),
        // S2.78: EXCEPT to a printer file.
        function(
            INTERP,
            "    fn conditions_met(&self, cond: &Conditions) -> bool {",
            "    ",
        ),
        function(
            INTERP,
            "    fn out_cond(&self, groups: &[Conditions]) -> bool {",
            "    ",
        ),
        function(
            INTERP,
            "    fn except(&mut self, name: Option<&str>) -> Result<()> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn overflow_line(&self, r: &OutRec) -> bool {",
            "    ",
        ),
        function(
            INTERP,
            "    pub(super) fn overflow_routine(&mut self, only: Option<&str>) -> Result<()> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn overflow_ind(&self, file: &str) -> Option<String> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn out_text(&mut self, f: &OutField, page: i32) -> Result<String> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn print_record(&mut self, r: &OutRec) -> Result<()> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn print_line(&mut self, r: &OutRec) -> Result<()> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn flush_printer(&mut self, file: &str) -> Result<()> {",
            "    ",
        ),
        function(
            PRINTER,
            "    pub(super) fn form_of(&self, i: usize) -> (i32, i32) {",
            "    ",
        ),
        function(
            PRINTER,
            "    pub(super) fn ensure_printer(&mut self, file: &str) -> Result<()> {",
            "    ",
        ),
        // S2.86: fixed-form SORTA and MOVEA.
        function(
            OPS,
            "    pub(super) fn sorta(&mut self, a: &Expr) -> Result<()> {",
            "    ",
        ),
        function(
            OPS,
            "    fn movea_parts(&mut self, e: &Expr) -> Result<(Vec<Vec<u8>>, Option<ArrayRef>, usize)> {",
            "    ",
        ),
        function(
            OPS,
            "    fn movea(&mut self, from: &Expr, to: &Expr, pad: bool) -> Result<()> {",
            "    ",
        ),
        // S2.85: fixed-form LOOKUP and XFOOT.
        function(
            OPS,
            "    pub(super) fn array_op(\n",
            "    ",
        ),
        function(
            OPS,
            "    fn lookup(\n",
            "    ",
        ),
        function(
            INTERP,
            "    fn num_inds(&mut self, ind: &ResultInd, d: &Dec) {",
            "    ",
        ),
        // S2.84: fixed-form bit and test operations.
        function(
            OPS,
            "    pub(super) fn bit_op(\n",
            "    ",
        ),
        function(
            OPS,
            "    fn bit_mask(&mut self, e: &Expr) -> Result<u8> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn set_result_inds(&mut self, ind: &ResultInd, hi: bool, lo: bool, eq: bool) {",
            "    ",
        ),
        // S2.83: fixed-form string operations.
        function(
            OPS,
            "fn split(e: &Option<Expr>) -> (Option<Expr>, Option<Expr>) {",
            "",
        ),
        function(
            OPS,
            "    fn put_positions(&mut self, result: &Option<Expr>, found: &[usize]) -> Result<()> {",
            "    ",
        ),
        function(
            OPS,
            "    fn put_left(&mut self, result: &Expr, text: &[u8], pad: bool) -> Result<()> {",
            "    ",
        ),
        function(
            OPS,
            "    fn op_error(&mut self, ind: &ResultInd, msg: String) -> Result<()> {",
            "    ",
        ),
        function(
            OPS,
            "    pub(super) fn string_op(\n",
            "    ",
        ),
        // S2.81: %EDITC's third argument (in bif's EDITC arm).
        function(
            INTERP,
            "    fn bif(&mut self, b: &str, args: &[Expr]) -> Result<Value> {",
            "    ",
        ),
        // S2.80: IN and OUT of data areas.
        function(
            INTERP,
            "    fn data_area(&mut self, input: bool, lock: bool, field: &str) -> Result<()> {",
            "    ",
        ),
        function(
            INTERP,
            "    fn dtaara_unlock_object(&mut self, library: &str, name: &str) {",
            "    ",
        ),
        function(
            INTERP,
            "    fn local_data_area(&mut self, input: bool, field: &str) -> Result<()> {",
            "    ",
        ),
        // S2.79: WRITE, OPEN, CLOSE and FEOD of printer files.
        function(
            PRINTER,
            "    pub(super) fn printer_op(&mut self, op: FileOp, i: usize, name: &str) -> Result<()> {",
            "    ",
        ),
        function(
            PRINTER,
            "    fn printer_write(&mut self, i: usize, fmt: &str) -> Result<()> {",
            "    ",
        ),
        function(
            PRINTER,
            "    fn printer_text(&mut self, f: &PField, page: i32) -> Result<String> {",
            "    ",
        ),
        function(
            PRINTER,
            "    pub(super) fn print_ops(&mut self, i: usize, file: &str, ops: &[PrintOp]) -> Result<()> {",
            "    ",
        ),
        // S2.75: CALL.
        function(
            EXEC_SQL,
            "    fn sql_call(&mut self, sql: &str) -> Result<Outcome> {",
            "    ",
        ),
        // S2.74: multiple-row FETCH and INSERT.
        function(EXEC_SQL, "    fn fetch_rows(\n", "    "),
        function(
            EXEC_SQL,
            "    fn sql_insert_rows(&mut self, sql: &str) -> Result<Outcome> {",
            "    ",
        ),
        // S2.73: UPDATE and DELETE WHERE CURRENT OF.
        function(
            EXEC_SQL,
            "    fn positioned_change(&mut self, sql: &str, w: &[String]) -> Result<Outcome> {",
            "    ",
        ),
        // S2.71: dynamic SQL and GET DIAGNOSTICS.
        function(
            EXEC_SQL,
            "    fn get_diagnostics(&mut self, sql: &str) -> Result<Outcome> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn statement_text(&mut self, from: &str) -> Result<String> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn using_values(&mut self, sql: &str) -> Result<Vec<String>> {",
            "    ",
        ),
        function(
            EXEC_SQL,
            "    fn single_row(&mut self, query: &str, hosts: &[(String, Option<String>)]) -> Result<Outcome> {",
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
    // S2.53 bif_override and temporal, S2.54 bif_more, temporal_arg and
    // special_arg, S2.55 date_arith and as_ts, S2.58 file_op,
    // file_failed, file_idx, opcode_name, file_op_inner, key_values,
    // fill_fields, file_status and move_input, S2.59 record_of,
    // record_change, rla_change and fref_of, S2.64 occurrence,
    // set_occurrence and occur_op, S2.69 exec_sql.rs's exec_sql,
    // sql_statement, sql_ddl, sql_change, run_sql, sql_overrides, session,
    // sql_error, words, find_word and sqlstate (exec_sql.rs's fields made
    // pub(super) for the native runtime, the user's authorization of
    // 2026-11-18), S2.70 host_list, literal, host_literal, host_targets,
    // bind_host, assign_hosts and single_row, S2.71 get_diagnostics,
    // statement_text and using_values, S2.72 sql_open, sql_fetch,
    // strip_cursor_clauses and with_rrn (OpenCursor and cursors made
    // pub(super), the user's second authorization of 2026-11-18), S2.73
    // positioned_change, S2.74 fetch_rows and sql_insert_rows, S2.75
    // sql_call, S2.78 conditions_met, out_cond, except, overflow_line,
    // overflow_routine, overflow_ind, out_text, print_record, print_line,
    // flush_printer and printer.rs form_of and ensure_printer, S2.79
    // printer.rs printer_op, printer_write, printer_text and print_ops,
    // S2.80 data_area, dtaara_unlock_object and local_data_area, S2.81
    // bif (its EDITC arm), S2.83 ops.rs split, put_positions, put_left,
    // op_error and string_op, S2.84 ops.rs bit_op and bit_mask and
    // interp.rs set_result_inds, S2.85 ops.rs array_op and lookup and
    // interp.rs num_inds, S2.86 ops.rs sorta, movea_parts and movea;
    // upstream still dd47e28.
    let reviewed = "6b96b57d8ab49801c1a41b6555ab51358e349b35abc80076fff0846811aa30ba";
    assert_eq!(
        hash, reviewed,
        "run_code_at or the error functions changed: review native_rt.rs, then record the new hash"
    );
}
