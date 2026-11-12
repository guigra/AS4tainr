//! The native engine inside the interpreter (docs/native/adr/0001, 0004):
//! the dispatcher `run_code_at` calls first, and the machine as the
//! `rpgrt::native::Runtime` generated routines call back into. As a
//! submodule of `interp` it reaches the machine's private state without
//! changing it.
//!
//! Each `Runtime` method is the matching arm of `Machine::run_code_at`'s
//! step, and `settle` is its error routing, written to do exactly what
//! the interpreter does (a test watches `run_code_at` for changes:
//! core/tests/native_tripwire.rs).

#![forbid(unsafe_code, unused_must_use)]
#![forbid(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects
)]

use super::super::ast::{BinOp, Expr};
use super::super::code::{Code, Ins, I};
use super::super::data::{self, a, Fit, Value};
use super::super::dec::Dec;
use super::super::native::{self, Engine};
use super::errors;
use super::{Flow, GotoSignal, LeaveSrSignal, Machine, ReturnPointSignal, ReturnSignal};
use anyhow::{anyhow, Result};
use rpgrt::native::{MonitorStack, NResult, Runtime, Settled};
use std::cmp::Ordering;

/// Before `run_code_at` runs `code` from `start`: None to let the
/// interpreter run it; Some with how the routine ended when generated
/// code ran it (or why the program cannot run).
pub(super) fn dispatch(m: &mut Machine<'_>, code: &Code, start: usize) -> Option<Result<Flow>> {
    let engine = match native::engine() {
        Ok(Engine::Interp) => return None,
        Ok(e) => e,
        Err(msg) => return Some(Err(abort(m, msg))),
    };
    let prog = m.prog;
    let table = prog.native.table(prog);
    let refuse = |m: &mut Machine<'_>, reason: String| {
        let fp = table
            .missing()
            .map(|e| e.fingerprint.as_str())
            .unwrap_or("");
        native::note_refusal(engine, &table.program, fp, &reason);
        match engine {
            Engine::Rust => Some(Err(abort(
                m,
                format!(
                    "{}=rust: program {}: {reason}",
                    native::ENGINE_VAR,
                    table.program
                ),
            ))),
            _ => {
                native::note_fallback(&table.program, &reason);
                None
            }
        }
    };
    if let Some(e) = table.missing() {
        let at = if e.line == 0 {
            String::new()
        } else {
            format!(" (line {})", e.line)
        };
        return refuse(
            m,
            format!(
                "no native code for {}{at}, fingerprint {}; generate it with AS4TAINR_NATIVE_EMIT and build with AS4TAINR_NATIVE_DIR",
                e.place, e.fingerprint
            ),
        );
    }
    let Some(run) = table.find(prog, code).and_then(|e| e.run) else {
        return refuse(m, "a routine not found in the program".to_string());
    };
    let mut rt = Rt { m, code };
    Some(run(&mut rt, start).map(|f| match f {
        rpgrt::native::Flow::Normal => Flow::Normal,
        rpgrt::native::Flow::Return(v) => Flow::Return(v),
        rpgrt::native::Flow::LeaveSr => Flow::LeaveSr,
    }))
}

/// An error that ends the program before any of its calculations run:
/// no error subroutine or ON-ERROR of this program takes it (it is not
/// the program's error). `recover` stops looking for an error subroutine
/// once the handler depth is reached, so the machine is put there; the
/// program ends with the error, and its caller sees a failed call.
fn abort(m: &mut Machine<'_>, msg: String) -> anyhow::Error {
    m.in_handler = u32::MAX;
    anyhow!(msg)
}

/// The machine running one routine for generated code.
struct Rt<'r, 'a> {
    m: &'r mut Machine<'a>,
    code: &'r Code,
}

/// The instruction at `pc` (borrowing only the routine, not the machine).
fn at(code: &Code, pc: usize) -> NResult<&I> {
    code.get(pc)
        .ok_or_else(|| anyhow!("native: no instruction {pc} in the routine"))
}

/// Generated code asked an instruction for what it is not: the routine
/// and its generated code do not match (they are found by fingerprint,
/// so this does not happen).
fn mismatch(pc: usize, want: &str) -> anyhow::Error {
    anyhow!("native: instruction {pc} is not {want}")
}

impl Runtime for Rt<'_, '_> {
    fn cond(&mut self, pc: usize) -> bool {
        self.code
            .get(pc)
            .is_some_and(|i| i.cond.is_empty() || self.m.conditions_met(&i.cond))
    }

    fn line(&mut self, pc: usize) {
        if let Some(i) = self.code.get(pc) {
            self.m.line = i.line;
        }
    }

