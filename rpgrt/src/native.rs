//! The interface between generated code and the engine that runs it
//! (docs/native/adr/0001, 0003).
//!
//! Stage 1: a generated routine is the control flow of one compiled
//! routine (`code::Code` in as4tainr-core), a `loop { match pc { … } }`
//! whose arms call a [`Runtime`] for each statement, each expression an
//! instruction evaluates, the conditioning-indicator test and the error
//! routing. as4tainr-core's interpreter implements [`Runtime`] with the
//! code it runs itself, so both engines share one set of semantics.
//!
//! Every method takes the program counter of the instruction it serves:
//! the runtime reads that instruction (its statement, its expressions) from
//! the routine it was given.

use crate::rpg::data::{self, DType, Fit, Value};
use crate::rpg::dec::Dec;
use crate::rpg::mem::Mem;

/// What every runtime call returns: errors are the interpreter's
/// (RNQ message IDs, control signals) and pass through unchanged.
pub type NResult<T> = anyhow::Result<T>;

/// How a routine ended (the interpreter's `Flow`).
#[derive(Debug)]
pub enum Flow {
    Normal,
    Return(Option<Value>),
    LeaveSr,
}

/// A generated routine: the runtime, and the instruction to start at
/// (0, or a TAG after a GOTO from an error subroutine).
pub type RoutineFn = fn(&mut dyn Runtime, usize) -> NResult<Flow>;

/// A MONITOR group entered: the MONITOR instruction (its handlers are
/// read from it), the end of its body (the first ON-ERROR) and its end
/// (after ENDMON).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MonitorRef {
    pub at: usize,
    pub end_body: usize,
    pub end: usize,
}

/// The MONITOR groups a routine is in, innermost last.
#[derive(Debug, Default)]
pub struct MonitorStack {
    groups: Vec<MonitorRef>,
}

impl MonitorStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// MONITOR: the group starts.
    pub fn push(&mut self, group: MonitorRef) {
        self.groups.push(group);
    }

    /// ON-ERROR or ENDMON: the innermost group ends.
    pub fn pop(&mut self) -> Option<MonitorRef> {
        self.groups.pop()
    }

    /// The innermost group.
    pub fn last(&self) -> Option<&MonitorRef> {
        self.groups.last()
    }

    /// After a GOTO from a subroutine to a TAG at `to`: only the groups
    /// whose body goes past it are still entered.
    pub fn retain_past(&mut self, to: usize) {
        self.groups.retain(|g| to < g.end_body);
    }

    pub fn len(&self) -> usize {
        self.groups.len()
    }

    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

/// Where a routine goes after an instruction and its error routing.
#[derive(Debug)]
pub enum Settled {
    /// On at this instruction (an index past the end ends the routine).
    Go(usize),
    /// The routine ends: how, or with the error no MONITOR group took.
    Exit(NResult<Flow>),
}

/// What generated code needs from the engine. Each method serves the
/// instruction at `pc` of the routine the runtime was given.
pub trait Runtime {
    /// The instruction's conditioning indicators are met (true when it
    /// has none).
    fn cond(&mut self, pc: usize) -> bool;
    /// The instruction starts: its source line is the current one.
    fn line(&mut self, pc: usize);
    /// Run a statement.
    fn stmt(&mut self, pc: usize) -> NResult<()>;
    /// The condition of a jump-if-not is true.
    fn truth(&mut self, pc: usize) -> NResult<bool>;
    /// FOR or DO: the index is past its limit.
    fn loop_past(&mut self, pc: usize) -> NResult<bool>;
    /// FOR or DO: step the index.
    fn loop_step(&mut self, pc: usize) -> NResult<()>;
    /// FOR-EACH: take the list's values.
    fn each_init(&mut self, pc: usize) -> NResult<()>;
    /// FOR-EACH: the next value is in the item (false: there is none).
    fn each_next(&mut self, pc: usize) -> NResult<bool>;
    /// EXSR.
    fn exsr(&mut self, pc: usize) -> NResult<()>;
    /// RETURN: always an error, the return signal or the value's error.
    fn ret(&mut self, pc: usize) -> NResult<()>;
    /// LEAVESR: always an error, the signal.
    fn leave_sr(&mut self, pc: usize) -> NResult<()>;
    /// CABxx: the comparison, its resulting indicators; true when it
    /// branches to a TAG in this routine (a TAG outside it is an error,
    /// the GOTO signal).
    fn cab(&mut self, pc: usize) -> NResult<bool>;
    /// GOTO out of a subroutine: always an error, the GOTO signal.
    fn goto_out(&mut self, pc: usize) -> NResult<()>;
    /// ENDSR with a return point: an error (the signal) when the point is
    /// set.
    fn end_sr(&mut self, pc: usize) -> NResult<()>;
    /// After the instruction at `pc`: its result (Ok: the next
    /// instruction), routed as the interpreter routes it (pointer faults,
    /// control signals, MONITOR groups, the source line).
    fn settle(&mut self, pc: usize, step: NResult<usize>, monitors: &mut MonitorStack) -> Settled;
    /// Stage 2 (docs/native/adr/0006): lowered code may run, because the
    /// routine runs in its own context: a main-program routine with no
    /// procedure frame, or a procedure's routine under its own frame (S2.8).
    /// Otherwise names may resolve elsewhere, and the interpreter does it.
    fn native_ok(&self) -> bool;
    /// Stage 2 (S2.8): where a procedure local at frame buffer `buf`,
    /// offset `off`, is in this call: the caller's storage for a parameter
    /// passed by reference, else the frame's own buffer (as the
    /// interpreter's `sym`), and an error where `check_buf` gives one.
    fn local(&mut self, buf: usize, off: usize) -> NResult<(usize, usize)>;
    /// Stage 2: the program's storage, for lowered code.
    fn mem(&mut self) -> &mut Mem;
    /// Stage 2 (S2.2): the digits and decimal places the precision rules
    /// give the arithmetic node at `path` (0 left, 1 right) of the value of
    /// the EVAL at `pc`, as the interpreter works them out.
    fn precision(&mut self, pc: usize, path: &[u8]) -> Option<(u32, u32)>;
    /// Stage 2 (S2.3): the resulting indicators of the fixed-form
    /// arithmetic operation at `pc` for the value its result field now
    /// holds, as the interpreter sets them.
    fn num_inds(&mut self, pc: usize, stored: &Dec);
    /// Stage 2 (S2.7): the remainder of the last fixed-form DIV, which MVR
    /// moves.
    fn remainder(&self) -> Dec;
    /// Stage 2 (S2.7): DIV keeps its remainder.
    fn set_remainder(&mut self, d: Dec);
    /// Stage 2 (S2.31): the most recent program or file status code, as
    /// %STATUS with no file gives it.
    fn status(&self) -> u32;
    /// Stage 2 (S2.35, ADR 0007): call the procedure `proc` of this module
    /// with its arguments in order, natively: the frame, the call stack
    /// entry and the procedure's own generated code, with no interpreter
    /// call. Its return value, if it has one.
    fn call(&mut self, pc: usize, proc: &str, args: Vec<CallArg>) -> NResult<Option<Value>>;
    /// Stage 2 (S2.36): DSPLY with no response field shows `text` (the
    /// interpreter's batch path: the job's messages, trailing blanks
    /// removed).
    fn dsply(&mut self, text: String);
    /// Stage 2 (S2.39): an indicator's value (`*INxx`, `*INLR`...).
    fn ind(&self, name: &str) -> bool;
    /// Stage 2 (S2.39): set an indicator.
    fn set_ind(&mut self, name: &str, on: bool);
    /// Stage 2 (S2.47): a pointer to storage at an absolute buffer and
    /// offset (the interpreter's `ptr_to`).
    fn addr_of(&self, buf: usize, off: usize) -> Value;
    /// Stage 2 (S2.47): a pointer field named `name` (with no index) was
    /// set to `v`: the storage it addresses grows to the size of the
    /// largest variable based on it (the interpreter's assign,
    /// `room_for_based`).
    fn pointer_set(&mut self, name: &str, v: &Value);
    /// Stage 2 (S2.48): where a based field is now: `off` bytes into the
    /// storage pointer `p` addresses, with `size` bytes there (the
    /// interpreter's `based_place`: MCH3601 for a pointer that addresses
    /// nothing, MCH0601 for storage too short).
    fn based_at(&mut self, p: &Value, off: usize, size: usize) -> NResult<(usize, usize)>;
    /// Stage 2 (S2.50): %ALLOC and ALLOC: new heap storage of `n` bytes
    /// (1 to 16 776 704, RNQ0425 otherwise), a pointer to it.
    fn alloc(&mut self, n: &Dec) -> NResult<Value>;
    /// Stage 2 (S2.50): %REALLOC and REALLOC: the heap storage `p`
    /// addresses made `n` bytes, its bytes kept (RNQ0426 for a pointer
    /// that is not to the start of heap storage).
    fn realloc(&mut self, p: &Value, n: &Dec) -> NResult<Value>;
    /// Stage 2 (S2.50): DEALLOC: the heap storage `p` addresses freed
    /// (*NULL is nothing; RNQ0426 for storage not from the heap or
    /// freed already).
    fn dealloc(&mut self, p: &Value) -> NResult<()>;
    /// Stage 2 (S2.45): the elements in use of a varying-dimension array.
    fn elems(&self, name: &str) -> u32;
    /// Stage 2 (S2.45): set them.
    fn set_elem_count(&mut self, name: &str, n: u32);
    /// Stage 2 (S2.58): the database file read at `pc` (CHAIN, READ,
    /// READE, READP, READPE, SETLL, SETGT) with its key's values: the
    /// interpreter's file_op up to the moves of the record into the
    /// program. The record's fields as external text, by their place in
    /// the record format (`None` for one the record does not have), when
    /// a record was read. An error of the operation comes back with the
    /// file's status, the counters and the file information data
    /// structure already set, as file_op leaves them.
    fn file_read(
        &mut self,
        pc: usize,
        keys: Option<Vec<KeyArg>>,
    ) -> NResult<Option<Vec<Option<String>>>>;
    /// Stage 2 (S2.58): the rest of the file operation at `pc` once its
    /// fields are moved (`moved`, or the error of a move): the status,
    /// %EOF/%FOUND/%EQUAL, the result indicators, the counters and the
    /// file information data structure.
    fn file_done(&mut self, pc: usize, moved: NResult<()>) -> NResult<()>;
    /// Stage 2 (S2.59): the start of the WRITE, UPDATE or DELETE at `pc`
    /// (errors.rs file_op's start, file_op_inner's open check and the
    /// authority check), before the record is made from the program's
    /// fields. An error comes back with file_op's end done.
    fn file_change_start(&mut self, pc: usize) -> NResult<()>;
    /// Stage 2 (S2.59): the change itself, with the record's fields as
    /// external text by their place in the record format (or the error of
    /// making them) and a DELETE's key, then file_op's end.
    fn file_change(
        &mut self,
        pc: usize,
        keys: Option<Vec<KeyArg>>,
        rec: NResult<Vec<Option<String>>>,
    ) -> NResult<()>;
    /// Stage 2 (S2.69): run the EXEC SQL at `pc` (exec_sql.rs exec_sql up
    /// to the SQLCA), its outcome for generated code to store.
    fn exec_sql(&mut self, pc: usize) -> NResult<SqlResult>;
    /// Stage 2 (S2.70): run the EXEC SQL with host variables at `pc`: its
    /// text from `pieces` with the input literals between them
    /// (bind_host), then, for SELECT/VALUES INTO or SET (`into`), the row
    /// into the `outs` (single_row, assign_hosts), else the change
    /// (sql_change); the outcome for generated code to store.
    fn exec_sql_hosts(
        &mut self,
        pc: usize,
        pieces: &[&str],
        lits: Vec<String>,
        outs: Vec<SqlSlot>,
        into: bool,
    ) -> NResult<SqlResult>;
    /// Stage 2 (S2.64): the current occurrence of the multiple-occurrence
    /// data structure in buffer `buf` (ops.rs occurrence).
    fn occurrence(&self, buf: usize) -> NResult<Dec>;
    /// Stage 2 (S2.64): make occurrence `n` of that data structure the
    /// current one (ops.rs set_occurrence: a whole number, 1 to its
    /// occurrences, RNQ0122 otherwise).
    fn set_occurrence(&mut self, buf: usize, n: &Dec) -> NResult<()>;
    /// Stage 2 (S2.58): %EOF, %FOUND or %EQUAL of the file named (as the
    /// running procedure finds it), or of the last file operation.
    fn file_status(&mut self, bit: FileBit, file: Option<&str>) -> NResult<bool>;
}

