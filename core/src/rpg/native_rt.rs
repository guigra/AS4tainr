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

use super::super::ast::{BinOp, Expr, FileOp};
use super::super::code::{Code, Ins, I};
use super::super::data::{self, a, Fit, Value};
use super::super::dec::Dec;
use super::super::native::{self, Engine};
use super::errors;
use super::{Flow, GotoSignal, LeaveSrSignal, Machine, ReturnPointSignal, ReturnSignal};
use anyhow::{anyhow, Result};
use rpgrt::native::{CallArg, FileBit, KeyArg, MonitorStack, NResult, Runtime, Settled};
use std::cmp::Ordering;
use std::io::Write as _;

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
    let Some(entry) = table.find(prog, code) else {
        return refuse(m, "a routine not found in the program".to_string());
    };
    let Some(run) = entry.run else {
        return refuse(m, format!("no native code for {}", entry.place));
    };
    // Lowered code runs only in the routine's own context (S2.8): a main
    // program routine with no procedure frame, a procedure's routine under
    // that procedure's own frame. Elsewhere names may resolve otherwise.
    let ok = match &entry.place {
        native::Place::Proc(name, _) => match (m.frames.last(), prog.procs.get(name)) {
            (Some(f), Some(pi)) => std::ptr::eq(f.proc, pi),
            _ => false,
        },
        _ => m.frames.is_empty(),
    };
    // ADR 0007 (S2.37): generated code has no interpreter fallback, so a
    // routine outside its own context is refused as a whole: under auto
    // the interpreter runs it, under rust it is an error. On IBM i a
    // subprocedure cannot run a subroutine of the main procedure (EXSR:
    // RNF7030/RNF7018 at severity 30, PUB400 capture
    // tools/golden/native45), the one way the interpreter gets here.
    if !ok {
        return refuse(
            m,
            format!(
                "{} runs under the frame of another procedure (a subprocedure's EXSR of a main procedure subroutine, which IBM i does not compile)",
                entry.place
            ),
        );
    }
    let mut rt = Rt {
        m,
        code,
        ok,
        pending: None,
    };
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
    /// Lowered code may run here (`Runtime::native_ok`).
    ok: bool,
    /// A file read between `file_read` and `file_done` (S2.58).
    pending: Option<PendingRead>,
}

/// What `file_read` leaves for `file_done` (S2.58): the file, the
/// operation, and what it read.
struct PendingRead {
    file: usize,
    found: bool,
    eof: bool,
    has_record: bool,
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

impl<'a> Rt<'_, 'a> {
    /// The frame of a native call (S2.35): call_proc's steps for the
    /// procedures lowering lets through.
    fn call_frame(&mut self, proc: &str, args: Vec<CallArg>) -> NResult<Option<Value>> {
        let prog: &'a super::super::program::Program = self.m.prog;
        let pi = prog
            .procs
            .get(proc)
            .ok_or_else(|| anyhow!("native: {proc} is not a procedure of this program"))?;
        if self.m.frames.len() > 1000 {
            anyhow::bail!("procedure {}: calls are nested too deeply", pi.name);
        }
        if args.len() > pi.params.len() {
            anyhow::bail!(
                "{proc}: {} parameters passed, it takes {}",
                args.len(),
                pi.params.len()
            );
        }
        if let Some(p) = pi.params.get(args.len()) {
            if !p.nopass {
                anyhow::bail!("{proc}: parameter {} is missing", p.name);
            }
        }
        let base = self.m.bufs.len();
        let mut alias = std::collections::HashMap::new();
        let mut values = Vec::new();
        let mut lengths = Vec::new();
        let passed = args.len();
        for (p, arg) in pi.params.iter().zip(args) {
            match arg {
                CallArg::Ref { buf, off, .. } => {
                    // The descriptor of a reference argument of the
                    // parameter's own type (call_proc).
                    lengths.push(Some(match p.dtype {
                        data::DType::Varchar(n) => {
                            let max = usize::try_from(n)?;
                            let pre = data::var_prefix(n);
                            let end = off
                                .checked_add(pre)
                                .ok_or_else(|| anyhow!("native: an argument's offset"))?;
                            let cur = self
                                .m
                                .bufs
                                .get(buf)
                                .and_then(|b| b.get(off..end))
                                .map(|x| data::var_len(n, x))
                                .unwrap_or(0);
                            (cur, 4, max)
                        }
                        t => (t.size(), 2, t.size()),
                    }));
                    if let Some(b) = p.buf {
                        alias.insert(b, (buf, off));
                    }
                }
                CallArg::Val(v) => {
                    let len = match &v {
                        Value::Char(c) => Some(c.len()),
                        _ => None,
                    };
                    lengths.push(match (p.dtype, len) {
                        (data::DType::Varchar(n), Some(k)) => {
                            let max = usize::try_from(n)?;
                            Some((k.min(max), 4, max))
                        }
                        (_, Some(k)) => Some((k, 2, k)),
                        _ => None,
                    });
                    values.push((p.name.clone(), v));
                }
            }
        }
        self.m.bufs.extend(pi.frame.iter().cloned());
        self.m.frames.push(super::Frame {
            proc: pi,
            base,
            alias,
            passed,
            omitted: Vec::new(),
            files: std::collections::HashMap::new(),
            lengths,
        });
        let pgm = self.m.stack_pgm();
        let lib = self.m.stack_lib();
        let job = self.m.io.job_key.clone();
        if self.m.line > 0 {
            crate::pgmmsg::stack_line(&job, self.m.line);
        }
        let ext = self.m.external_proc_name(&pi.name);
        let module = self.m.stack_module();
        let actgrp = self.m.io.actgrp.clone();
        let stack_id = crate::pgmmsg::stack_push(&job, (&lib, &pgm), &module, &ext, &actgrp);
        let saved_routine = std::mem::replace(&mut self.m.routine, pi.name.clone());
        let saved_line = self.m.line;
        let r = self.call_body(pi, values);
        crate::pgmmsg::stack_pop(&job, stack_id);
        self.m.frames.pop();
        self.m.bufs.truncate(base);
        self.m.routine = saved_routine;
        self.m.line = saved_line;
        let v = r?;
        Ok(match (pi.returns, v) {
            (Some(t), Some(v)) => {
                let mut buf = t.initial();
                data::store(&t, &v, &mut buf, Fit::Strict { half_adjust: false }, false)?;
                Some(data::load(&t, &buf)?)
            }
            (Some(_), None) => {
                anyhow::bail!("procedure {} ended without returning a value", pi.name)
            }
            (None, _) => None,
        })
    }