    fn stmt(&mut self, pc: usize) -> NResult<()> {
        match &at(self.code, pc)?.ins {
            Ins::Stmt(s) => self.m.stmt_e(s),
            _ => Err(mismatch(pc, "a statement")),
        }
    }

    fn truth(&mut self, pc: usize) -> NResult<bool> {
        match &at(self.code, pc)?.ins {
            Ins::JumpIfNot(c, _) => self.m.eval(c)?.truth(),
            _ => Err(mismatch(pc, "a jump-if-not")),
        }
    }

    fn loop_past(&mut self, pc: usize) -> NResult<bool> {
        let Ins::LoopTest {
            var, limit, down, ..
        } = &at(self.code, pc)?.ins
        else {
            return Err(mismatch(pc, "a loop test"));
        };
        let v = self.m.eval(var)?.num()?;
        match limit {
            Some(l) => {
                let l = self.m.eval(l)?.num()?;
                Ok(if *down { v < l } else { v > l })
            }
            None => Ok(false),
        }
    }

    fn loop_step(&mut self, pc: usize) -> NResult<()> {
        let Ins::LoopStep { var, by, down } = &at(self.code, pc)?.ins else {
            return Err(mismatch(pc, "a loop step"));
        };
        let step = match by {
            Some(b) => self.m.eval(b)?.num()?,
            None => Dec::from_i64(1),
        };
        let v = self.m.eval(var)?.num()?;
        let n = if *down { v.sub(&step)? } else { v.add(&step)? };
        self.m.assign(
            var,
            &Value::Num(n),
            Fit::Strict { half_adjust: false },
            false,
        )
    }

    fn each_init(&mut self, pc: usize) -> NResult<()> {
        let Ins::EachInit { list } = &at(self.code, pc)?.ins else {
            return Err(mismatch(pc, "a FOR-EACH start"));
        };
        let vals = self.m.list_of(list)?;
        let key = (self.code.as_ptr() as usize, pc, self.m.frames.len());
        self.m.each.insert(key, (vals, 0));
        Ok(())
    }

    fn each_next(&mut self, pc: usize) -> NResult<bool> {
        let Ins::EachNext { var, init, .. } = &at(self.code, pc)?.ins else {
            return Err(mismatch(pc, "a FOR-EACH step"));
        };
        let key = (self.code.as_ptr() as usize, *init, self.m.frames.len());
        let next = self.m.each.get_mut(&key).and_then(|(v, at)| {
            let x = v.get(*at).cloned();
            // A list's length is a usize: it cannot reach usize::MAX.
            *at = at.saturating_add(1);
            x
        });
        match next {
            Some(v) => {
                self.m
                    .assign(var, &v, Fit::Strict { half_adjust: false }, false)?;
                Ok(true)
            }
            None => {
                self.m.each.remove(&key);
                Ok(false)
            }
        }
    }

    fn exsr(&mut self, pc: usize) -> NResult<()> {
        let i = at(self.code, pc)?;
        let line = i.line;
        let Ins::ExSr(name) = &i.ins else {
            return Err(mismatch(pc, "an EXSR"));
        };
        let prog = self.m.prog;
        let local = self.m.frames.last().map(|f| f.proc);
        let sub = local
            .and_then(|p| p.routines.subroutines.get(name))
            .or_else(|| prog.routines.subroutines.get(name))
            .ok_or_else(|| anyhow!("subroutine {name} is not defined"))?;
        let saved = std::mem::replace(&mut self.m.routine, name.clone());
        let r = self.m.run_code(sub);
        self.m.routine = saved;
        self.m.line = line;
        match r? {
            Flow::Return(v) => Err(ReturnSignal(v).into()),
            _ => Ok(()),
        }
    }

    fn ret(&mut self, pc: usize) -> NResult<()> {
        let Ins::Return(v) = &at(self.code, pc)?.ins else {
            return Err(mismatch(pc, "a RETURN"));
        };
        let val = match v {
            // A data structure array (a procedure returning LIKEDS(ds)
            // DIM(n)): all its elements.
            Some(x) if self.m.whole_ds_array(x).is_some() => {
                let d = self
                    .m
                    .whole_ds_array(x)
                    .ok_or_else(|| mismatch(pc, "a data structure array"))?;
                let n = usize::try_from(d.dim)
                    .ok()
                    .and_then(|dim| d.size.checked_mul(dim))
                    .ok_or_else(|| anyhow!("native: data structure array size"))?;
                let end = d
                    .off
                    .checked_add(n)
                    .ok_or_else(|| anyhow!("native: data structure array size"))?;
                let buf = std::ops::Index::index(&self.m.bufs, d.buf);
                let bytes = buf
                    .get(d.off..end)
                    .ok_or_else(|| anyhow!("native: data structure array past its storage"))?;
                Some(Value::Char(bytes.to_vec()))
            }
            Some(x) => Some(self.m.eval(x)?),
            None => None,
        };
        Err(ReturnSignal(val).into())
    }