/// How an EXEC SQL ended (S2.69): SQLCODE, SQLSTATE, SQLERRMC's text,
/// SQLERRD(3) and SQLERRD(5).
#[derive(Clone, Debug)]
pub struct SqlResult {
    pub code: i64,
    pub state: String,
    pub text: String,
    pub rows: i64,
    pub errd5: i64,
}

/// An INTO target (S2.70): its storage and type, whether it is numeric,
/// and its indicator variable's storage and type.
#[derive(Clone, Debug)]
pub struct SqlSlot {
    pub buf: usize,
    pub off: usize,
    pub ty: DType,
    pub numeric: bool,
    pub ind: Option<(usize, usize, DType)>,
}

/// An input host variable's value as an SQL literal (S2.70), as
/// exec_sql.rs host_literal and literal make it: a number as its text, an
/// indicator as '1' or '0', a figurative constant as '', characters
/// quoted (null bytes dropped, quotes doubled), and a fixed character
/// field of length 1 to 10 485 760 as CAST(... AS char(n)).
pub fn sql_literal(v: &Value, char_len: Option<u32>) -> NResult<String> {
    let lit = match v {
        Value::Num(d) => d.to_string(),
        Value::Ind(b) => if *b { "'1'" } else { "'0'" }.to_string(),
        Value::Fig(_) => "''".to_string(),
        Value::Char(c) => format!("'{}'", data::a(c).replace('\0', "").replace('\'', "''")),
        other => anyhow::bail!("{} cannot be an SQL value", other.describe()),
    };
    Ok(match (char_len, v) {
        (Some(n), Value::Char(_)) if (1..=10_485_760).contains(&n) => {
            format!("CAST({lit} AS char({n}))")
        }
        _ => lit,
    })
}

/// A key value of a file operation (S2.58), as the interpreter's
/// key_values evaluates it: a number, or characters.
#[derive(Clone, Debug)]
pub enum KeyArg {
    Num(Dec),
    Char(Vec<u8>),
}

/// %EOF, %FOUND or %EQUAL (S2.58).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileBit {
    Eof,
    Found,
    Equal,
}

/// How a field read becomes a value (S2.58).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveKind {
    Num,
    Char,
    Hex,
}

/// A program field's value as the external text of a record field, as the
/// interpreter's record_of makes it (S2.59): a number with the field's
/// decimal places (an error if it does not fit them), hexadecimal bytes
/// as two capital digits each, characters as text.
pub fn field_text(v: &Value, kind: MoveKind, decimals: u8) -> NResult<String> {
    Ok(match kind {
        MoveKind::Num => v.num()?.rescale(decimals, false)?.to_string(),
        MoveKind::Hex => v.chars()?.iter().map(|b| format!("{b:02X}")).collect(),
        MoveKind::Char => data::a(&v.chars()?),
    })
}

/// A field of a record read as the value the program gets, as the
/// interpreter's fill_fields makes it: a number parsed exactly, the text
/// as characters, or for a hexadecimal field two digits a byte (a pair
/// that is not two hexadecimal digits gives 0, an odd last digit is left
/// out).
pub fn field_value(text: &str, kind: MoveKind) -> NResult<Value> {
    Ok(match kind {
        MoveKind::Num => Value::Num(Dec::parse(text)?),
        MoveKind::Char => Value::str(text),
        MoveKind::Hex => Value::Char(
            text.as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| {
                    std::str::from_utf8(pair.as_slice())
                        .ok()
                        .and_then(|d| u8::from_str_radix(d, 16).ok())
                        .unwrap_or(0)
                })
                .collect(),
        ),
    })
}

/// An argument of a native procedure call (S2.35): the caller's storage
/// for a parameter passed by reference (an absolute buffer and offset), or
/// a value for a VALUE or CONST parameter.
#[derive(Clone, Debug)]
pub enum CallArg {
    /// By reference: the storage, and the argument's own type (S2.51:
    /// as the interpreter's loc_of gives it, for OPTIONS(*VARSIZE)).
    Ref {
        buf: usize,
        off: usize,
        dtype: crate::rpg::data::DType,
    },
    Val(Value),
}