    /// A call through a prototype (S2.46): call_named_inner's path for a
    /// prototype that lowering lets through. A program (EXTPGM) gets each
    /// value as a copy in its parameter's type, through the program call
    /// service (`call_program`, a phase B dependency, ADR 0007). Otherwise
    /// the procedure the prototype names is this module's, or one bound
    /// into the program (`call_bound`), or it is CPD5D02, as there.
    fn call_proto(&mut self, name: &str, args: Vec<CallArg>) -> NResult<Option<Value>> {
        let pr = self
            .m
            .proto(name)
            .ok_or_else(|| anyhow!("{name} is not a procedure or prototype of this program"))?;
        if let Some(pgm) = &pr.extpgm {
            let pgm = match self.m.sym(pgm) {
                Some(super::super::program::Sym::Const(v)) => {
                    a(&v.chars()?).trim().to_ascii_uppercase()
                }
                Some(_) => anyhow::bail!("native: the program of {name} is not a constant"),
                None => pgm.clone(),
            };
            let base = self.m.bufs.len();
            let mut locs = Vec::with_capacity(args.len());
            for (arg, p) in args.into_iter().zip(&pr.params) {
                match arg {
                    // As args_for's loc_of (S2.51): the argument's own size
                    // and type (they are the parameter's but for *VARSIZE).
                    CallArg::Ref { buf, off, dtype } => locs.push(super::Loc {
                        buf,
                        off,
                        size: dtype.size(),
                        dtype,
                    }),
                    CallArg::Val(v) => {
                        let mut b = p.dtype.initial();
                        data::store(
                            &p.dtype,
                            &v,
                            &mut b,
                            Fit::Strict { half_adjust: false },
                            false,
                        )?;
                        locs.push(super::Loc {
                            buf: self.m.bufs.len(),
                            off: 0,
                            size: b.len(),
                            dtype: p.dtype,
                        });
                        self.m.bufs.push(b);
                    }
                }
            }
            let r = self.m.call_program(&pgm, &locs);
            self.m.bufs.truncate(base);
            r?;
            return Ok(None);
        }
        let target = pr.extproc.clone().unwrap_or_else(|| name.to_string());
        let sym = target.to_ascii_uppercase();
        // call_named_inner's order (S2.56): a procedure of this module, one
        // the binder found, a CEE API, a C runtime function. EXTPROC written
        // in lower case names the system's C function even when the module
        // has one of that name (`system_named`).
        let system_named = pr
            .written
            .as_deref()
            .is_some_and(|w| w != w.to_ascii_uppercase())
            && super::crt::C_FUNCTIONS.contains(&target.as_str());
        // args_for's check: only a C function takes more arguments than
        // its prototype has.
        let fits = |n: usize| -> NResult<()> {
            if n > pr.params.len() {
                anyhow::bail!(
                    "{name}: {n} parameters passed, it takes {}",
                    pr.params.len()
                );
            }
            Ok(())
        };
        if !system_named && self.m.prog.procs.contains_key(&target) {
            fits(args.len())?;
            return self.call_frame(&target, args);
        }
        let bound = if system_named {
            None
        } else {
            self.m
                .io
                .binding
                .as_ref()
                .and_then(|b| b.imports.get(&sym))
                .cloned()
        };
        let as_arg = |arg: CallArg| match arg {
            CallArg::Ref { buf, off, dtype } => super::Arg::Ref(super::Loc {
                buf,
                off,
                size: dtype.size(),
                dtype,
            }),
            CallArg::Val(v) => super::Arg::Val(v),
        };
        if let Some(t) = bound {
            fits(args.len())?;
            let a: Vec<super::Arg> = args.into_iter().map(as_arg).collect();
            return self.m.call_bound(&sym, &t, a);
        }
        if super::cee::CEE_APIS.contains(&sym.as_str()) {
            fits(args.len())?;
            let a: Vec<super::Arg> = args.into_iter().map(as_arg).collect();
            let v = self
                .m
                .cee_call(&sym, &a)
                .ok_or_else(|| anyhow!("native: {sym} is not a CEE API"))??;
            return Ok(Some(v));
        }
        if super::crt::C_FUNCTIONS.contains(&sym.as_str()) {
            // The prototype's arguments as args_for gives them, CONST ones
            // passed by reference as their values (an output buffer as its
            // address, C_OUT_BUFFERS); the rest as values (printf's list).
            let n = args.len().min(pr.params.len());
            let mut a = Vec::with_capacity(args.len());
            for (k, arg) in args.into_iter().enumerate() {
                let konst = k < n && pr.params.get(k).is_some_and(|p| p.konst);
                // OPTIONS(*STRING) (S2.66, args_for): a character value for
                // a pointer parameter goes as a pointer to a copy of it with
                // a null byte after, in the heap until the call ends.
                let string = k < n
                    && pr
                        .params
                        .get(k)
                        .is_some_and(|p| p.string && p.dtype == data::DType::Pointer);
                if let (true, CallArg::Val(Value::Char(chars))) = (string, &arg) {
                    let mut bytes = chars.clone();
                    bytes.push(0);
                    let h = self.m.bufs.heap.alloc(bytes.len());
                    if let Some(b) = self.m.bufs.heap.blocks.get_mut(h) {
                        *b = bytes;
                    }
                    self.m.temp_strings.push(h);
                    let at = rpgrt::rpg::mem::HEAP_BASE
                        .checked_add(h)
                        .ok_or_else(|| anyhow!("native: a heap block"))?;
                    a.push(super::Arg::Val(self.addr_of(at, 0)));
                    continue;
                }
                a.push(match arg {
                    CallArg::Ref { buf, off, dtype } if konst => {
                        if super::crt::C_OUT_BUFFERS.contains(&(sym.as_str(), k)) {
                            super::Arg::Val(self.addr_of(buf, off))
                        } else {
                            let end = off
                                .checked_add(dtype.size())
                                .ok_or_else(|| anyhow!("native: an argument's size"))?;
                            let bytes = self
                                .m
                                .bufs
                                .get(buf)
                                .and_then(|b| b.get(off..end))
                                .ok_or_else(|| anyhow!("native: an argument's storage"))?;
                            super::Arg::Val(data::load(&dtype, bytes)?)
                        }
                    }
                    other => as_arg(other),
                });
            }
            let v = self
                .m
                .c_call(&sym, &a)
                .ok_or_else(|| anyhow!("native: {sym} is not a C function"))??;
            return Ok(Some(v));
        }
        anyhow::bail!(
            "CPD5D02: Definition for procedure {target} not found: it is not in this module, and the program is not bound to a module or service program that exports it (CRTPGM, CRTSRVPGM, BNDDIR)."
        )
    }

