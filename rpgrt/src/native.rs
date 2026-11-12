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
    /// Stage 2 (docs/native/adr/0006): a procedure is running, so names
    /// may be its locals; lowered code then lets the interpreter do it.
    fn in_procedure(&self) -> bool;
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

/// The bytes of a field: buffer `buf`, `off`, the type's size.
fn bytes<'m>(mem: &'m Mem, buf: usize, off: usize, t: &DType) -> NResult<&'m [u8]> {
    let end = off
        .checked_add(t.size())
        .ok_or_else(|| anyhow::anyhow!("native: a field past the end of storage"))?;
    mem.get(buf)
        .and_then(|b| b.get(off..end))
        .ok_or_else(|| anyhow::anyhow!("native: a field past the end of its storage"))
}

/// A field's value, as the interpreter reads a standalone field with no
/// index (`load_var`: `data::load` over its bytes).
pub fn load(mem: &Mem, buf: usize, off: usize, t: DType) -> NResult<Value> {
    data::load(&t, bytes(mem, buf, off, &t)?)
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
    let end = off
        .checked_add(t.size())
        .ok_or_else(|| anyhow::anyhow!("native: a field past the end of storage"))?;
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