/// A sum, difference or product with the decimal places the precision
/// rules give it, when known; extra places cut off (the interpreter's
/// `to_rule_scale`).
pub fn rule_scale(p: Option<(u32, u32)>, v: Dec) -> Dec {
    match p {
        Some((_, dr)) if dr <= 37 => u8::try_from(dr)
            .ok()
            .and_then(|d| v.rescale(d, false).ok())
            .unwrap_or(v),
        _ => v,
    }
}

/// A comparison operator of lowered code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// The first step of a path into an EVAL's target instead of its value
/// (S2.44: %SUBST's start and length as a target).
pub const TARGET_PATH: u8 = 255;

/// %SUBST as the target of an EVAL (S2.44): `cur` (the field's current
/// characters) with positions `start` to `start + len - 1` replaced by
/// `val` stored as a character field of that length (padded or cut by
/// `fit` and `right`); with no `len`, to the end. The interpreter's
/// assign_value "SUBST" arm: a start of 0, or a part past the end, is
/// RNQ0100; a start or length that is negative or past the end (which
/// the interpreter reaches by wrapping an unsigned conversion) is the same
/// error here, never a panic.
pub fn subst_target(
    mut cur: Vec<u8>,
    start: &Dec,
    len: Option<&Dec>,
    val: &Value,
    fit: Fit,
    right: bool,
) -> NResult<Vec<u8>> {
    let outside = || anyhow::anyhow!("RNQ0100: %SUBST is outside the field.");
    let start = usize::try_from(start.to_i128()?).map_err(|_| outside())?;
    let first = start.checked_sub(1).ok_or_else(outside)?;
    let len = match len {
        Some(l) => usize::try_from(l.to_i128()?).map_err(|_| outside())?,
        None => cur
            .len()
            .checked_add(1)
            .and_then(|n| n.checked_sub(start))
            .ok_or_else(outside)?,
    };
    let end = first.checked_add(len).ok_or_else(outside)?;
    if end > cur.len() {
        return Err(outside());
    }
    let mut piece = vec![data::BLANK; len];
    data::store(
        &DType::Char(u32::try_from(len)?),
        val,
        &mut piece,
        fit,
        right,
    )?;
    cur.get_mut(first..end)
        .ok_or_else(outside)?
        .copy_from_slice(&piece);
    Ok(cur)
}

/// A path to an expression node written as runs of equal steps (a long
/// chain's left operands, S2.41): the steps. Its length is bounded where
/// the runs are written, by lowering's limit on an instruction's nodes
/// (`lower::MAX_NODES`); nothing is cut here, since a shorter path would
/// name another node.
pub fn path_rle(runs: &[(u8, usize)]) -> Vec<u8> {
    let mut out = Vec::new();
    for &(step, n) in runs {
        out.extend(std::iter::repeat_n(step, n));
    }
    out
}

/// A buffer of storage as the interpreter's `bufs[i]` gives it. `Mem`'s
/// `Index` does not panic: a buffer that does not exist (a pointer fault)
/// reads as a blank scratch area, the interpreter's behaviour, which
/// `Mem::get` (an Option) would not keep.
pub fn storage(mem: &Mem, buf: usize) -> &Vec<u8> {
    std::ops::Index::index(mem, buf)
}

/// A field's value, as the interpreter reads a standalone field with no
/// index (`load_var`: `data::load` over its bytes; a parameter longer than
/// the storage passed for it, OPTIONS(*VARSIZE), reads what is there and
/// then blanks).
pub fn load(mem: &Mem, buf: usize, off: usize, t: DType) -> NResult<Value> {
    let b = storage(mem, buf);
    let n = t.size();
    match off.checked_add(n) {
        Some(end) if end <= b.len() => data::load(
            &t,
            b.get(off..end)
                .ok_or_else(|| anyhow::anyhow!("native: a field past the end of its storage"))?,
        ),
        _ => {
            let mut tmp = b.get(off..).unwrap_or(&[]).to_vec();
            tmp.resize(n, 0x40);
            data::load(&t, &tmp)
        }
    }
}

/// A field's number (`load` then `Value::num`).
pub fn load_num(mem: &Mem, buf: usize, off: usize, t: DType) -> NResult<Dec> {
    load(mem, buf, off, t)?.num()
}

/// Assign to a field as the interpreter assigns to a standalone field
/// with no index (`store_var`: `data::store` into its bytes).
pub fn store(
    mem: &mut Mem,
    buf: usize,
    off: usize,
    t: DType,
    v: &Value,
    fit: Fit,
    right: bool,
) -> NResult<()> {
    let n = t.size();
    let end = off
        .checked_add(n)
        .ok_or_else(|| anyhow::anyhow!("native: a field past the end of storage"))?;
    let have = storage(mem, buf).len();
    if end > have {
        // A parameter longer than its argument (OPTIONS(*VARSIZE)): only
        // the argument's storage changes (the interpreter's store_var).
        let mut tmp = vec![0x40u8; n];
        data::store(&t, v, &mut tmp, fit, right)?;
        let k = have.saturating_sub(off);
        if let (Some(dst), Some(src)) = (
            mem.get_mut(buf)
                .and_then(|b| b.get_mut(off.min(have)..have)),
            tmp.get(..k),
        ) {
            dst.copy_from_slice(src);
        }
        return Ok(());
    }
    let out = mem
        .get_mut(buf)
        .and_then(|b| b.get_mut(off..end))
        .ok_or_else(|| anyhow::anyhow!("native: a field past the end of its storage"))?;
    data::store(&t, v, out, fit, right)
}

/// Compare as the interpreter compares two non-float operands
/// (`data::compare`, then the operator).
pub fn cmp(op: CmpOp, x: &Value, y: &Value) -> NResult<bool> {
    use std::cmp::Ordering::*;
    let o = data::compare(x, y)?;
    Ok(match op {
        CmpOp::Eq => o == Equal,
        CmpOp::Ne => o != Equal,
        CmpOp::Lt => o == Less,
        CmpOp::Le => o != Greater,
        CmpOp::Gt => o == Greater,
        CmpOp::Ge => o != Less,
    })
}

/// %TRIM, %TRIML and %TRIMR with no second argument (S2.9): the value's
/// characters without leading (`left`) and trailing (`right`) blanks, as
/// the interpreter computes them.
pub fn trim(v: &Value, left: bool, right: bool) -> NResult<Value> {
    let s = v.chars()?;
    let lo = if left {
        s.iter().take_while(|&&c| c == data::BLANK).count()
    } else {
        0
    };
    let rest = s.get(lo..).unwrap_or_default();
    let keep = if right {
        rest.len()
            .saturating_sub(rest.iter().rev().take_while(|&&c| c == data::BLANK).count())
    } else {
        rest.len()
    };
    Ok(Value::Char(rest.get(..keep).unwrap_or_default().to_vec()))
}

/// %SUBST's start (S2.10): the index of its first character, or the
/// interpreter's error when it is outside the string (start 1 to its
/// length; upstream #9).
pub fn subst_start(s: &[u8], start: &Value) -> NResult<usize> {
    let start = start.num()?.to_i128()?;
    let from = usize::try_from(start)
        .ok()
        .filter(|&n| n >= 1 && n <= s.len())
        .and_then(|n| n.checked_sub(1));
    from.ok_or_else(|| {
        anyhow::anyhow!(
            "RNQ0100: %SUBST start {start} is outside the string (length {}).",
            s.len()
        )
    })
}

/// %SUBST's characters from `from` (S2.10): `len` of them, or the rest of
/// the string; the interpreter's errors for a negative length or one
/// past the end.
pub fn subst_take(s: &[u8], from: usize, len: Option<&Value>) -> NResult<Value> {
    let len = match len {
        Some(x) => {
            let n = x.num()?.to_i128()?;
            if n < 0 {
                anyhow::bail!("RNQ0100: %SUBST length {n} is negative.");
            }
            usize::try_from(n).unwrap_or(usize::MAX)
        }
        None => s.len().saturating_sub(from),
    };
    match from.checked_add(len).and_then(|end| s.get(from..end)) {
        Some(part) => Ok(Value::Char(part.to_vec())),
        None => anyhow::bail!(
            "RNQ0100: %SUBST length {len} goes past the end of the string (length {}).",
            s.len()
        ),
    }
}