    fn leave_sr(&mut self, _pc: usize) -> NResult<()> {
        Err(LeaveSrSignal.into())
    }

    fn cab(&mut self, pc: usize) -> NResult<bool> {
        let Ins::Cab {
            cmp,
            f1,
            f2,
            to,
            ind,
            out,
        } = &at(self.code, pc)?.ins
        else {
            return Err(mismatch(pc, "a CABxx"));
        };
        let x = self.m.eval(f1)?;
        let y = self.m.eval(f2)?;
        let o = data::compare(&x, &y)?;
        self.m.set_result_inds(
            ind,
            o == Ordering::Greater,
            o == Ordering::Less,
            o == Ordering::Equal,
        );
        let hit = match cmp {
            None => true,
            Some(BinOp::Eq) => o == Ordering::Equal,
            Some(BinOp::Ne) => o != Ordering::Equal,
            Some(BinOp::Lt) => o == Ordering::Less,
            Some(BinOp::Le) => o != Ordering::Greater,
            Some(BinOp::Gt) => o == Ordering::Greater,
            Some(BinOp::Ge) => o != Ordering::Less,
            _ => false,
        };
        if hit {
            if let Some(t) = out {
                return Err(GotoSignal(t.clone()).into());
            }
        }
        Ok(hit && *to != usize::MAX)
    }

    fn goto_out(&mut self, pc: usize) -> NResult<()> {
        match &at(self.code, pc)?.ins {
            Ins::GotoOut(t) => Err(GotoSignal(t.clone()).into()),
            _ => Err(mismatch(pc, "a GOTO out of the subroutine")),
        }
    }

    fn end_sr(&mut self, pc: usize) -> NResult<()> {
        let Ins::EndSr(p) = &at(self.code, pc)?.ins else {
            return Err(mismatch(pc, "an ENDSR with a return point"));
        };
        // The return point, then the field is blank again (as the
        // interpreter does: ILE RPG Reference, INFSR).
        let point = a(&self.m.eval(p)?.chars()?).trim().to_ascii_uppercase();
        if matches!(p, Expr::Name { .. }) {
            self.m.assign(
                p,
                &Value::str(""),
                Fit::Truncate { half_adjust: false },
                false,
            )?;
        }
        if !point.is_empty() {
            return Err(ReturnPointSignal(point).into());
        }
        Ok(())
    }

    fn in_procedure(&self) -> bool {
        // Names resolve against the running procedure's locals first
        // (`Machine::sym`): lowered code then lets the interpreter do it.
        !self.m.frames.is_empty()
    }

    fn mem(&mut self) -> &mut rpgrt::rpg::mem::Mem {
        &mut self.m.bufs
    }

    fn num_inds(&mut self, pc: usize, stored: &Dec) {
        if let Some(Ins::Stmt(super::super::ast::Stmt::Arith { ind, .. })) =
            self.code.get(pc).map(|i| &i.ins)
        {
            self.m.num_inds(ind, stored);
        }
    }

    fn precision(&mut self, pc: usize, path: &[u8]) -> Option<(u32, u32)> {
        let Some(Ins::Stmt(super::super::ast::Stmt::Eval { value, .. })) =
            self.code.get(pc).map(|i| &i.ins)
        else {
            return None;
        };
        let mut node = value;
        for step in path {
            node = match node {
                Expr::Bin(_, l, r) => {
                    if *step == 0 {
                        l
                    } else {
                        r
                    }
                }
                _ => return None,
            };
        }
        self.m.num_precision(node)
    }

