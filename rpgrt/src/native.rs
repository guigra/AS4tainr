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

use crate::rpg::data::Value;

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