/// MOVE's and MOVEL's image of a value (S2.11), as the interpreter's
/// `move_image`: a number as zoned digits of its type's precision (a
/// literal's own digits when `t` is not numeric), anything else as its
/// characters.
pub fn move_image(v: &Value, t: Option<DType>) -> NResult<Vec<u8>> {
    match v {
        Value::Num(d) => {
            let (digits, decs) = match t {
                Some(t) if t.is_numeric() => t.precision(),
                _ => (
                    d.int_digits().saturating_add(u32::from(d.scale())).max(1),
                    u32::from(d.scale()),
                ),
            };
            let zt = DType::Zoned(digits, decs);
            let mut b = zt.initial();
            data::store(
                &zt,
                &Value::Num(*d),
                &mut b,
                Fit::Truncate { half_adjust: false },
                false,
            )?;
            Ok(b)
        }
        other => other.chars(),
    }
}

/// MOVE (`left` false) and MOVEL (S2.11): `src` over the result's image
/// `dst`, from the right or the left, after padding it when `pad`, and the
/// value to assign (the interpreter's `do_move`). A numeric result takes
/// the digits of each byte and the sign from the zone of the last.
pub fn move_into(left: bool, pad: bool, src: &[u8], mut dst: Vec<u8>, tt: DType) -> NResult<Value> {
    let n = dst.len();
    if pad {
        let fill = if tt.is_numeric() { 0xF0 } else { data::BLANK };
        dst.fill(fill);
    }
    let k = src.len().min(n);
    let (to, from) = if left {
        (dst.get_mut(..k), src.get(..k))
    } else {
        (
            dst.get_mut(n.saturating_sub(k)..),
            src.get(src.len().saturating_sub(k)..),
        )
    };
    match (to, from) {
        (Some(to), Some(from)) if to.len() == from.len() => to.copy_from_slice(from),
        _ => anyhow::bail!("MOVE: the operands' lengths do not fit"),
    }
    if !tt.is_numeric() {
        return Ok(Value::Char(dst));
    }
    let (_, decs) = tt.precision();
    let mut digits: Vec<u8> = dst.iter().map(|b| 0xF0 | (b & 0x0F).min(9)).collect();
    if let (Some(last), Some(d)) = (dst.last(), digits.last_mut()) {
        let neg = last & 0xF0 == 0xD0;
        *d = if neg { 0xD0 } else { 0xF0 } | (last & 0x0F).min(9);
    }
    let len = u32::try_from(digits.len())?;
    Ok(Value::Num(data::decode_num(
        &DType::Zoned(len, decs),
        &digits,
    )?))
}

/// %SCAN (S2.12): the position of the first `pat` in `s` from `start`
/// (1 when not given) on, or 0, as the interpreter computes it.
pub fn scan(pat: &[u8], s: &[u8], start: Option<&Value>) -> NResult<Dec> {
    let start = match start {
        Some(x) => x.num()?.to_i128()?,
        None => 1,
    };
    // The interpreter's check (upstream #9): a start from 1 to the
    // string's length; an empty string allows any start of 1 or more.
    let len = i128::try_from(s.len())?;
    if start < 1 || !s.is_empty() && start > len {
        anyhow::bail!(
            "RNQ0100: %SCAN start {start} is outside the string (length {}).",
            s.len()
        );
    }
    let found = match usize::try_from(start) {
        Ok(st) if !pat.is_empty() && st <= s.len() => st
            .checked_sub(1)
            .and_then(|from| s.get(from..))
            .and_then(|rest| rest.windows(pat.len()).position(|w| w == pat))
            .and_then(|p| p.checked_add(st))
            .unwrap_or(0),
        _ => 0,
    };
    Ok(Dec::from_i64(i64::try_from(found)?))
}

/// %XFOOT, %MAXARR or %MINARR of a whole array (S2.68), the
/// interpreter's arms: the sum of the elements' numbers; or the index of
/// the first greatest, or smallest, element (data::compare), 0 for none.
pub fn arr_fold(all: &[Value], kind: &str) -> NResult<Dec> {
    use std::cmp::Ordering;
    if kind == "XFOOT" {
        let mut sum = Dec::ZERO;
        for v in all {
            sum = sum.add(&v.num()?)?;
        }
        return Ok(sum);
    }
    let want = if kind == "MAXARR" {
        Ordering::Greater
    } else {
        Ordering::Less
    };
    let mut best: Option<(usize, &Value)> = None;
    for (k, v) in all.iter().enumerate() {
        let better = match &best {
            None => true,
            Some((_, bv)) => data::compare(v, bv)? == want,
        };
        if better {
            best = Some((
                k.checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("%{kind}: index"))?,
                v,
            ));
        }
    }
    Ok(Dec::from_i64(i64::try_from(best.map_or(0, |b| b.0))?))
}

/// %SCANR (S2.67): the position of the last occurrence of `pat` in `s`
/// from `start` (1 if none) for `len` bytes (to the end if none), 0 if it
/// is not there or `pat` is empty; the interpreter's "SCANR" arm. A start
/// below 1, or a start and length past the end, is RNQ0100, as there. A
/// start more than one past the end with no length (where the
/// interpreter's default length subtracts past zero, a panic in a debug
/// build) is RNQ0100 too (fidelity.md).
pub fn scanr(pat: &[u8], s: &[u8], start: Option<&Value>, len: Option<&Value>) -> NResult<Dec> {
    let out = || anyhow::anyhow!("RNQ0100: %SCANR start or length is outside the string.");
    let total = i128::try_from(s.len())?;
    let start = match start {
        Some(x) => x.num()?.to_i128()?,
        None => 1,
    };
    let len = match len {
        Some(x) => x.num()?.to_i128()?,
        None => total
            .checked_add(1)
            .and_then(|t| t.checked_sub(start.max(1)))
            .ok_or_else(out)?,
    };
    if start < 1 || len < 0 {
        return Err(out());
    }
    let end = start
        .checked_sub(1)
        .and_then(|b| b.checked_add(len))
        .ok_or_else(out)?;
    if end > total {
        return Err(out());
    }
    let from = usize::try_from(start.checked_sub(1).ok_or_else(out)?)?;
    let to = usize::try_from(end)?;
    let hay = s.get(from..to).ok_or_else(out)?;
    let found = if pat.is_empty() || pat.len() > hay.len() {
        0
    } else {
        hay.windows(pat.len())
            .rposition(|w| w == pat)
            .and_then(|p| i128::try_from(p).ok())
            .and_then(|p| p.checked_add(start))
            .unwrap_or(0)
    };
    Ok(Dec::from_i64(i64::try_from(found)?))
}

/// %LEFT (`left`) or %RIGHT (S2.67): the first, or last, `n` bytes of `s`;
/// the interpreter's "LEFT" | "RIGHT" arm, RNQ0100 when `n` is longer than
/// the string (a negative `n` is, as its `as usize` makes it, a number past
/// any length).
pub fn left_right(s: &[u8], n: &Value, left: bool) -> NResult<Value> {
    let n = n.num()?.to_i128()?;
    let len = i128::try_from(s.len())?;
    if n < 0 || n > len {
        // The interpreter's message shows `n as usize`.
        let shown = if n < 0 {
            n.checked_add(1i128 << 64).unwrap_or(n)
        } else {
            n
        };
        anyhow::bail!(
            "RNQ0100: %{} length {shown} is longer than the string.",
            if left { "LEFT" } else { "RIGHT" }
        );
    }
    let k = usize::try_from(n)?;
    let part = if left {
        s.get(..k)
    } else {
        s.len().checked_sub(k).and_then(|f| s.get(f..))
    };
    Ok(Value::Char(
        part.ok_or_else(|| anyhow::anyhow!("native: %LEFT/%RIGHT"))?
            .to_vec(),
    ))
}

/// %CHECK (`rev` false) or %CHECKR with no start (S2.34): the position of
/// the first, or last, character of `s` that is not in `set`, 0 when every
/// one is; the interpreter's "CHECK" | "CHECKR" arm.
pub fn check(set: &[u8], s: &[u8], rev: bool) -> NResult<Dec> {
    let found = if rev {
        s.iter().rposition(|c| !set.contains(c))
    } else {
        s.iter().position(|c| !set.contains(c))
    };
    let pos = match found {
        Some(p) => p
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("RNQ0103: position overflow"))?,
        None => 0,
    };
    Ok(Dec::from_i64(i64::try_from(pos)?))
}

/// %XLATE (S2.12): `s` with each character of `from` that has a
/// counterpart in `to` changed to it, from position `start` (1 when not
/// given) on; the interpreter's error for a start outside the string.
pub fn xlate(from: &[u8], to: &[u8], s: &[u8], start: Option<&Value>) -> NResult<Value> {
    let start = match start {
        Some(x) => x.num()?.to_i128()?,
        None => 1,
    };
    let first = usize::try_from(start)
        .ok()
        .filter(|&n| n >= 1 && n <= s.len().max(1));
    let Some(first) = first else {
        anyhow::bail!("RNQ0100: %XLATE start position {start} is outside the string.");
    };
    Ok(Value::Char(
        s.iter()
            .enumerate()
            .map(|(k, c)| match from.iter().position(|f| f == c) {
                Some(p) if k >= first.saturating_sub(1) => to.get(p).copied().unwrap_or(*c),
                _ => *c,
            })
            .collect(),
    ))
}