    fn settle(&mut self, pc: usize, step: NResult<usize>, monitors: &mut MonitorStack) -> Settled {
        // A pointer fault in a condition (IF, DOW...).
        let step = match (step, self.m.take_fault()) {
            (Err(e), _) if errors::is_signal(&e) => Err(e),
            (_, Some(f)) => Err(anyhow!(f)),
            (s, None) => s,
        };
        let err = match step {
            Ok(t) => return Settled::Go(t),
            Err(err) => err,
        };
        if let Some(r) = err.downcast_ref::<ReturnSignal>() {
            return Settled::Exit(Ok(Flow::Return(r.0.clone()).into_native()));
        }
        if err.downcast_ref::<LeaveSrSignal>().is_some() {
            return Settled::Exit(Ok(rpgrt::native::Flow::LeaveSr));
        }
        // GOTO from a subroutine to a TAG here.
        if let Some(GotoSignal(t)) = err.downcast_ref::<GotoSignal>() {
            let to = self
                .code
                .iter()
                .position(|x| matches!(&x.ins, Ins::Label(l) if l == t));
            if let Some(to) = to {
                monitors.retain_past(to);
                return Settled::Go(to);
            }
            return Settled::Exit(Err(err));
        }
        if err.downcast_ref::<ReturnPointSignal>().is_some() {
            return Settled::Exit(Err(err));
        }
        // An escape message sent to a caller passes this procedure by.
        if self.m.passes_by(&err) {
            return Settled::Exit(Err(err));
        }
        // An error inside a monitored body goes to the first ON-ERROR that
        // lists its status (or *ALL / nothing).
        let mut handled = None;
        let code = errors::status_code(&err);
        let id = errors::message_id(&err);
        while let Some(g) = monitors.last().copied() {
            if pc < g.end_body {
                handled = match self.code.get(g.at).map(|i| &i.ins) {
                    Some(Ins::Monitor { handlers, .. }) => handlers
                        .iter()
                        .find(|(codes, _)| {
                            codes.is_empty()
                                || codes.iter().any(|c| {
                                    c.strip_prefix("MSG:").is_some_and(|m| m == id)
                                        || errors::on_error_takes(c, code)
                                })
                        })
                        .map(|h| h.1),
                    _ => None,
                };
                break;
            }
            monitors.pop();
        }
        let line = self.code.get(pc).map(|i| i.line).unwrap_or(0);
        match handled {
            Some(h) => {
                self.m.note_error(&err);
                Settled::Go(h)
            }
            None if line == 0 => Settled::Exit(Err(err)),
            // Where the error happened, for its exception message's sender
            // (shown as "line N").
            None => {
                let first = errors::origin(&err);
                let called = first.as_ref().is_some_and(|a| a.called);
                let sender = first.map(|a| a.sender).unwrap_or_else(|| self.m.sender());
                Settled::Exit(Err(err.context(errors::At {
                    line,
                    sender,
                    called,
                })))
            }
        }
    }
}

trait IntoNative {
    fn into_native(self) -> rpgrt::native::Flow;
}

impl IntoNative for Flow {
    fn into_native(self) -> rpgrt::native::Flow {
        match self {
            Flow::Normal => rpgrt::native::Flow::Normal,
            Flow::Return(v) => rpgrt::native::Flow::Return(v),
            Flow::LeaveSr => rpgrt::native::Flow::LeaveSr,
        }
    }
}

/// The PSDS positions the run fills from the clock (1-based, inclusive):
/// the job's date and century (191-200), the run date and job date
/// (270-281) and the run time (282-287), as `psds_start` writes them.
const PSDS_CLOCK: &[(usize, usize)] = &[(191, 200), (270, 287)];

/// When a test is collecting them (docs/native/adr/0005): what the
/// program left as its call ended.
pub(crate) fn observe(m: &Machine<'_>) {
    if !native::observing() {
        return;
    }
    let prog = m.prog;
    let mut globals: Vec<Vec<u8>> = (0..prog.globals.len())
        .map(|i| m.bufs.get(i).cloned().unwrap_or_default())
        .collect();
    let psds = prog.psds.as_ref().and_then(|n| match prog.syms.get(n) {
        Some(super::super::program::Sym::Ds(d)) => Some((d.buf, d.off)),
        _ => None,
    });
    if let Some((buf, off)) = psds {
        if let Some(b) = globals.get_mut(buf) {
            for &(from, to) in PSDS_CLOCK {
                let range = off
                    .checked_add(from)
                    .and_then(|s| s.checked_sub(1))
                    .zip(off.checked_add(to));
                if let Some((s, e)) = range {
                    if let Some(bytes) = b.get_mut(s..e) {
                        bytes.fill(b' ');
                    }
                }
            }
        }
    }
    let mut names: Vec<Vec<String>> = vec![Vec::new(); globals.len()];
    for (name, sym) in &prog.syms {
        let buf = match sym {
            super::super::program::Sym::Var(v) if !v.local => Some(v.buf),
            super::super::program::Sym::Ds(d) if !d.local => Some(d.buf),
            _ => None,
        };
        if let Some(n) = buf.and_then(|b| names.get_mut(b)) {
            n.push(name.clone());
        }
    }
    for n in &mut names {
        n.sort();
    }
    let mut indicators: Vec<(String, bool)> = m.inds.iter().map(|(k, v)| (k.clone(), *v)).collect();
    indicators.sort();
    native::record(native::Observation {
        program: prog.name.clone(),
        engine: native::engine().ok(),
        globals,
        names,
        indicators,
    });
}
