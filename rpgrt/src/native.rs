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

/// A field's value, as the interpreter reads a standalone field with no
/// index (`load_var`: `data::load` over its bytes; a parameter longer than
/// the storage passed for it, OPTIONS(*VARSIZE), reads what is there and
/// then blanks).
pub fn load(mem: &Mem, buf: usize, off: usize, t: DType) -> NResult<Value> {
    let b = std::ops::Index::index(mem, buf);
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
    let have = std::ops::Index::index(&*mem, buf).len();
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