/// %CHAR of a number (S2.14): the interpreter's `char_of_num_edit`, with
/// the program's DECEDIT (the decimal point, and a zero before it when
/// the value has no integer digits). No leading zeros, a leading '-' for
/// a negative value.
pub fn char_of_num(d: &Dec, point: char, zero: bool) -> Value {
    let s = d.abs().to_string();
    let s = if zero && s.starts_with("0.") && d.scale() > 0 {
        s.replacen('.', &point.to_string(), 1)
    } else {
        let t = if d.scale() > 0 {
            s.trim_start_matches('0').to_string()
        } else {
            s
        };
        t.replacen('.', &point.to_string(), 1)
    };
    let s = if s.is_empty() { "0".to_string() } else { s };
    if d.is_negative() {
        Value::str(&format!("-{s}"))
    } else {
        Value::str(&s)
    }
}

/// %DEC and %DECH with digits and decimal places (S2.14): the value with
/// those decimal places, truncated or half adjusted, and the
/// interpreter's error when it does not fit the digits.
pub fn dec_to(d: &Dec, digits: u32, decimals: u32, half: bool) -> NResult<Dec> {
    let r = d.rescale(u8::try_from(decimals)?, half)?;
    if !r.fits(digits, decimals) {
        anyhow::bail!(
            "RNQ0103: The target for a numeric operation is too small to hold the result."
        );
    }
    Ok(r)
}

/// `/` in an expression (S2.17): the interpreter's `divide`. The
/// quotient gets the decimal places of the ILE RPG precision rules,
/// Dr = M - ((L1 - D1) + D2), with M = 30, or 63 when an operand has
/// more than 30 digits, and at most 37. The operands' precision comes
/// from their nodes, or else from their own digits. When the quotient
/// does not fit, it gets fewer places. A zero divisor is MCH1211.
pub fn divide(a: &Dec, b: &Dec, pl: Option<(u32, u32)>, pr: Option<(u32, u32)>) -> NResult<Dec> {
    let own = |d: &Dec| {
        let s = u32::from(d.scale());
        (d.int_digits().saturating_add(s).max(1), s)
    };
    let (l1, d1) = pl.unwrap_or_else(|| own(a));
    let (l2, d2) = pr.unwrap_or_else(|| own(b));
    let m: u32 = if l1.max(l2) > 30 { 63 } else { 30 };
    let mut dr = m
        .saturating_sub(l1.saturating_sub(d1).saturating_add(d2))
        .min(37);
    loop {
        match a.div_scale(b, u8::try_from(dr)?) {
            Ok(q) => return Ok(q),
            Err(e) if dr == 0 || e.to_string().starts_with("MCH1211") => return Err(e),
            Err(_) => dr = dr.saturating_sub(1),
        }
    }
}

/// Where an array element is when its index is known only at run time
/// (S2.21): the interpreter's `elem_off`, an index from 1 to `dim`, else
/// its RNQ0121 error with the same text.
pub fn elem_at(off: usize, stride: usize, dim: u32, index: &Value) -> NResult<usize> {
    let i = index.num()?.to_i128()?;
    if i < 1 || i > i128::from(dim) {
        anyhow::bail!("RNQ0121: An array index is out of range (index {i}, {dim} elements).");
    }
    let k = usize::try_from(i)?.saturating_sub(1);
    k.checked_mul(stride)
        .and_then(|d| off.checked_add(d))
        .ok_or_else(|| {
            anyhow::anyhow!("RNQ0121: An array index is out of range (index {i}, {dim} elements).")
        })
}

/// An element of a varying-dimension array read (S2.45): the index must
/// be 1 to the elements in use (the interpreter's eval of an indexed
/// name), then within the dimension (`elem_at`).
pub fn elem_in_use(
    off: usize,
    stride: usize,
    dim: u32,
    index: &Value,
    in_use: u32,
) -> NResult<usize> {
    let i = index.num()?.to_i128()?;
    if i < 1 || i > i128::from(in_use) {
        anyhow::bail!(
            "RNQ0121: An array index is out of range (index {i}, {in_use} elements in use)."
        );
    }
    elem_at(off, stride, dim, index)
}

/// An element of a varying-dimension array written (S2.45): an index
/// past the elements in use grows a DIM(*AUTO) array to it, within the
/// dimension; otherwise it is the interpreter's RNQ0121. The element's
/// place, and the new number of elements in use when it grows.
pub fn elem_grow(
    off: usize,
    stride: usize,
    dim: u32,
    index: &Value,
    in_use: u32,
    auto: bool,
) -> NResult<(usize, Option<u32>)> {
    let k = index.num()?.to_i128()?;
    let mut grow = None;
    if k > i128::from(in_use) {
        if auto && k <= i128::from(dim) {
            grow = Some(u32::try_from(k)?);
        } else {
            anyhow::bail!(
                "RNQ0121: An array index is out of range (index {k}, {in_use} elements in use)."
            );
        }
    }
    Ok((elem_at(off, stride, dim, index)?, grow))
}

/// Where a global array is: its buffer, its first element's offset, the
/// distance between elements, its dimension and its elements' type.
#[derive(Clone, Copy, Debug)]
pub struct ArrayPlace {
    pub buf: usize,
    pub off: usize,
    pub stride: usize,
    pub dim: u32,
    pub ty: DType,
}

/// %ELEM of a varying-dimension array set (S2.45): the interpreter's
/// assign_value "ELEM" arm. A count outside 0 to the dimension is RNQ0121;
/// the elements from the ones in use to the new count get their initial
/// value. The new count.
pub fn set_elems(mem: &mut Mem, arr: &ArrayPlace, in_use: u32, v: &Dec) -> NResult<u32> {
    let n = v.to_i128()?;
    if n < 0 || n > i128::from(arr.dim) {
        anyhow::bail!("RNQ0121: %ELEM {n} is outside 0 to {}.", arr.dim);
    }
    let n = u32::try_from(n)?;
    let init = arr.ty.initial();
    let buf = arr.buf;
    for k in in_use..n {
        let at = usize::try_from(k)?
            .checked_mul(arr.stride)
            .and_then(|d| arr.off.checked_add(d))
            .ok_or_else(|| anyhow::anyhow!("native: an array element's place"))?;
        let end = at
            .checked_add(init.len())
            .ok_or_else(|| anyhow::anyhow!("native: an array element's place"))?;
        mem.get_mut(buf)
            .and_then(|b| b.get_mut(at..end))
            .ok_or_else(|| anyhow::anyhow!("native: an array element past its storage"))?
            .copy_from_slice(&init);
    }
    Ok(n)
}

/// %REPLACE (S2.23): `rep` in place of `len` characters of `s` from
/// `start` (1 and `rep`'s length when not given), as the interpreter
/// computes it: the start and length are cut to the string, so a start
/// past the end appends.
pub fn replace(rep: &[u8], s: &[u8], start: Option<&Value>, len: Option<&Value>) -> NResult<Value> {
    // The interpreter converts with `as usize`: a negative value becomes a
    // very large one, which the cuts below bring back to the string.
    let size = |v: &Value| -> NResult<usize> {
        let i = v.num()?.to_i128()?;
        Ok(usize::try_from(i).unwrap_or(usize::MAX))
    };
    let start = match start {
        Some(v) => size(v)?,
        None => 1,
    };
    let len = match len {
        Some(v) => size(v)?,
        None => rep.len(),
    };
    let from = start.saturating_sub(1).min(s.len());
    let to = from.saturating_add(len).min(s.len());
    let mut out = s.get(..from).unwrap_or_default().to_vec();
    out.extend_from_slice(rep);
    out.extend_from_slice(s.get(to..).unwrap_or_default());
    Ok(Value::Char(out))
}