    /// The values of VALUE and CONST parameters, then the procedure's
    /// calculations: its generated code, with the error noted as `guarded`
    /// and `recover` do for a procedure with no *PSSR.
    fn call_body(
        &mut self,
        pi: &'a super::super::program::ProcInfo,
        values: Vec<(String, Value)>,
    ) -> NResult<Option<Value>> {
        for (n, v) in values {
            let Some(super::super::program::Sym::Var(var)) = pi.syms.get(&n) else {
                anyhow::bail!("native: parameter {n} of {} has no storage", pi.name);
            };
            let (b, o) = self.local(var.buf, var.off)?;
            rpgrt::native::store(
                &mut self.m.bufs,
                b,
                o,
                var.dtype,
                &v,
                Fit::Strict { half_adjust: false },
                false,
            )?;
            // call_proc assigns each value with `assign`, which makes room
            // for the procedure's variables based on a pointer (S2.51:
            // room_for_based, with the new frame the last).
            if var.dtype == data::DType::Pointer {
                self.pointer_set(&n, &v);
            }
        }
        let detail = &pi.routines.detail;
        let r = match dispatch(self.m, detail, 0) {
            Some(r) => r,
            // AS4TAINR_RPG_ENGINE=auto and no generated code for the
            // procedure: the interpreter runs it, as it would any routine.
            None => self.m.run_code(detail),
        };
        match r {
            Ok(Flow::Return(v)) => Ok(v),
            Ok(_) => Ok(None),
            Err(e) => {
                if !errors::is_signal(&e) && !self.passes_by_n(&e) {
                    self.note_error_n(&e);
                }
                Err(e)
            }
        }
    }
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
        let key = (self.code.as_ptr().addr(), pc, self.m.frames.len());
        self.m.each.insert(key, (vals, 0));
        Ok(())
    }

    fn each_next(&mut self, pc: usize) -> NResult<bool> {
        let Ins::EachNext { var, init, .. } = &at(self.code, pc)?.ins else {
            return Err(mismatch(pc, "a FOR-EACH step"));
        };
        let key = (self.code.as_ptr().addr(), *init, self.m.frames.len());
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
        // The subroutine's generated code (S2.40, ADR 0007); the
        // interpreter only under auto with none, as for any routine
        // (run_code is the same dispatch first).
        let r = match dispatch(self.m, sub, 0) {
            Some(r) => r,
            None => self.m.run_code(sub),
        };
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
                let buf = rpgrt::native::storage(&self.m.bufs, d.buf);
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

    fn native_ok(&self) -> bool {
        self.ok
    }

    fn local(&mut self, buf: usize, off: usize) -> NResult<(usize, usize)> {
        // As `Machine::sym` places a local: a parameter passed by
        // reference is the caller's storage, anything else the frame's.
        let f = self
            .m
            .frames
            .last()
            .ok_or_else(|| anyhow!("native: a procedure local with no frame"))?;
        let (b, o) = match f.alias.get(&buf) {
            Some((b, o)) => (
                *b,
                off.checked_add(*o)
                    .ok_or_else(|| anyhow!("native: a local's offset"))?,
            ),
            None => (
                f.base
                    .checked_add(buf)
                    .ok_or_else(|| anyhow!("native: a local's buffer"))?,
                off,
            ),
        };
        self.m.check_buf(b)?;
        Ok((b, o))
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

    fn call(&mut self, pc: usize, proc: &str, args: Vec<CallArg>) -> NResult<Option<Value>> {
        // S2.35 (ADR 0007): the fork's own procedure call, for what
        // lowering lets through (a procedure of this module with no *PSSR,
        // ON-EXIT, local files or pointer initializations, arguments of
        // the parameters' own types). It does, in their order, what the
        // interpreter's call_named and call_proc do; it does not call them.
        let _ = pc;
        let job = self.m.io.job_key.clone();
        crate::pgmmsg::stack_line(&job, self.m.line);
        // The OPTIONS(*STRING) copies made for this call (S2.66).
        let mark = self.m.temp_strings.len();
        // AS4TAINR_TRACE_CALLS=1, as call_named prints it.
        let trace = std::env::var("AS4TAINR_TRACE_CALLS").is_ok();
        if trace {
            // A diagnostic that cannot be written is dropped (section 5:
            // writing can fail; eprintln! would panic).
            let _ = writeln!(
                std::io::stderr(),
                "[call] {} line {} -> {proc}",
                self.m.prog.name,
                self.m.line
            );
        }
        // A procedure of this module (S2.35), or a prototype (S2.46).
        let local = self.m.prog.procs.contains_key(proc);
        let r = if local {
            self.call_frame(proc, args)
        } else {
            self.call_proto(proc, args)
        }
        .map_err(|mut e| {
            // An error the called procedure did not handle comes out of
            // the call: 00202 where it is caught (call_named).
            if let Some(at) = e.downcast_mut::<errors::At>() {
                at.called = true;
            }
            e
        });
        if trace {
            if let Ok(Some(v)) = &r {
                let shown = match v {
                    Value::Char(c) if c.len() > 40 => {
                        format!("'{}...'", a(c.get(..40).unwrap_or_default()))
                    }
                    v => v.describe(),
                };
                let _ = writeln!(std::io::stderr(), "[return] {proc} = {shown}");
            }
        }
        // A call that ends normally leaves the program status 00000, with
        // PSDS positions 11-15 (call_named; PUB400 verify34).
        if r.is_ok() && self.m.last.code != 0 && self.m.last.code < 1000 {
            self.m.last.code = 0;
            if let Some(ds) = self.m.prog.psds.clone() {
                self.m.ds_zoned(&ds, 11, 5, 0);
            }
        }
        // call_named: the OPTIONS(*STRING) copies last as long as the call
        // (S2.66).
        let made = if mark < self.m.temp_strings.len() {
            self.m.temp_strings.split_off(mark)
        } else {
            Vec::new()
        };
        for h in made {
            if let (Some(f), Some(b)) = (
                self.m.bufs.heap.freed.get_mut(h),
                self.m.bufs.heap.blocks.get_mut(h),
            ) {
                *f = true;
                *b = Vec::new();
            }
        }
        r
    }

    fn ind(&self, name: &str) -> bool {
        // Machine::ind: the indicator's value, off when never set.
        self.m.inds.get(&ind_name(name)).copied().unwrap_or(false)
    }

    fn set_ind(&mut self, name: &str, on: bool) {
        // Machine::set_ind, step by step: an overflow indicator coming on
        // makes its file's overflow lines due again.
        let n = ind_name(name);
        if on && !self.m.ofl_inds.is_empty() && self.m.inds.get(&n) != Some(&true) {
            for (ind, file) in &self.m.ofl_inds {
                if *ind == n {
                    self.m.ofl_written.remove(file);
                    if self.m.in_detail {
                        self.m.of_late.insert(n.clone());
                    }
                }
            }
        }
        self.m.inds.insert(n, on);
    }

    fn addr_of(&self, buf: usize, off: usize) -> Value {
        // ptrs.rs ptr_to, step by step: lent storage belongs to the
        // lender's activation (globals) or call (locals), the heap to no
        // one, this module's globals to its activation and the rest to
        // this call's frame.
        use rpgrt::rpg::mem::{Ptr, BAD_BUF};
        if buf == BAD_BUF {
            return Value::Fig(data::Fig::Null);
        }
        let (Ok(o), Some(p)) = (u32::try_from(off), self.ptr_parts(buf)) else {
            return Value::Fig(data::Fig::Null);
        };
        let (kind, owner, b) = p;
        Value::Ptr(Ptr {
            kind,
            owner,
            buf: b,
            off: o,
        })
    }

    fn based_at(&mut self, p: &Value, off: usize, size: usize) -> NResult<(usize, usize)> {
        // ptrs.rs based_place: addr(pointer, off + size), then off on.
        let need = off
            .checked_add(size)
            .ok_or_else(|| anyhow!("native: a based field's size"))?;
        let (b, o) = self.addr_n(p, need)?;
        let o = o
            .checked_add(off)
            .ok_or_else(|| anyhow!("native: a based field's offset"))?;
        Ok((b, o))
    }

    fn alloc(&mut self, n: &Dec) -> NResult<Value> {
        // ptrs.rs alloc.
        let n = n.to_i128()?;
        if !(1..=16_776_704).contains(&n) {
            anyhow::bail!("RNQ0425: {n} bytes cannot be allocated (1 to 16,776,704 bytes).");
        }
        let h = self.m.bufs.heap.alloc(usize::try_from(n)?);
        let buf = rpgrt::rpg::mem::HEAP_BASE
            .checked_add(h)
            .ok_or_else(|| anyhow!("native: a heap address"))?;
        Ok(self.addr_of(buf, 0))
    }

    fn realloc(&mut self, p: &Value, n: &Dec) -> NResult<Value> {
        // ptrs.rs realloc.
        use rpgrt::rpg::mem::{HEAP_BASE, LENT_BASE};
        let n = n.to_i128()?;
        if !(1..=16_776_704).contains(&n) {
            anyhow::bail!("RNQ0425: {n} bytes cannot be allocated (1 to 16,776,704 bytes).");
        }
        let (buf, off) = self.addr_n(p, 0)?;
        if !(HEAP_BASE..LENT_BASE).contains(&buf) || off != 0 {
            anyhow::bail!(
                "RNQ0426: the pointer for %REALLOC does not address heap storage from ALLOC or %ALLOC."
            );
        }
        let n = usize::try_from(n)?;
        self.m
            .bufs
            .get_mut(buf)
            .ok_or_else(|| anyhow!("native: heap storage {buf}"))?
            .resize(n, 0);
        Ok(p.clone())
    }

    fn dealloc(&mut self, p: &Value) -> NResult<()> {
        // ptrs.rs dealloc.
        use rpgrt::rpg::mem::{HEAP_BASE, LENT_BASE};
        if matches!(p, Value::Fig(data::Fig::Null)) {
            return Ok(());
        }
        let (buf, off) = self.addr_n(p, 0)?;
        if !(HEAP_BASE..LENT_BASE).contains(&buf) || off != 0 {
            anyhow::bail!(
                "RNQ0426: the pointer for DEALLOC does not address heap storage from ALLOC or %ALLOC."
            );
        }
        let h = buf
            .checked_sub(HEAP_BASE)
            .ok_or_else(|| anyhow!("native: a heap address"))?;
        let freed = self
            .m
            .bufs
            .heap
            .freed
            .get_mut(h)
            .ok_or_else(|| anyhow!("native: heap storage {h}"))?;
        if *freed {
            anyhow::bail!("RNQ0426: the heap storage for DEALLOC was already freed.");
        }
        *freed = true;
        Ok(())
    }

    fn pointer_set(&mut self, name: &str, v: &Value) {
        // interp.rs assign: a storage or heap pointer stored into a name.
        if let Value::Ptr(ptr) = v {
            if matches!(
                ptr.kind,
                rpgrt::rpg::mem::KIND_STORAGE | rpgrt::rpg::mem::KIND_HEAP
            ) {
                self.room_for_based_n(name, ptr);
            }
        }
    }

    fn elems(&self, name: &str) -> u32 {
        // bifs.rs elems_in_use for a varying-dimension array: the count
        // set, 0 at first.
        self.m.var_elems.get(name).copied().unwrap_or(0)
    }

    fn set_elem_count(&mut self, name: &str, n: u32) {
        self.m.var_elems.insert(name.to_string(), n);
    }

    fn file_read(
        &mut self,
        pc: usize,
        keys: Option<Vec<KeyArg>>,
    ) -> NResult<Option<Vec<Option<String>>>> {
        self.file_read_n(pc, keys)
    }

    fn file_done(&mut self, pc: usize, moved: NResult<()>) -> NResult<()> {
        self.file_done_n(pc, moved)
    }

    fn file_change_start(&mut self, pc: usize) -> NResult<()> {
        self.file_change_start_n(pc)
    }

    fn file_change(
        &mut self,
        pc: usize,
        keys: Option<Vec<KeyArg>>,
        rec: NResult<Vec<Option<String>>>,
    ) -> NResult<()> {
        self.file_change_n(pc, keys, rec)
    }

    fn occurrence(&self, buf: usize) -> NResult<Dec> {
        // ops.rs occurrence: the current occurrence of the data
        // structure (occurs_of's buffer; lowering names only those).
        let (_, _, cur) = self.m.occur.get(&buf).ok_or_else(|| {
            anyhow!("native: buffer {buf} is not a multiple-occurrence data structure")
        })?;
        Ok(Dec::from_i64(i64::from(*cur)))
    }

    fn set_occurrence(&mut self, buf: usize, n: &Dec) -> NResult<()> {
        // ops.rs set_occurrence, with the number as its callers take it
        // (`num()?.to_i128()?`: a whole number).
        let n = n.to_i128()?;
        let (_, count, cur) = self.m.occur.get_mut(&buf).ok_or_else(|| {
            anyhow!("native: buffer {buf} is not a multiple-occurrence data structure")
        })?;
        if n < 1 || n > i128::from(*count) {
            return Err(anyhow!(
                "RNQ0122: The occurrence {n} is outside the data structure ({count} occurrences)."
            ));
        }
        *cur = u32::try_from(n)?;
        Ok(())
    }

    fn file_status(&mut self, bit: FileBit, file: Option<&str>) -> NResult<bool> {
        // interp.rs bif "EOF"/"FOUND"/"EQUAL" and file_status.
        let st = match file {
            None => &self.m.last,
            Some(n) => {
                let i = self
                    .file_idx_n(n)
                    .ok_or_else(|| anyhow!("{n} is not a file"))?;
                &self
                    .m
                    .files
                    .get(i)
                    .ok_or_else(|| anyhow!("native: no file {i}"))?
                    .status
            }
        };
        Ok(match bit {
            FileBit::Eof => st.eof,
            FileBit::Found => st.found,
            FileBit::Equal => st.equal,
        })
    }

    fn dsply(&mut self, text: String) {
        // Stmt::Dsply with no response: `messages.push(text.trim_end())`.
        self.m.messages.push(text.trim_end().to_string());
    }

    fn status(&self) -> u32 {
        // bifs.rs "STATUS" with no argument: `self.last`'s code.
        self.m.last.code
    }

    fn remainder(&self) -> Dec {
        self.m.remainder
    }

    fn set_remainder(&mut self, d: Dec) {
        self.m.remainder = d;
    }

    fn precision(&mut self, pc: usize, path: &[u8]) -> Option<(u32, u32)> {
        // The expression the path starts from: an EVAL's value, or a
        // condition (S2.14).
        // A path into the target starts with TARGET_PATH (S2.44).
        let (into_target, path) = match path.split_first() {
            Some((&rpgrt::native::TARGET_PATH, rest)) => (true, rest),
            _ => (false, path),
        };
        let mut node = match self.code.get(pc).map(|i| &i.ins) {
            Some(Ins::Stmt(super::super::ast::Stmt::Eval { target, .. })) if into_target => target,
            Some(Ins::Stmt(super::super::ast::Stmt::Eval { value, .. })) => value,
            Some(Ins::JumpIfNot(c, _)) => c,
            // A CALLP's arguments (S2.35).
            Some(Ins::Stmt(super::super::ast::Stmt::CallP(e))) => e,
            // A RETURN's value (S2.38).
            Some(Ins::Return(Some(v))) => v,
            // A DSPLY's message (S2.36).
            Some(Ins::Stmt(super::super::ast::Stmt::Dsply {
                message: Some(m), ..
            })) => m,
            _ => return None,
        };
        // Each step: 0 or 1 into an operation's operands, or the index of
        // a built-in function's or a procedure call's argument.
        for step in path {
            node = match node {
                Expr::Bin(_, l, r) => {
                    if *step == 0 {
                        l
                    } else {
                        r
                    }
                }
                Expr::Bif(_, args) => args.get(usize::from(*step))?,
                Expr::Name {
                    args: Some(args), ..
                } => args.get(usize::from(*step))?,
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
        if self.passes_by_n(&err) {
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
                // A group none of whose handlers takes the error is left,
                // and the enclosing group gets it (run_code_at, upstream
                // #10).
                if handled.is_some() {
                    break;
                }
            }
            monitors.pop();
        }
        let line = self.code.get(pc).map(|i| i.line).unwrap_or(0);
        match handled {
            Some(h) => {
                self.note_error_n(&err);
                Settled::Go(h)
            }
            None if line == 0 => Settled::Exit(Err(err)),
            // Where the error happened, for its exception message's sender
            // (shown as "line N").
            None => {
                let first = errors::origin(&err);
                let called = first.as_ref().is_some_and(|a| a.called);
                let sender = first.map(|a| a.sender).unwrap_or_else(|| self.sender_n());
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
    // A pointer's owner is the number of the activation it was taken in,
    // which differs between two runs in one process (S2.47): compared by
    // kind, buffer and offset only (bytes 4 to 8 of each pointer cleared).
    for sym in prog.syms.values() {
        let super::super::program::Sym::Var(v) = sym else {
            continue;
        };
        if v.local || v.dtype != data::DType::Pointer {
            continue;
        }
        let n = if v.dim == 0 { 1 } else { v.dim };
        for k in 0..n {
            let at = usize::try_from(k)
                .ok()
                .and_then(|k| k.checked_mul(v.stride))
                .and_then(|d| v.off.checked_add(d))
                .and_then(|o| o.checked_add(4));
            if let Some((s, e)) = at.and_then(|s| s.checked_add(4).map(|e| (s, e))) {
                if let Some(bytes) = globals.get_mut(v.buf).and_then(|b| b.get_mut(s..e)) {
                    bytes.fill(0);
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

/// An indicator's name as the machine keeps it (the interpreter's
/// `ind_name`): upper case, and a number of up to two digits with two.
fn ind_name(n: &str) -> String {
    let t = n.trim().to_ascii_uppercase();
    match t.parse::<u32>() {
        Ok(v) if t.len() <= 2 => format!("{v:02}"),
        _ => t,
    }
}

// ---- the error path (S2.43, ADR 0007) ----------------------------------
//
// Written again from the interpreter's errors.rs (note_error,
// psds_error, passes_by and the status data structure helpers) and
// interp.rs (sender), step by step; native_tripwire.rs watches them. A
// file error (status 1000 and up) still goes to the interpreter's
// note_error: its file information comes from the interpreter's file
// state, which phase A's file operations replace.
impl Rt<'_, '_> {
    /// errors.rs passes_by: an escape message sent to a caller passes this
    /// procedure by.
    fn passes_by_n(&self, err: &anyhow::Error) -> bool {
        err.downcast_ref::<crate::pgmmsg::Escape>()
            .is_some_and(|e| {
                e.passes(
                    &self.m.io.job_key,
                    crate::pgmmsg::stack_top_id(&self.m.io.job_key),
                )
            })
    }

    /// interp.rs sender: the call stack entry that sends a message.
    fn sender_n(&self) -> crate::pgmmsg::Sender {
        let s = crate::pgmmsg::stack(&self.m.io.job_key);
        match s.last() {
            Some(e) => crate::pgmmsg::Sender {
                pgm: e.pgm.clone(),
                lib: crate::qtemp::logical(&e.lib),
                module: e.module.clone(),
                proc: e.proc.clone(),
                line: self.m.line,
                inst: String::new(),
            },
            None => crate::pgmmsg::Sender {
                pgm: self.m.stack_pgm(),
                lib: crate::qtemp::logical(&self.m.stack_lib()),
                module: self.m.prog.name.clone(),
                proc: self.m.prog.name.clone(),
                line: self.m.line,
                inst: String::new(),
            },
        }
    }

    /// errors.rs note_error: %STATUS and %ERROR as the operation left
    /// them, the PSDS's exception, and the exception message in the call
    /// message queue of the procedure that handles it.
    fn note_error_n(&mut self, err: &anyhow::Error) -> u32 {
        let code = errors::status_code(err);
        if code >= 1000 {
            // File errors: the interpreter's, for its file state (above).
            return self.m.note_error(err);
        }
        if std::env::var("AS4TAINR_TRACE_ERRORS").is_ok() {
            let _ = writeln!(
                std::io::stderr(),
                "[handled in {} at line {}] {err:#}",
                self.m.prog.name,
                self.m.line
            );
        }
        self.m.last = super::Status {
            error: true,
            code,
            ..super::Status::default()
        };
        self.psds_error_n(code, err);
        let job = self.m.io.job_key.clone();
        if let Some(id) = crate::pgmmsg::stack_target(&job, "*", 0) {
            let (m, logged) = match err.downcast_ref::<crate::pgmmsg::Escape>() {
                Some(e) => (e.0.clone(), e.0.key != 0),
                None => {
                    let msgid = errors::message_id(err);
                    let mut m = crate::pgmmsg::from_error(err, &msgid);
                    m.data = data::e(&m.text);
                    let at = errors::origin(err)
                        .map(|a| a.sender)
                        .unwrap_or_else(|| self.sender_n());
                    m.receiver.line = at.line;
                    if msgid.starts_with("RNX") {
                        m.logged_sender = Some(crate::pgmmsg::rpg_runtime_sender());
                    }
                    m.sender = at;
                    (m, false)
                }
            };
            if logged {
                crate::pgmmsg::stack_send_unlogged(&job, id, m);
            } else if let Some(k) = crate::pgmmsg::stack_send(&job, id, m) {
                self.m.logged.push(k);
            }
        }
        code
    }

    /// errors.rs psds_error for a program error (status below 1000): the
    /// previous status to 16-20, the status to 11-15, the statement, the
    /// routine, the exception ID and its data.
    fn psds_error_n(&mut self, code: u32, err: &anyhow::Error) {
        let Some(ds) = self.m.prog.psds.clone() else {
            return;
        };
        let prev = self.ds_get_n(&ds, 11, 5);
        if prev.len() == 5 {
            self.ds_put_n(&ds, 15, &prev);
        }
        self.ds_zoned_n(&ds, 11, 5, i64::from(code));
        let stmt = format!("{:>8}", self.m.line);
        self.ds_text_n(&ds, 21, 8, &stmt);
        let routine = self.m.routine.clone();
        self.ds_text_n(&ds, 29, 8, &routine);
        let id = errors::message_id(err);
        self.ds_text_n(&ds, 40, 7, &id);
        let root = err.root_cause().to_string();
        let text = root
            .split_once(": ")
            .map(|(_, t)| t.to_string())
            .unwrap_or(root);
        self.ds_text_n(&ds, 91, 80, text.lines().next().unwrap_or(""));
    }

    /// errors.rs ds_put: bytes into a global data structure at an offset;
    /// what falls past its end is not kept.
    fn ds_put_n(&mut self, ds: &str, off: usize, bytes: &[u8]) {
        let Some(super::super::program::Sym::Ds(d)) = self.m.prog.syms.get(ds) else {
            return;
        };
        let Some(room) = d.size.checked_sub(off).filter(|r| *r > 0) else {
            return;
        };
        let n = bytes.len().min(room);
        let Some(at) = d.off.checked_add(off) else {
            return;
        };
        let Some(end) = at.checked_add(n) else {
            return;
        };
        if let (Some(dst), Some(src)) = (
            self.m.bufs.get_mut(d.buf).and_then(|b| b.get_mut(at..end)),
            bytes.get(..n),
        ) {
            dst.copy_from_slice(src);
        }
    }

    /// errors.rs ds_get.
    fn ds_get_n(&self, ds: &str, off: usize, len: usize) -> Vec<u8> {
        let Some(super::super::program::Sym::Ds(d)) = self.m.prog.syms.get(ds) else {
            return Vec::new();
        };
        let end = off.saturating_add(len).min(d.size);
        if off >= end {
            return Vec::new();
        }
        let (Some(a), Some(b)) = (d.off.checked_add(off), d.off.checked_add(end)) else {
            return Vec::new();
        };
        self.m
            .bufs
            .get(d.buf)
            .and_then(|x| x.get(a..b))
            .map(|x| x.to_vec())
            .unwrap_or_default()
    }

    /// errors.rs ds_text: text in EBCDIC, blank-padded or cut to `len`, at
    /// 1-based position `from`.
    fn ds_text_n(&mut self, ds: &str, from: usize, len: usize, text: &str) {
        let mut b = data::e(text);
        b.resize(len, data::BLANK);
        b.truncate(len);
        if let Some(off) = from.checked_sub(1) {
            self.ds_put_n(ds, off, &b);
        }
    }

    /// errors.rs ds_zoned: a zoned number of `digits` digits (the value
    /// modulo 10^digits) at 1-based position `from`.
    fn ds_zoned_n(&mut self, ds: &str, from: usize, digits: u32, v: i64) {
        let t = data::DType::Zoned(digits, 0);
        let mut b = t.initial();
        let Some(m) = 10i64.checked_pow(digits) else {
            return;
        };
        let v = v.rem_euclid(m);
        if data::store(
            &t,
            &Value::Num(Dec::from_i64(v)),
            &mut b,
            Fit::Truncate { half_adjust: false },
            false,
        )
        .is_ok()
        {
            if let Some(off) = from.checked_sub(1) {
                self.ds_put_n(ds, off, &b);
            }
        }
    }
}

impl Rt<'_, '_> {
    /// ptr_to's kind, owner and buffer number for an absolute buffer
    /// (S2.47); None when it does not fit a pointer's 32-bit fields.
    fn ptr_parts(&self, buf: usize) -> Option<(u8, u32, u32)> {
        use rpgrt::rpg::mem::{HEAP_BASE, KIND_HEAP, KIND_STORAGE, LENT_BASE};
        if buf >= LENT_BASE {
            let rel = buf.checked_sub(LENT_BASE)?;
            let k = rel >> 24;
            let b = rel & 0xFF_FFFF;
            let l = self.m.bufs.lent.get(k)?;
            let owner = if b < l.globals { l.act } else { l.owner };
            Some((KIND_STORAGE, owner, u32::try_from(b).ok()?))
        } else if buf >= HEAP_BASE {
            Some((
                KIND_HEAP,
                0,
                u32::try_from(buf.checked_sub(HEAP_BASE)?).ok()?,
            ))
        } else {
            let owner = if buf < self.m.prog.globals.len() {
                self.m.owner
            } else {
                self.m.frame_owner
            };
            Some((KIND_STORAGE, owner, u32::try_from(buf).ok()?))
        }
    }
}

// ---- pointers (S2.47), written again from ptrs.rs addr and interp.rs
// room_for_based; native_tripwire.rs watches them.
impl Rt<'_, '_> {
    /// ptrs.rs addr: the absolute buffer and offset a pointer addresses,
    /// with `size` bytes there, or MCH3601/MCH0601.
    fn addr_n(&self, v: &Value, size: usize) -> NResult<(usize, usize)> {
        use rpgrt::rpg::mem::{HEAP_BASE, KIND_HEAP, KIND_PROC, KIND_SYS, LENT_BASE};
        let p = match v {
            Value::Ptr(p) => *p,
            Value::Fig(data::Fig::Null) => anyhow::bail!(
                "MCH3601: the pointer is not set (*NULL) where storage is used through it."
            ),
            other => anyhow::bail!("a pointer is expected, not {}", other.describe()),
        };
        let pb = usize::try_from(p.buf)?;
        let buf = match p.kind {
            KIND_PROC | KIND_SYS => {
                anyhow::bail!("MCH3601: a procedure or system pointer does not address storage.")
            }
            KIND_HEAP => {
                if pb >= self.m.bufs.heap.blocks.len() {
                    anyhow::bail!("MCH3601: the pointer addresses heap storage that does not exist.");
                }
                HEAP_BASE
                    .checked_add(pb)
                    .ok_or_else(|| anyhow!("native: a heap address"))?
            }
            _ if p.owner == self.m.frame_owner => pb,
            _ if p.owner == self.m.owner && pb < self.m.prog.globals.len() => pb,
            _ => match self
                .m
                .bufs
                .lent
                .iter()
                .rposition(|l| l.owner == p.owner || (l.act == p.owner && pb < l.globals))
            {
                Some(k) => k
                    .checked_shl(24)
                    .and_then(|s| LENT_BASE.checked_add(s))
                    .and_then(|b| b.checked_add(pb))
                    .ok_or_else(|| anyhow!("native: a lent address"))?,
                None => anyhow::bail!(
                    "MCH3601: the pointer addresses storage of a program that is not running, or that this call cannot reach."
                ),
            },
        };
        let Some(b) = self.m.bufs.get(buf) else {
            anyhow::bail!("MCH3601: the pointer addresses storage that no longer exists.");
        };
        let off = usize::try_from(p.off)?;
        if off.checked_add(size).is_none_or(|end| end > b.len()) {
            anyhow::bail!(
                "MCH0601: {size} bytes at offset {} are outside the {} bytes of storage the pointer addresses.",
                p.off,
                b.len()
            );
        }
        Ok((buf, off))
    }

    /// interp.rs room_for_based: the storage a pointer now addresses gets
    /// room for the largest variable based on that pointer (the frame's
    /// and the module's), up to 16 MiB.
    fn room_for_based_n(&mut self, name: &str, p: &rpgrt::rpg::mem::Ptr) {
        let mut need = 0usize;
        if let Some(f) = self.m.frames.last() {
            for (b, ptr) in &f.proc.based {
                if ptr.eq_ignore_ascii_case(name) {
                    need = need.max(f.proc.frame.get(*b).map_or(0, |x| x.len()));
                }
            }
        }
        for (b, ptr) in &self.m.prog.based {
            if ptr.eq_ignore_ascii_case(name) {
                need = need.max(self.m.prog.globals.get(*b).map_or(0, |x| x.len()));
            }
        }
        if need == 0 {
            return;
        }
        let Ok((buf, off)) = self.addr_n(&Value::Ptr(*p), 0) else {
            return;
        };
        let Some(want) = off.checked_add(need) else {
            return;
        };
        if want > 16 * 1024 * 1024 {
            return;
        }
        if let Some(b) = self.m.bufs.get_mut(buf) {
            if b.len() < want {
                b.resize(want, 0);
            }
        }
    }
}

// ---- reads of database files (S2.58, ADR 0007) -------------------------
//
// Written again from the interpreter's errors.rs (file_op, file_failed,
// file_idx, opcode_name) and interp.rs (file_op_inner's reads,
// key_values' text of a value, fill_fields' null values), in their order;
// native_tripwire.rs watches them. The record operations themselves are
// `rla`'s, which the interpreter calls too. Lowering lets through only
// externally described DISK files with no INFDS, no result data
// structure or field list, no E extender and no error indicator, read
// with no input specifications; the moves of the fields into the program
// are generated code.
impl Rt<'_, '_> {
    /// errors.rs file_idx: the file a name refers to, one declared in the
    /// running procedure first, then the program's.
    fn file_idx_n(&self, name: &str) -> Option<usize> {
        if let Some(i) = self.m.frames.last().and_then(|f| f.files.get(name)) {
            return Some(*i);
        }
        let files = &self.m.prog.files;
        let proc = self.m.frames.last().map(|f| f.proc.name.as_str());
        let local = |f: &super::super::program::FileInfo| f.decl.proc.as_deref() == proc;
        files
            .iter()
            .position(|f| local(f) && f.decl.name == name)
            .or_else(|| files.iter().position(|f| local(f) && f.format == name))
            .or_else(|| {
                files.iter().position(|f| {
                    f.decl.proc.is_none() && (f.decl.name == name || f.format == name)
                })
            })
            .or_else(|| self.m.prog.file_index(name))
    }

    /// The file statement at `pc`.
    fn file_stmt(&self, pc: usize) -> NResult<FileStmt<'_>> {
        match &at(self.code, pc)?.ins {
            Ins::Stmt(super::super::ast::Stmt::File {
                op,
                file,
                key,
                ext,
                ind,
                ..
            }) => Ok((*op, file.as_str(), key, ext, ind)),
            _ => Err(mismatch(pc, "a file operation")),
        }
    }

    fn file_n(&mut self, i: usize) -> NResult<&mut super::FileState> {
        self.m
            .files
            .get_mut(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))
    }

    /// errors.rs file_op's start (E is refused by lowering): the error
    /// flag off, and a file under commitment control in a savepoint of
    /// the unit of work. Then file_op_inner's read, and fill_fields' null
    /// values. An error ends the operation here (file_op's end).
    fn file_read_n(
        &mut self,
        pc: usize,
        keys: Option<Vec<KeyArg>>,
    ) -> NResult<Option<Vec<Option<String>>>> {
        let (op, file, key, ext, _) = self.file_stmt(pc)?;
        let file = file.to_string();
        let no_lock = ext.no_lock;
        // *LOVAL/*START and *HIVAL/*END positioning.
        let special = match key.as_deref() {
            Some([Expr::Fig(super::super::data::Fig::LoVal)]) => Some(false),
            Some([Expr::Fig(super::super::data::Fig::HiVal)]) => Some(true),
            Some([Expr::Name { parts, .. }]) if parts.first().is_some_and(|p| p == "*START") => {
                Some(false)
            }
            Some([Expr::Name { parts, .. }]) if parts.first().is_some_and(|p| p == "*END") => {
                Some(true)
            }
            _ => None,
        };
        let i = self
            .file_idx_n(&file)
            .ok_or_else(|| anyhow!("{file} is not a file of this program"))?;
        self.file_n(i)?.status.error = false;
        let cmt = self.file_n(i)?.cmt.clone();
        if let Some(d) = &cmt {
            self.m.block(async {
                d.client
                    .batch_execute("SAVEPOINT as4tainr_rla")
                    .await
                    .map_err(anyhow::Error::from)
            })?;
        }
        let read = self.read_inner_n(i, op, &file, special, keys, no_lock, cmt.as_deref());
        let read = read.and_then(|r| {
            let rec = match &r.record {
                Some(rec) => Some(self.fields_read_n(i, rec)?),
                None => None,
            };
            Ok((r, rec))
        });
        match read {
            Ok((r, rec)) => {
                self.pending = Some(PendingRead {
                    file: i,
                    found: r.found,
                    eof: r.eof,
                    has_record: r.record.is_some(),
                });
                Ok(rec)
            }
            Err(err) => {
                self.pending = None;
                self.file_end_n(i, op, &file, cmt.as_deref(), Err(&err))?;
                Err(err)
            }
        }
    }

    /// interp.rs file_op_inner for a read of a DISK file.
    #[allow(clippy::too_many_arguments)] // file_op_inner's own inputs, one by one.
    fn read_inner_n(
        &mut self,
        i: usize,
        op: FileOp,
        file: &str,
        special: Option<bool>,
        keys: Option<Vec<KeyArg>>,
        no_lock: bool,
        cmt: Option<&crate::cmtctl::Definition>,
    ) -> NResult<crate::rla::ReadResult> {
        if self.file_n(i)?.open.is_none() {
            return Err(anyhow!("CPF4135: File {file} is not open."));
        }
        let lock = !no_lock;
        let client: &tokio_postgres::Client = match cmt {
            Some(d) => &d.client,
            None => self.m.io.data.unwrap_or(self.m.io.client),
        };
        let job = self.m.io.job_number;
        // key_values: a number as its text, characters as text.
        let keys: Option<Vec<String>> = match special {
            Some(_) => None,
            None => keys.map(|ks| {
                ks.into_iter()
                    .map(|k| match k {
                        KeyArg::Num(d) => d.to_string(),
                        KeyArg::Char(c) => a(&c),
                    })
                    .collect()
            }),
        };
        let rt = self.m.io.rt.clone();
        let f = self
            .m
            .files
            .get_mut(i)
            .and_then(|f| f.open.as_mut())
            .ok_or_else(|| anyhow!("file not open"))?;
        let given = keys.as_deref();
        let keys = given.unwrap_or(&[]);
        Ok(match op {
            FileOp::SetLL | FileOp::SetGT => {
                let eq = match (op, special) {
                    (FileOp::SetLL, Some(false)) => rt.block_on(f.setll(client, &[]))?,
                    (FileOp::SetGT, Some(true)) | (FileOp::SetLL, Some(true)) => {
                        rt.block_on(f.setgt(client, &[]))?;
                        false
                    }
                    (FileOp::SetGT, Some(false)) => rt.block_on(f.setll(client, &[]))?,
                    (FileOp::SetLL, None) => rt.block_on(f.setll(client, keys))?,
                    _ => rt.block_on(f.setgt(client, keys))?,
                };
                crate::rla::ReadResult {
                    found: eq,
                    eof: false,
                    record: None,
                    rrn: None,
                }
            }
            FileOp::Chain => {
                let r = rt.block_on(f.chain(client, keys))?;
                rt.block_on(f.lock_after_read(client, r, lock, job))?
            }
            FileOp::Read | FileOp::ReadP | FileOp::ReadE | FileOp::ReadPE => {
                // READE/READPE: the key's values, None with no key or a
                // *LOVAL/*HIVAL/*START/*END one.
                let r = match op {
                    FileOp::Read => rt.block_on(f.read(client))?,
                    FileOp::ReadP => rt.block_on(f.read_previous(client))?,
                    FileOp::ReadE => rt.block_on(f.read_equal(client, given))?,
                    _ => rt.block_on(f.read_previous_equal(client, given))?,
                };
                rt.block_on(f.lock_after_read(client, r, lock, job))?
            }
            _ => return Err(anyhow!("native: {op:?} is not a read")),
        })
    }

    /// interp.rs fill_fields up to the moves: null values (ALWNULL(*NO):
    /// a data mapping error; *USRCTL: the null indicators), then the
    /// record's fields by their place in the record format.
    fn fields_read_n(
        &mut self,
        i: usize,
        rec: &crate::rla::Record,
    ) -> NResult<Vec<Option<String>>> {
        let prog: &super::super::program::Program = self.m.prog;
        let fi = prog
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?;
        let fref = fi
            .fref
            .as_ref()
            .ok_or_else(|| anyhow!("native: file {} has no description", fi.decl.name))?;
        let nulls: Vec<String> = crate::rla::null_fields(rec)
            .into_iter()
            .filter(|n| {
                fref.file
                    .fields
                    .iter()
                    .any(|f| &f.name == n && !f.data_type.is_lob())
            })
            .collect();
        if let (Some(first), super::AlwNull::No) = (nulls.first(), self.m.alwnull) {
            return Err(anyhow!(
                "CPF5035: Data mapping error on file {}: field {} has the null value, and the program does not allow null values (ALWNULL(*NO)).",
                fref.file.name,
                first
            ));
        }
        if self.m.alwnull == super::AlwNull::UsrCtl {
            for (fd, pname) in fref.file.fields.iter().zip(&fi.fields) {
                if fd.null_capable {
                    if nulls.contains(&fd.name) {
                        self.m.nullind.insert(pname.clone());
                    } else {
                        self.m.nullind.remove(pname);
                    }
                }
            }
        }
        Ok(fref
            .file
            .fields
            .iter()
            .map(|fd| rec.get(&fd.name).cloned())
            .collect())
    }

    /// The rest of file_op_inner (the status and the result indicators)
    /// and file_op's end, after the moves.
    fn file_done_n(&mut self, pc: usize, moved: NResult<()>) -> NResult<()> {
        let (op, file, _, _, ind) = self.file_stmt(pc)?;
        let file = file.to_string();
        let ind = ind.clone();
        let p = self
            .pending
            .take()
            .ok_or_else(|| anyhow!("native: {op:?} {file} ended with no read"))?;
        let i = p.file;
        let cmt = self.file_n(i)?.cmt.clone();
        if let Err(err) = moved {
            self.file_end_n(i, op, &file, cmt.as_deref(), Err(&err))?;
            return Err(err);
        }
        match op {
            FileOp::SetLL | FileOp::SetGT => {
                let st = super::Status {
                    found: p.found,
                    equal: op == FileOp::SetLL && p.found,
                    eof: false,
                    error: false,
                    ..super::Status::default()
                };
                self.file_n(i)?.status = st.clone();
                self.m.last = st;
                if let Some(eq) = &ind.eq {
                    self.m.set_ind(eq, p.found && op == FileOp::SetLL);
                }
            }
            _ => {
                let st = if op == FileOp::Chain {
                    super::Status {
                        found: p.found,
                        eof: false,
                        equal: false,
                        error: false,
                        ..super::Status::default()
                    }
                } else {
                    super::Status {
                        found: p.has_record,
                        eof: p.eof,
                        equal: false,
                        error: false,
                        ..super::Status::default()
                    }
                };
                self.file_n(i)?.status = st.clone();
                self.m.last = st;
                if op == FileOp::Chain {
                    if let Some(hi) = &ind.hi {
                        self.m.set_ind(hi, !p.found);
                    }
                } else if let Some(eq) = &ind.eq {
                    self.m.set_ind(eq, p.eof);
                }
            }
        }
        self.file_end_n(i, op, &file, cmt.as_deref(), Ok(()))
    }

    /// errors.rs file_op after file_op_inner: the savepoint released (or
    /// rolled back to), the counters, the status code (00011 end of file,
    /// 00012 not found) or the error's (file_failed), the operation and
    /// record names. The file has no INFDS (lowering), so infds_update
    /// has nothing to do.
    fn file_end_n(
        &mut self,
        i: usize,
        op: FileOp,
        file: &str,
        cmt: Option<&crate::cmtctl::Definition>,
        r: Result<(), &anyhow::Error>,
    ) -> NResult<()> {
        if let Some(d) = cmt {
            self.m.block(async {
                if d.client
                    .batch_execute("RELEASE SAVEPOINT as4tainr_rla")
                    .await
                    .is_err()
                {
                    d.client
                        .batch_execute("ROLLBACK TO SAVEPOINT as4tainr_rla")
                        .await?;
                }
                Ok::<(), anyhow::Error>(())
            })?;
        }
        let prog: &super::super::program::Program = self.m.prog;
        let fi = prog
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?;
        let by_name = fi.decl.name == file;
        let record = if fi.fref.is_some() || fi.display.is_some() {
            if by_name {
                fi.format.clone()
            } else {
                file.to_string()
            }
        } else {
            String::new()
        };
        let f = self.file_n(i)?;
        match op {
            FileOp::Write => f.writes = f.writes.saturating_add(1),
            FileOp::Read
            | FileOp::ReadE
            | FileOp::ReadP
            | FileOp::ReadPE
            | FileOp::Chain
            | FileOp::ReadC
            | FileOp::Exfmt => f.reads = f.reads.saturating_add(1),
            _ => f.others = f.others.saturating_add(1),
        }
        match r {
            Err(err) if !errors::is_signal(err) => {
                // errors.rs file_failed.
                let code = errors::status_code(err);
                let st = &mut self.file_n(i)?.status;
                st.error = true;
                st.code = code;
                st.msgid = errors::message_id(err);
                self.m.err_file = Some(i);
                self.m.last = super::Status {
                    error: true,
                    code,
                    ..super::Status::default()
                };
            }
            Err(_) => {}
            Ok(()) if self.file_n(i)?.status.error => {}
            Ok(()) => {
                let st = &self.file_n(i)?.status;
                let code = match op {
                    FileOp::Read
                    | FileOp::ReadE
                    | FileOp::ReadP
                    | FileOp::ReadPE
                    | FileOp::ReadC
                        if st.eof =>
                    {
                        11
                    }
                    FileOp::Chain | FileOp::SetLL | FileOp::SetGT if !st.found => 12,
                    _ => 0,
                };
                let st = &mut self.file_n(i)?.status;
                st.code = code;
                st.msgid.clear();
                self.m.last.code = code;
            }
        }
        let st = &mut self.file_n(i)?.status;
        st.opcode = format!(
            "{:<5}{}",
            opcode_name_n(op),
            if by_name { 'F' } else { 'R' }
        );
        st.record = record;
        Ok(())
    }
}

// ---- WRITE, UPDATE and DELETE of database files (S2.59, ADR 0007) -----
//
// Written again from errors.rs file_op and interp.rs file_op_inner (the
// change arm), record_of's null values, record_change and rla_change, in
// their order; native_tripwire.rs watches them. A file with trigger
// programs goes to the interpreter's triggered_change, which calls them:
// an external program call, as the other calls of programs (the user's
// choice, 2026-11-16), to be written again in phase B. Lowering lets
// through what it lets through for reads (S2.58).
impl Rt<'_, '_> {
    /// file_op's start, file_op_inner's open check, and the authority
    /// check of the change arm (*CHANGE on the file). An error ends the
    /// operation (file_op's end).
    fn file_change_start_n(&mut self, pc: usize) -> NResult<()> {
        let (op, file, _, _, _) = self.file_stmt(pc)?;
        let file = file.to_string();
        let i = self
            .file_idx_n(&file)
            .ok_or_else(|| anyhow!("{file} is not a file of this program"))?;
        self.file_n(i)?.status.error = false;
        let cmt = self.file_n(i)?.cmt.clone();
        if let Some(d) = &cmt {
            self.m.block(async {
                d.client
                    .batch_execute("SAVEPOINT as4tainr_rla")
                    .await
                    .map_err(anyhow::Error::from)
            })?;
        }
        let r = (|| -> NResult<()> {
            if self.file_n(i)?.open.is_none() {
                return Err(anyhow!("CPF4135: File {file} is not open."));
            }
            let fref = self.fref_n(i)?;
            self.m.block(crate::authority::check(
                self.m.io.client,
                &self.m.io.user,
                &fref.library,
                &fref.file.name,
                "*FILE",
                crate::authority::Aut::Change,
            ))
        })();
        match r {
            Ok(()) => {
                self.pending = Some(PendingRead {
                    file: i,
                    found: false,
                    eof: false,
                    has_record: false,
                });
                Ok(())
            }
            Err(err) => {
                self.pending = None;
                self.file_end_n(i, op, &file, cmt.as_deref(), Err(&err))?;
                Err(err)
            }
        }
    }

    /// interp.rs fref_of: the file as it was opened, or as compiled.
    fn fref_n(&self, i: usize) -> NResult<crate::catalog::FileRef> {
        let f = self
            .m
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?;
        let fi = self
            .m
            .prog
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?;
        f.open
            .as_ref()
            .map(|o| o.file.clone())
            .or_else(|| fi.fref.clone())
            .ok_or_else(|| anyhow!("CPF4101: File {} is not open.", fi.decl.name))
    }

    /// The change arm of file_op_inner after the record is made: the
    /// record (record_of's fields and null values; none for DELETE), the
    /// change (record_change), %ERROR's status on success, then
    /// file_op's end.
    fn file_change_n(
        &mut self,
        pc: usize,
        keys: Option<Vec<KeyArg>>,
        rec: NResult<Vec<Option<String>>>,
    ) -> NResult<()> {
        let (op, file, key, _, _) = self.file_stmt(pc)?;
        let file = file.to_string();
        // *START/*END as a DELETE key: no key (file_op_inner's special).
        let special = matches!(
            key.as_deref(),
            Some([Expr::Name { parts, .. }])
                if parts.first().is_some_and(|p| p == "*START" || p == "*END")
        );
        let p = self
            .pending
            .take()
            .ok_or_else(|| anyhow!("native: {op:?} {file} with no start"))?;
        let i = p.file;
        let cmt = self.file_n(i)?.cmt.clone();
        let keys: Option<Vec<String>> = match special {
            true => None,
            false => keys.map(|ks| {
                ks.into_iter()
                    .map(|k| match k {
                        KeyArg::Num(d) => d.to_string(),
                        KeyArg::Char(c) => a(&c),
                    })
                    .collect()
            }),
        };
        let made = rec.and_then(|cols| self.record_n(i, op, cols));
        let r = made.and_then(|rec| self.record_change_n(i, op, &rec, &keys, cmt.as_deref()));
        match r {
            Ok(()) => {
                if let Some(d) = &cmt {
                    d.changed();
                }
                self.m.last = super::Status {
                    found: true,
                    ..super::Status::default()
                };
                self.file_end_n(i, op, &file, cmt.as_deref(), Ok(()))
            }
            Err(err) => {
                self.file_end_n(i, op, &file, cmt.as_deref(), Err(&err))?;
                Err(err)
            }
        }
    }

    /// record_of: the record format's fields by name, with the null
    /// values under ALWNULL(*USRCTL); DELETE takes an empty record.
    fn record_n(
        &self,
        i: usize,
        op: FileOp,
        cols: Vec<Option<String>>,
    ) -> NResult<crate::rla::Record> {
        let mut rec = crate::rla::Record::new();
        if op == FileOp::Delete {
            return Ok(rec);
        }
        let prog: &super::super::program::Program = self.m.prog;
        let fi = prog
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?;
        let fref = fi
            .fref
            .as_ref()
            .ok_or_else(|| anyhow!("{} is not a database file", fi.decl.name))?;
        for (fd, text) in fref.file.fields.iter().zip(cols) {
            if let Some(t) = text {
                rec.insert(fd.name.clone(), t);
            }
        }
        if self.m.alwnull == super::AlwNull::UsrCtl {
            let nulls: Vec<String> = fref
                .file
                .fields
                .iter()
                .zip(&fi.fields)
                .filter(|(fd, p)| fd.null_capable && self.m.nullind.contains(*p))
                .map(|(fd, _)| fd.name.clone())
                .collect();
            if !nulls.is_empty() {
                rec.insert(crate::rla::NULLS.to_string(), nulls.join(","));
            }
        }
        Ok(rec)
    }

    /// interp.rs record_change: trigger programs or not (asked once),
    /// the change, and the record lock released after UPDATE and DELETE.
    fn record_change_n(
        &mut self,
        i: usize,
        op: FileOp,
        rec: &crate::rla::Record,
        keys: &Option<Vec<String>>,
        cmt: Option<&crate::cmtctl::Definition>,
    ) -> NResult<()> {
        let rt = self.m.io.rt.clone();
        let client: &tokio_postgres::Client = match cmt {
            Some(d) => &d.client,
            None => self.m.io.data.unwrap_or(self.m.io.client),
        };
        let fref = self.fref_n(i)?;
        let triggered = match self.file_n(i)?.pftrg {
            Some(t) => t,
            None => {
                let t: bool = rt
                    .block_on(client.query_one(
                        "SELECT as4tainr_svc.has_pftrg($1, $2)",
                        &[&fref.library, &fref.file.name],
                    ))
                    .map(|r| r.get(0))
                    .unwrap_or(false);
                self.file_n(i)?.pftrg = Some(t);
                t
            }
        };
        if triggered {
            // Trigger programs: the interpreter's record_change, which
            // calls them (above).
            return self.m.record_change(i, op, rec, keys);
        }
        let r = self.rla_change_n(i, op, rec, keys, client);
        if r.is_ok() && matches!(op, FileOp::Update | FileOp::Delete) {
            if let Some(f) = self.file_n(i)?.open.as_mut() {
                rt.block_on(f.release(client))?;
            }
        }
        r
    }

    /// interp.rs rla_change: WRITE, UPDATE, or DELETE (of the record a
    /// key finds, CPF5013 if none, or of the record last read).
    fn rla_change_n(
        &mut self,
        i: usize,
        op: FileOp,
        rec: &crate::rla::Record,
        keys: &Option<Vec<String>>,
        client: &tokio_postgres::Client,
    ) -> NResult<()> {
        let rt = self.m.io.rt.clone();
        let f = self
            .file_n(i)?
            .open
            .as_mut()
            .ok_or_else(|| anyhow!("file not open"))?;
        match op {
            FileOp::Write => rt.block_on(f.write(client, rec)).map(|_| ()),
            FileOp::Update => rt.block_on(f.update(client, rec)),
            _ => rt.block_on(async {
                if let Some(k) = keys {
                    let r = f.chain(client, k).await?;
                    if !r.found {
                        return Err(anyhow!("CPF5013: The record to delete was not found."));
                    }
                }
                f.delete(client).await
            }),
        }
    }
}

/// A file statement's operation, file name, key, extenders and result
/// indicators (S2.58).
type FileStmt<'s> = (
    FileOp,
    &'s str,
    &'s Option<Vec<Expr>>,
    &'s super::super::ast::Ext,
    &'s super::super::ast::ResultInd,
);

/// errors.rs opcode_name: the operation name an INFDS shows (ILE RPG
/// Reference, "File Feedback Information", *OPCODE).
fn opcode_name_n(op: FileOp) -> &'static str {
    match op {
        FileOp::Read => "READ",
        FileOp::ReadE => "READE",
        FileOp::ReadP => "READP",
        FileOp::ReadPE => "REDPE",
        FileOp::Chain => "CHAIN",
        FileOp::SetLL => "SETLL",
        FileOp::SetGT => "SETGT",
        FileOp::Write => "WRITE",
        FileOp::Update => "UPDAT",
        FileOp::Delete => "DELET",
        FileOp::Open => "OPEN",
        FileOp::Close => "CLOSE",
        FileOp::Unlock => "UNLCK",
        FileOp::Feod => "FEOD",
        FileOp::Exfmt => "EXFMT",
        FileOp::ReadC => "READC",
    }
}