/// An edit code (S2.26): the interpreter's `edit_code_with`, written again
/// under section 5's lints, with the program's DECEDIT (the decimal point,
/// and whether a zero goes before it).
pub fn edit_code_with(
    v: &Dec,
    digits: u32,
    decs: u32,
    code: char,
    point: char,
    zero: bool,
) -> NResult<String> {
    let bad = || anyhow::anyhow!("edit code {code}: {digits} digits, {decs} decimal places");
    let code = code.to_ascii_uppercase();
    let thousands = if point == ',' { '.' } else { ',' };
    let d = v.rescale(u8::try_from(decs)?, false).unwrap_or(*v);
    let w = usize::try_from(digits)?;
    let raw = format!("{:0>w$}", d.mantissa().unsigned_abs().to_string());
    let decs_n = usize::try_from(decs)?;
    let cut = raw.len().checked_sub(decs_n).ok_or_else(bad)?;
    let int = raw.get(..cut).ok_or_else(bad)?;
    let frac = raw.get(cut..).ok_or_else(bad)?;
    let commas = matches!(code, '1' | '2' | 'A' | 'B' | 'J' | 'K' | 'N' | 'O');
    let zero_blank = matches!(code, '2' | '4' | 'B' | 'D' | 'K' | 'M' | 'O' | 'Q' | 'Z');
    if code == 'X' {
        if d.is_negative() {
            let last = raw.len().checked_sub(1).ok_or_else(bad)?;
            let digit = raw
                .get(last..)
                .and_then(|c| c.parse::<usize>().ok())
                .ok_or_else(bad)?;
            let zoned = "}JKLMNOPQR".chars().nth(digit).ok_or_else(bad)?;
            return Ok(format!("{}{zoned}", raw.get(..last).ok_or_else(bad)?));
        }
        return Ok(raw);
    }
    if code == 'Z' {
        let all = format!("{int}{frac}");
        let t = all.trim_start_matches('0');
        return Ok(format!("{t:>w$}"));
    }
    if code == 'Y' {
        let s = if digits == 8 {
            format!(
                "{}/{}/{}",
                raw.get(0..2).ok_or_else(bad)?,
                raw.get(2..4).ok_or_else(bad)?,
                raw.get(4..8).ok_or_else(bad)?
            )
        } else {
            let s = format!("{raw:0>6}");
            let n = s.len();
            let at = |k: usize| n.checked_sub(k).ok_or_else(bad);
            format!(
                "{}/{}/{}",
                s.get(at(6)?..at(4)?).ok_or_else(bad)?,
                s.get(at(4)?..at(2)?).ok_or_else(bad)?,
                s.get(at(2)?..).ok_or_else(bad)?
            )
        };
        return Ok(match s.strip_prefix('0') {
            Some(rest) => format!(" {rest}"),
            None => s,
        });
    }
    let int_trim = int.trim_start_matches('0');
    let mut int_out = String::new();
    for (k, c) in int_trim.chars().enumerate() {
        let left = int_trim.len().saturating_sub(k);
        if commas && k > 0 && left.checked_rem(3) == Some(0) {
            int_out.push(thousands);
        }
        int_out.push(c);
    }
    if zero && decs > 0 && int_out.is_empty() {
        int_out.push('0');
    }
    let body = if decs > 0 {
        format!("{int_out}{point}{frac}")
    } else if int_out.is_empty() {
        "0".to_string()
    } else {
        int_out.clone()
    };
    let neg = d.is_negative();
    let sign = match code {
        'A' | 'B' | 'C' | 'D' => {
            if neg {
                "CR"
            } else {
                "  "
            }
        }
        'J' | 'K' | 'L' | 'M' => {
            if neg {
                "-"
            } else {
                " "
            }
        }
        _ => "",
    };
    let lead = match (matches!(code, 'N' | 'O' | 'P' | 'Q'), neg) {
        (true, true) => "-",
        (true, false) => " ",
        _ => "",
    };
    let groups = if commas {
        usize::try_from(digits.saturating_sub(decs).saturating_sub(1) / 3)?
    } else {
        0
    };
    let width = w
        .saturating_add(usize::from(decs > 0))
        .saturating_add(groups);
    if d.is_zero() && zero_blank {
        return Ok(" ".repeat(width.saturating_add(sign.len()).saturating_add(lead.len())));
    }
    let out = format!("{lead}{body}");
    let ow = width.saturating_add(lead.len());
    Ok(format!("{out:>ow$}{sign}"))
}

/// %EDITC of a number with a literal edit code (S2.26), as the
/// interpreter's "EDITC" arm: the value's digits and decimal places are
/// its node's precision (`p`), or else its own.
pub fn editc(
    v: &Dec,
    p: Option<(u32, u32)>,
    code: char,
    point: char,
    zero: bool,
) -> NResult<Value> {
    let (digits, decs) = p.unwrap_or_else(|| {
        let s = u32::from(v.scale());
        (v.int_digits().saturating_add(s), s)
    });
    Ok(Value::str(&edit_code_with(
        v, digits, decs, code, point, zero,
    )?))
}

/// %LOOKUP's array (S2.27): the DIM elements of a global array, loaded as
/// the interpreter's `list_of` loads them.
pub fn lookup_list(
    mem: &Mem,
    (buf, off, stride, dim): (usize, usize, usize, u32),
    t: DType,
) -> NResult<Vec<Value>> {
    let mut all = Vec::new();
    for k in 0..usize::try_from(dim)? {
        let at = k
            .checked_mul(stride)
            .and_then(|d| off.checked_add(d))
            .ok_or_else(|| anyhow::anyhow!("%LOOKUP: element {k} is outside storage"))?;
        all.push(load(mem, buf, at, t)?);
    }
    Ok(all)
}

/// %LOOKUP, %LOOKUPLT, LE, GE and GT (S2.27): the index of the element of
/// `all` found for `x`, 0 when none, as the interpreter's `search`, from
/// `start` (1) for `n` elements (to the end). `how` is "", LT, LE, GE or
/// GT.
pub fn search(
    x: &Value,
    all: &[Value],
    start: Option<&Value>,
    n: Option<&Value>,
    how: &str,
) -> NResult<Dec> {
    use std::cmp::Ordering;
    // The interpreter converts with `as usize`: a negative value becomes a
    // very large one.
    let size = |v: &Value| -> NResult<usize> {
        Ok(usize::try_from(v.num()?.to_i128()?).unwrap_or(usize::MAX))
    };
    let start = match start {
        Some(v) => size(v)?,
        None => 1,
    };
    let n = match n {
        Some(v) => size(v)?,
        None => all.len().saturating_add(1).saturating_sub(start.max(1)),
    };
    if start < 1 {
        anyhow::bail!("RNQ0121: the start index {start} is not valid");
    }
    let mut best: Option<(usize, &Value)> = None;
    for (k, v) in all.iter().enumerate().skip(start.saturating_sub(1)).take(n) {
        let o = data::compare(v, x)?;
        let ok = match how {
            "" => o == Ordering::Equal,
            "LT" => o == Ordering::Less,
            "LE" => o != Ordering::Greater,
            "GE" => o != Ordering::Less,
            _ => o == Ordering::Greater,
        };
        if !ok {
            continue;
        }
        let pos = k.saturating_add(1);
        if how.is_empty() || o == Ordering::Equal && (how == "LE" || how == "GE") {
            return Ok(Dec::from_i64(i64::try_from(pos)?));
        }
        let closer = match best {
            None => true,
            Some((_, bv)) => {
                let c = data::compare(v, bv)?;
                if matches!(how, "LT" | "LE") {
                    c == Ordering::Greater
                } else {
                    c == Ordering::Less
                }
            }
        };
        if closer {
            best = Some((pos, v));
        }
    }
    Ok(Dec::from_i64(i64::try_from(
        best.map(|b| b.0).unwrap_or(0),
    )?))
}

/// %SCANRPL (S2.28): each `pat` in `s` from `start` (1) for `len`
/// characters (to the end) replaced by `rep`, as the interpreter computes
/// it, with its RNQ0100 error for a start or length outside the string.
pub fn scanrpl(
    pat: &[u8],
    rep: &[u8],
    s: &[u8],
    start: Option<&Value>,
    len: Option<&Value>,
) -> NResult<Value> {
    let bad = || anyhow::anyhow!("RNQ0100: %SCANRPL start or length is outside the string.");
    // The interpreter converts with `as usize`: a negative value becomes a
    // very large one, which fails the check below.
    let size = |v: &Value| -> NResult<usize> {
        Ok(usize::try_from(v.num()?.to_i128()?).unwrap_or(usize::MAX))
    };
    let start = match start {
        Some(v) => size(v)?,
        None => 1,
    };
    let len = match len {
        Some(v) => size(v)?,
        None => s.len().saturating_add(1).saturating_sub(start.max(1)),
    };
    let from = start.checked_sub(1).ok_or_else(bad)?;
    let to = from
        .checked_add(len)
        .filter(|&e| e <= s.len())
        .ok_or_else(bad)?;
    let mut out = s.get(..from).ok_or_else(bad)?.to_vec();
    let region = s.get(from..to).ok_or_else(bad)?;
    let mut i = 0usize;
    while let Some(rest) = region.get(i..) {
        if rest.is_empty() {
            break;
        }
        if !pat.is_empty() && rest.starts_with(pat) {
            out.extend_from_slice(rep);
            i = i.saturating_add(pat.len());
        } else {
            out.extend(rest.first());
            i = i.saturating_add(1);
        }
    }
    out.extend_from_slice(s.get(to..).ok_or_else(bad)?);
    Ok(Value::Char(out))
}

/// An edit word (S2.29): the interpreter's `edit_word`, written again
/// under section 5's lints.
pub fn edit_word(v: &Dec, digits: u32, decs: u32, word: &str) -> NResult<String> {
    let w: Vec<char> = word.chars().collect();
    let at = |i: usize| w.get(i).copied();
    let stop = w.iter().position(|c| *c == '0' || *c == '*');
    let last_blank = w.iter().rposition(|c| *c == ' ');
    let body_end = match (last_blank, stop) {
        (Some(b), Some(s)) => b.max(s),
        (Some(b), None) => b,
        (None, Some(s)) => s,
        (None, None) => anyhow::bail!("the edit word '{word}' has no digit positions"),
    };
    let floating = stop.is_some_and(|s| s > 0 && s.checked_sub(1).and_then(at) == Some('$'));
    let positions = (0..=body_end)
        .filter(|&i| at(i) == Some(' ') || Some(i) == stop)
        .count();
    if u32::try_from(positions).unwrap_or(u32::MAX) < digits {
        anyhow::bail!(
            "the edit word '{word}' has {positions} digit positions, the field {digits} digits"
        );
    }
    let d = v.rescale(u8::try_from(decs)?, false).unwrap_or(*v);
    let text = format!("{:0>positions$}", d.mantissa().unsigned_abs());
    let mut ds = text.chars().skip(text.len().saturating_sub(positions));
    let fill = if stop.and_then(at) == Some('*') {
        '*'
    } else {
        ' '
    };
    let mut out: Vec<char> = Vec::with_capacity(w.len());
    let mut significant = false;
    let mut forced = false;
    let mut first_digit: Option<usize> = None;
    for (i, &c) in w.iter().enumerate().take(body_end.saturating_add(1)) {
        if floating && Some(i.saturating_add(1)) == stop {
            continue;
        }
        if c == ' ' || Some(i) == stop {
            let dg = ds.next().unwrap_or('0');
            if significant || forced || dg != '0' {
                if !significant && !forced {
                    first_digit = Some(out.len());
                }
                significant = true;
                out.push(dg);
            } else {
                out.push(fill);
            }
            if Some(i) == stop {
                forced = true;
                if first_digit.is_none() {
                    first_digit = Some(out.len());
                }
            }
        } else if c == '&' {
            out.push(' ');
        } else if c == '$' && i == 0 {
            out.push('$');
        } else if significant || forced {
            out.push(c);
        } else {
            out.push(fill);
        }
    }
    if floating {
        let k = first_digit.unwrap_or(out.len()).min(out.len());
        out.insert(k, '$');
    }
    let rest: Vec<char> = w
        .get(body_end.saturating_add(1)..)
        .unwrap_or_default()
        .to_vec();
    let rest_s: String = rest.iter().collect();
    let status_len = match rest_s.find("CR") {
        Some(p) => p.saturating_add(2),
        None if rest.last() == Some(&'-') => rest.len(),
        None => 0,
    }
    .min(rest.len());
    let negative = d.is_negative();
    for &c in rest.get(..status_len).unwrap_or_default() {
        out.push(if negative {
            if c == '&' {
                ' '
            } else {
                c
            }
        } else {
            ' '
        });
    }
    for &c in rest.get(status_len..).unwrap_or_default() {
        out.push(if c == '&' { ' ' } else { c });
    }
    Ok(out.into_iter().collect())
}

/// %EDITW (S2.29), as the interpreter's "EDITW" arm: the digits and
/// decimal places of a field's type (`p`), or else the value's own (at
/// least one digit).
pub fn editw(v: &Dec, p: Option<(u32, u32)>, word: &str) -> NResult<Value> {
    let (digits, decs) = p.unwrap_or_else(|| {
        let s = u32::from(v.scale());
        (v.int_digits().saturating_add(s).max(1), s)
    });
    Ok(Value::str(&edit_word(v, digits, decs, word)?))
}

/// Stage 2 (S2.49): pointer arithmetic, `p + n` or `p - n` (back): the
/// interpreter's ptrs.rs ptr_arith, written again. The number must be
/// whole; it is taken modulo 2^64 as the interpreter's `as i64` takes it.
/// An offset outside 0..=u32::MAX is MCH0601, a *NULL pointer MCH3601.
pub fn ptr_move(p: &Value, n: &Dec, back: bool) -> NResult<Value> {
    let p = match p {
        Value::Ptr(p) => *p,
        Value::Fig(crate::rpg::data::Fig::Null) => {
            anyhow::bail!("MCH3601: pointer arithmetic on a pointer that is not set (*NULL).")
        }
        // A pointer field loads as a pointer or *NULL only.
        other => anyhow::bail!("native: pointer arithmetic on {}", other.describe()),
    };
    let whole = n.to_i128()?;
    let wrapped = u64::try_from(whole.rem_euclid(1i128 << 64))?;
    let n = i64::from_ne_bytes(wrapped.to_ne_bytes());
    let by = if back { n.checked_neg() } else { Some(n) };
    // mem::Ptr::add, with its sum checked (it adds unchecked).
    by.and_then(|n| i64::from(p.off).checked_add(n))
        .and_then(|off| u32::try_from(off).ok())
        .map(|off| Value::Ptr(crate::rpg::mem::Ptr { off, ..p }))
        .ok_or_else(|| {
            anyhow::anyhow!("MCH0601: pointer arithmetic gave an offset outside the storage.")
        })
}

/// Stage 2 (S2.53): how %CHAR shows a date, time or timestamp field: the
/// format the interpreter's bif_override picks for it (the explicit one,
/// or the field's, or the control specification's), known when the code
/// is generated.
#[derive(Clone, Copy, Debug)]
pub enum TemporalFmt {
    Date(crate::rpg::dates::DateFmt),
    Time(crate::rpg::dates::TimeFmt),
    /// A timestamp: *ISO0 (digits only) or as it is.
    Ts {
        iso0: bool,
    },
}

/// Stage 2 (S2.53): %CHAR of a date, time or timestamp field's value
/// (its *ISO text), as bifs.rs bif_override does: the value parsed as
/// bifs.rs `temporal` parses it, then shown in `f`. A value that does not
/// parse as one (bif_override gives no value) is %CHAR's generic form,
/// the characters as they are.
pub fn temporal_char(v: &Value, f: TemporalFmt) -> NResult<Value> {
    use crate::rpg::dates::{self, DateFmt, TimeFmt};
    let Value::Char(c) = v else {
        return Ok(Value::Char(v.chars()?));
    };
    let t = data::a(c);
    let b = t.as_bytes();
    let at = |k: usize| b.get(k).copied();
    enum K {
        Date(dates::Ymd),
        Time(dates::Hms),
        Ts,
    }
    let kind = match t.len() {
        10 if at(4) == Some(b'-') && at(7) == Some(b'-') => {
            dates::parse_date(&t, &DateFmt::iso()).ok().map(K::Date)
        }
        8 if at(2) == Some(b'.') && at(5) == Some(b'.') => {
            dates::parse_time(&t, &TimeFmt::iso()).ok().map(K::Time)
        }
        19..=32 if at(4) == Some(b'-') && at(10) == Some(b'-') => {
            dates::parse_ts(&t).ok().map(|_| K::Ts)
        }
        _ => None,
    };
    Ok(match (kind, f) {
        (Some(K::Date(d)), TemporalFmt::Date(f)) => Value::str(&dates::format_date(d, &f)),
        (Some(K::Time(h)), TemporalFmt::Time(f)) => Value::str(&dates::format_time(h, &f)),
        (Some(K::Ts), TemporalFmt::Ts { iso0 }) => {
            let text = t.trim_end().to_string();
            if iso0 {
                Value::str(
                    &text
                        .chars()
                        .filter(|c| c.is_ascii_digit())
                        .collect::<String>(),
                )
            } else {
                Value::str(&text)
            }
        }
        // A value of another kind than its field's: not one lowering
        // makes (a field's value is of its own kind, or does not parse).
        (Some(_), _) => anyhow::bail!("native: %CHAR of a value of another kind than its field"),
        (None, _) => Value::Char(c.clone()),
    })
}

/// Stage 2 (S2.54): which of %DATE, %TIME and %TIMESTAMP.
#[derive(Clone, Copy, Debug)]
pub enum TemporalBif {
    Date,
    Time,
    Ts,
}

/// Stage 2 (S2.54): %DATE, %TIME or %TIMESTAMP, as bifs.rs bif_more does.
/// `v` is the argument's value, or None for none, *SYS or *JOB (the
/// current date and time, dates::now). `fmt` is a special value given as
/// the second argument. `dft` is the control specification's DATFMT or
/// TIMFMT code (dates::DateFmt::code).
pub fn temporal_bif(
    kind: TemporalBif,
    v: Option<Value>,
    fmt: Option<&str>,
    dft: &str,
) -> NResult<Value> {
    use crate::rpg::dates::{self, DateFmt, TimeFmt};
    let date_text = |d: dates::Ymd| Value::str(&dates::format_date(d, &DateFmt::iso()));
    let time_text = |t: dates::Hms| Value::str(&dates::format_time(t, &TimeFmt::iso()));
    let Some(v) = v else {
        let now = dates::now();
        return Ok(match kind {
            TemporalBif::Date => date_text(now.0),
            TemporalBif::Time => time_text(now.1),
            TemporalBif::Ts => Value::str(&dates::format_ts(&now, false)),
        });
    };
    let parsed = temporal_kind(&v);
    // A format with no separator for a number: its digits ("*MDY0").
    let digits = |f: &str| format!("{}0", f.trim_end_matches('0'));
    Ok(match kind {
        TemporalBif::Date => {
            if fmt.is_none() {
                match parsed {
                    Some(Parsed::Date(d)) => return Ok(date_text(d)),
                    Some(Parsed::Ts(ts)) => return Ok(date_text(ts.0)),
                    Some(Parsed::Time(_)) => anyhow::bail!("%DATE of a time is not valid"),
                    None => {}
                }
            }
            let f = fmt.map_or_else(|| dft.to_string(), str::to_string);
            let d = match &v {
                Value::Num(n) => dates::parse_date(
                    &n.trunc().mantissa().to_string(),
                    &DateFmt::parse(&digits(&f))?,
                )?,
                other => dates::parse_date(&data::a(&other.chars()?), &DateFmt::parse(&f)?)?,
            };
            date_text(d)
        }
        TemporalBif::Time => {
            if fmt.is_none() {
                match parsed {
                    Some(Parsed::Time(h)) => return Ok(time_text(h)),
                    Some(Parsed::Ts(ts)) => return Ok(time_text(ts.1)),
                    Some(Parsed::Date(_)) => anyhow::bail!("%TIME of a date is not valid"),
                    None => {}
                }
            }
            let f = fmt.map_or_else(|| dft.to_string(), str::to_string);
            let h = match &v {
                Value::Num(n) => dates::parse_time(
                    &n.trunc().mantissa().to_string(),
                    &TimeFmt::parse(&digits(&f))?,
                )?,
                other => dates::parse_time(&data::a(&other.chars()?), &TimeFmt::parse(&f)?)?,
            };
            time_text(h)
        }
        TemporalBif::Ts => {
            let ts = match parsed {
                Some(Parsed::Date(d)) => (d, (0, 0, 0), 0),
                Some(Parsed::Ts(t)) => t,
                Some(Parsed::Time(_)) => anyhow::bail!("%TIMESTAMP of a time is not valid"),
                None => match &v {
                    Value::Num(n) => dates::parse_ts(&n.trunc().mantissa().to_string())?,
                    other => dates::parse_ts(&data::a(&other.chars()?))?,
                },
            };
            Value::str(&dates::format_ts(&ts, false))
        }
    })
}

/// What a value is as a date, time or timestamp (bifs.rs `temporal`).
enum Parsed {
    Date(crate::rpg::dates::Ymd),
    Time(crate::rpg::dates::Hms),
    Ts(crate::rpg::dates::Ts),
}

fn temporal_kind(v: &Value) -> Option<Parsed> {
    use crate::rpg::dates::{self, DateFmt, TimeFmt};
    let Value::Char(c) = v else { return None };
    let t = data::a(c);
    let b = t.as_bytes();
    let at = |k: usize| b.get(k).copied();
    match t.len() {
        10 if at(4) == Some(b'-') && at(7) == Some(b'-') => dates::parse_date(&t, &DateFmt::iso())
            .ok()
            .map(Parsed::Date),
        8 if at(2) == Some(b'.') && at(5) == Some(b'.') => dates::parse_time(&t, &TimeFmt::iso())
            .ok()
            .map(Parsed::Time),
        19..=32 if at(4) == Some(b'-') && at(10) == Some(b'-') => {
            dates::parse_ts(&t).ok().map(Parsed::Ts)
        }
        _ => None,
    }
}

/// A value as bifs.rs `as_ts` makes it a timestamp (S2.55).
fn temporal_ts(v: &Value, what: &str) -> NResult<crate::rpg::dates::Ts> {
    Ok(match temporal_kind(v) {
        Some(Parsed::Date(d)) => (d, (0, 0, 0), 0),
        Some(Parsed::Time(h)) => ((1, 1, 1), h, 0),
        Some(Parsed::Ts(t)) => t,
        None => anyhow::bail!("{what}: {} is not a date, time or timestamp", v.describe()),
    })
}

/// Stage 2 (S2.55): %DIFF(x : y : unit), as bifs.rs bif_more.
pub fn temporal_diff(x: &Value, y: &Value, unit: data::DurUnit) -> NResult<Dec> {
    let a = temporal_ts(x, "%DIFF")?;
    let b = temporal_ts(y, "%DIFF")?;
    Ok(Dec::new(crate::rpg::dates::diff(&a, &b, unit), 0))
}

/// Stage 2 (S2.55): %SUBDT(x : unit), as bifs.rs bif_more.
pub fn temporal_subdt(x: &Value, unit: data::DurUnit) -> NResult<Dec> {
    let t = temporal_ts(x, "%SUBDT")?;
    Ok(Dec::new(i128::from(crate::rpg::dates::subdt(&t, unit)), 0))
}

/// Stage 2 (S2.55): a date, time or timestamp plus or minus a duration
/// (%YEARS ... %MSECONDS of `n`), as bifs.rs date_arith. The duration's
/// number is taken as int_arg takes it: whole, and modulo 2^64 as the
/// interpreter's `as i64`.
pub fn temporal_add(base: &Value, unit: data::DurUnit, n: &Dec, minus: bool) -> NResult<Value> {
    use crate::rpg::dates::{self, DateFmt, TimeFmt};
    let whole = n.to_i128()?;
    let wrapped = u64::try_from(whole.rem_euclid(1i128 << 64))?;
    let n = i64::from_ne_bytes(wrapped.to_ne_bytes());
    let n = if minus {
        n.checked_neg()
            .ok_or_else(|| anyhow::anyhow!("native: a duration of {n} cannot be negated"))?
    } else {
        n
    };
    Ok(match temporal_kind(base) {
        Some(Parsed::Date(d)) => Value::str(&dates::format_date(
            dates::add_to_date(d, unit, n)?,
            &DateFmt::iso(),
        )),
        Some(Parsed::Time(h)) => Value::str(&dates::format_time(
            dates::add_to_time(h, unit, n)?,
            &TimeFmt::iso(),
        )),
        Some(Parsed::Ts(t)) => {
            Value::str(&dates::format_ts(&dates::add_to_ts(&t, unit, n)?, false))
        }
        None => anyhow::bail!("{} is not a date, time or timestamp", base.describe()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_stack_keeps_groups_past_a_tag() {
        let mut s = MonitorStack::new();
        s.push(MonitorRef {
            at: 0,
            end_body: 10,
            end: 12,
        });
        s.push(MonitorRef {
            at: 2,
            end_body: 5,
            end: 7,
        });
        // A TAG at 6 is past the inner body, inside the outer one.
        s.retain_past(6);
        assert_eq!(s.len(), 1);
        assert_eq!(s.last().map(|g| g.at), Some(0));
        assert_eq!(s.pop().map(|g| g.end), Some(12));
        assert!(s.is_empty());
    }
}
