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
    /// `checked`: a call of the procedure by its name, whose arguments
    /// call_named_inner's args_for checks against the procedure's own
    /// parameters; a call through a prototype (S2.87: a procedure pointer
    /// among them) is call_proc's, which takes more arguments than the
    /// procedure has (it does not see them; %PARMS counts them) and
    /// leaves the prototype's check to args_for.
    fn call_frame(
        &mut self,
        proc: &str,
        args: Vec<CallArg>,
        checked: bool,
    ) -> NResult<Option<Value>> {
        let prog: &'a super::super::program::Program = self.m.prog;
        let pi = prog
            .procs
            .get(proc)
            .ok_or_else(|| anyhow!("native: {proc} is not a procedure of this program"))?;
        if self.m.frames.len() > 1000 {
            anyhow::bail!("procedure {}: calls are nested too deeply", pi.name);
        }
        if checked && args.len() > pi.params.len() {
            anyhow::bail!(
                "{proc}: {} parameters passed, it takes {}",
                args.len(),
                pi.params.len()
            );
        }
        if let Some(p) = pi.params.get(args.len()).filter(|_| checked) {
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
        let mut r = self.call_body(pi, values);
        // ON-EXIT (S2.90), as interp.rs call_proc: it runs however the
        // procedure ends, its indicator on when it ended abnormally; a
        // RETURN there replaces the value of a normal end; an error there
        // is the call's.
        if let Some((ind, exit)) = &pi.on_exit {
            let abnormal = r.is_err();
            let x = (|| -> NResult<Option<Value>> {
                if let Some(n) = ind {
                    let Some(super::super::program::Sym::Var(v)) = self.m.sym(n) else {
                        anyhow::bail!("native: the ON-EXIT indicator {n} has no storage");
                    };
                    rpgrt::native::store(
                        &mut self.m.bufs,
                        v.buf,
                        v.off,
                        v.dtype,
                        &Value::Ind(abnormal),
                        Fit::Strict { half_adjust: false },
                        false,
                    )?;
                }
                let flow = match dispatch(self.m, &exit.detail, 0) {
                    Some(f) => f,
                    None => self.m.run_code(&exit.detail),
                }?;
                Ok(match flow {
                    Flow::Return(v) => v,
                    _ => None,
                })
            })();
            match x {
                Ok(Some(v)) if !abnormal => r = Ok(Some(v)),
                Ok(_) => {}
                Err(e) => r = Err(e),
            }
        }
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
        // EXTPROC(procedure pointer) (S2.87): the procedure it addresses
        // now (interp.rs call_named_inner); one of another program, module
        // or service program is called there (call_bound_in, the
        // interpreter's call services: ADR 0007's external calls).
        let target = match &pr.extproc_ptr {
            Some(var) => match self.procptr_target_n(name, var, args)? {
                ProcTarget::Called(v) => return Ok(v),
                ProcTarget::Here(pname, a) => {
                    return self.call_proto_target_n(name, pr, pname, a);
                }
            },
            None => pr.extproc.clone().unwrap_or_else(|| name.to_string()),
        };
        self.call_proto_target_n(name, pr, target, args)
    }

    /// call_proto's end (S2.87: split off, so a procedure pointer's target
    /// takes it too): a procedure of this module, one the binder found, a
    /// CEE API, a C runtime function, in call_named_inner's order.
    fn call_proto_target_n(
        &mut self,
        name: &str,
        pr: &super::super::program::Proto,
        target: String,
        args: Vec<CallArg>,
    ) -> NResult<Option<Value>> {
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
            return self.call_frame(&target, args, false);
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

    fn each_array(&mut self, pc: usize, arr: rpgrt::native::ArrPlace) -> NResult<()> {
        // list_of of an array with no index: its elements in use (every
        // one, an array of fixed dimension), loaded in order; then the
        // EachInit arm's state under the same key.
        if !matches!(&at(self.code, pc)?.ins, Ins::EachInit { .. }) {
            return Err(mismatch(pc, "a FOR-EACH start"));
        }
        let mut vals = Vec::new();
        for k in 0..usize::try_from(arr.dim)? {
            let at = k
                .checked_mul(arr.stride)
                .and_then(|d| arr.off.checked_add(d))
                .ok_or_else(|| anyhow!("native: an element's place"))?;
            vals.push(rpgrt::native::load(&self.m.bufs, arr.buf, at, arr.ty)?);
        }
        let key = (self.code.as_ptr().addr(), pc, self.m.frames.len());
        self.m.each.insert(key, (vals, 0));
        Ok(())
    }

    fn each_take(&mut self, pc: usize) -> NResult<Option<Value>> {
        let Ins::EachNext { init, .. } = &at(self.code, pc)?.ins else {
            return Err(mismatch(pc, "a FOR-EACH step"));
        };
        let key = (self.code.as_ptr().addr(), *init, self.m.frames.len());
        let next = self.m.each.get_mut(&key).and_then(|(v, at)| {
            let x = v.get(*at).cloned();
            *at = at.saturating_add(1);
            x
        });
        if next.is_none() {
            self.m.each.remove(&key);
        }
        Ok(next)
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
            self.call_frame(proc, args, true)
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

    fn exec_sql_dyn(
        &mut self,
        pc: usize,
        kind: rpgrt::native::SqlDyn,
        name: &str,
        text: Option<Value>,
        lits: Vec<String>,
    ) -> NResult<rpgrt::native::SqlResult> {
        let o = self.exec_sql_dyn_n(pc, kind, name, text, lits)?;
        Ok(sql_result(o))
    }

    fn sql_open(
        &mut self,
        pc: usize,
        name: &str,
        pieces: &[&str],
        qlits: Vec<String>,
        prepared: Option<&str>,
        lits: Vec<String>,
    ) -> NResult<rpgrt::native::SqlResult> {
        let o = self.sql_cursor_n(pc, |rt| rt.sql_open_n(name, pieces, qlits, prepared, lits))?;
        Ok(sql_result(o))
    }

    fn sql_fetch(
        &mut self,
        pc: usize,
        name: &str,
        orient: &str,
        n: i64,
        outs: Vec<rpgrt::native::SqlSlot>,
        into: bool,
    ) -> NResult<rpgrt::native::SqlResult> {
        let o = self.sql_cursor_n(pc, |rt| rt.sql_fetch_n(name, orient, n, &outs, into))?;
        Ok(sql_result(o))
    }

    fn sql_positioned(
        &mut self,
        pc: usize,
        cursor: &str,
        table: &str,
        pieces: &[&str],
        qlits: Vec<String>,
    ) -> NResult<rpgrt::native::SqlResult> {
        let o = self.sql_cursor_n(pc, |rt| rt.sql_positioned_n(cursor, table, pieces, qlits))?;
        Ok(sql_result(o))
    }

    fn sql_fetch_rows(
        &mut self,
        pc: usize,
        name: &str,
        orient: &str,
        n: i64,
        count: Dec,
        dim: u32,
        host: &str,
        fields: Vec<rpgrt::native::SqlRowSlot>,
    ) -> NResult<rpgrt::native::SqlResult> {
        let o = self.sql_cursor_n(pc, |rt| {
            rt.sql_fetch_rows_n(name, orient, n, &count, dim, host, &fields)
        })?;
        Ok(sql_result(o))
    }

    fn sql_insert_rows(
        &mut self,
        pc: usize,
        target: &str,
        count: Dec,
        dim: u32,
        host: &str,
        fields: Vec<rpgrt::native::SqlRowSlot>,
    ) -> NResult<rpgrt::native::SqlResult> {
        let o = self.sql_cursor_n(pc, |rt| {
            rt.sql_insert_rows_n(target, &count, dim, host, &fields)
        })?;
        Ok(sql_result(o))
    }

    fn sql_call(
        &mut self,
        pc: usize,
        name: &str,
        exprs: Vec<String>,
        outs: Vec<Option<rpgrt::native::SqlSlot>>,
    ) -> NResult<rpgrt::native::SqlResult> {
        let o = self.sql_cursor_n(pc, |rt| rt.sql_call_n(name, &exprs, &outs))?;
        Ok(sql_result(o))
    }

    fn printer_write(
        &mut self,
        pc: usize,
        fields: Vec<rpgrt::native::OutSlot>,
        indds: Option<rpgrt::native::OutSlot>,
    ) -> NResult<()> {
        self.printer_write_n(pc, &fields, indds.as_ref())
    }

    fn workstn(
        &mut self,
        pc: usize,
        fields: Vec<rpgrt::native::OutSlot>,
        indds: Option<rpgrt::native::OutSlot>,
        key: Option<rpgrt::native::OutSlot>,
    ) -> NResult<()> {
        self.workstn_n(pc, &fields, indds.as_ref(), key.as_ref())
    }

    fn printer_op(&mut self, pc: usize) -> NResult<()> {
        self.printer_op_n(pc)
    }

    fn str_at(&mut self, p: &Value, max: Option<Dec>) -> NResult<Value> {
        // ptrs.rs str_at: the pointer's place (addr), then the bytes up to
        // the first null within max (else 16 MB) and the storage's end.
        let max = match max {
            Some(m) => Some(usize::try_from(m.to_i128()?).unwrap_or(usize::MAX)),
            None => None,
        };
        let (buf, off) = self.addr_n(p, 0)?;
        let b = self
            .m
            .bufs
            .get(buf)
            .ok_or_else(|| anyhow!("native: %STR's storage"))?;
        let limit = max.unwrap_or(16 * 1024 * 1024);
        let end = off.saturating_add(limit).min(b.len());
        let s = b.get(off..end).unwrap_or(&[]);
        let n = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        Ok(Value::Char(s.get(..n).unwrap_or(&[]).to_vec()))
    }

    fn paddr(&mut self, name: &str) -> NResult<Value> {
        self.paddr_n(name)
    }

    fn sorta(&mut self, pc: usize, arr: rpgrt::native::ArrPlace, descend: bool) -> NResult<()> {
        if !matches!(&at(self.code, pc)?.ins, Ins::Stmt(_)) {
            return Err(mismatch(pc, "a SORTA"));
        }
        self.sorta_n(&arr, descend)
    }

    fn movea(
        &mut self,
        pc: usize,
        from: rpgrt::native::MoveaSide,
        to: rpgrt::native::MoveaSide,
        pad: bool,
    ) -> NResult<()> {
        if !matches!(&at(self.code, pc)?.ins, Ins::Stmt(_)) {
            return Err(mismatch(pc, "a MOVEA"));
        }
        self.movea_n(&from, &to, pad)
    }

    fn array_op(&mut self, pc: usize, op: rpgrt::native::ArrOpIn) -> NResult<()> {
        if !matches!(&at(self.code, pc)?.ins, Ins::Stmt(_)) {
            return Err(mismatch(pc, "an array operation"));
        }
        self.array_op_n(&op)
    }

    fn bit_op(&mut self, pc: usize, op: rpgrt::native::BitOpIn) -> NResult<()> {
        if !matches!(&at(self.code, pc)?.ins, Ins::Stmt(_)) {
            return Err(mismatch(pc, "a bit operation"));
        }
        self.bit_op_n(&op)
    }

    fn str_op(
        &mut self,
        pc: usize,
        op: rpgrt::native::StrOpIn,
        result: rpgrt::native::StrOut,
    ) -> NResult<()> {
        if !matches!(&at(self.code, pc)?.ins, Ins::Stmt(_)) {
            return Err(mismatch(pc, "a string operation"));
        }
        self.str_op_n(op, &result)
    }

    fn data_area(
        &mut self,
        pc: usize,
        input: bool,
        lock: bool,
        fields: Vec<rpgrt::native::DtaSlot>,
    ) -> NResult<()> {
        if !matches!(&at(self.code, pc)?.ins, Ins::Stmt(_)) {
            return Err(mismatch(pc, "an IN or OUT"));
        }
        for f in &fields {
            self.data_area_n(input, lock, f)?;
        }
        Ok(())
    }

    fn except(
        &mut self,
        pc: usize,
        name: Option<&str>,
        fields: Vec<rpgrt::native::OutSlot>,
    ) -> NResult<()> {
        if !matches!(&at(self.code, pc)?.ins, Ins::Stmt(_)) {
            return Err(mismatch(pc, "an EXCEPT"));
        }
        self.except_n(name, &fields)
    }

    fn sql_close(&mut self, pc: usize, name: &str) -> NResult<rpgrt::native::SqlResult> {
        let o = self.sql_cursor_n(pc, |rt| {
            Ok(match rt.m.sql.cursors.remove(name) {
                Some(_) => sql_ok(0),
                None => sql_err(-501, format!("SQL0501: Cursor {name} is not open.")),
            })
        })?;
        Ok(sql_result(o))
    }

    fn sql_diagnostics(
        &mut self,
        pc: usize,
        condition: bool,
        items: &[&str],
    ) -> NResult<(Vec<Value>, rpgrt::native::SqlResult)> {
        let (vals, o) = self.sql_diagnostics_n(pc, condition, items)?;
        Ok((vals, sql_result(o)))
    }

    fn exec_sql_hosts(
        &mut self,
        pc: usize,
        pieces: &[&str],
        lits: Vec<String>,
        outs: Vec<rpgrt::native::SqlSlot>,
        into: bool,
    ) -> NResult<rpgrt::native::SqlResult> {
        let o = self.exec_sql_hosts_n(pc, pieces, lits, outs, into)?;
        Ok(rpgrt::native::SqlResult {
            code: o.code,
            state: o.state,
            text: o.text,
            rows: o.rows,
            errd5: o.errd5,
        })
    }

    fn exec_sql(&mut self, pc: usize) -> NResult<rpgrt::native::SqlResult> {
        let o = self.exec_sql_n(pc)?;
        Ok(rpgrt::native::SqlResult {
            code: o.code,
            state: o.state,
            text: o.text,
            rows: o.rows,
            errd5: o.errd5,
        })
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
            FileBit::Error => st.error,
        })
    }

    fn file_code(&mut self, file: &str) -> NResult<u32> {
        // bifs.rs "STATUS" with a file: file_status of it, its code.
        let i = self
            .file_idx_n(file)
            .ok_or_else(|| anyhow!("{file} is not a file"))?;
        Ok(self
            .m
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?
            .status
            .code)
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
                // + or - of a float expression (S2.108's float_expr steps
                // into it; the review of 2026-11-19).
                Expr::Unary(_, x) if *step == 0 => x,
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

// ---- embedded SQL, step 1 (S2.69, ADR 0007) -----------------------------
//
// Written again from the interpreter's exec_sql.rs, in its order: exec_sql
// (the outcome, an SQL error as one, `last`), sql_statement's start (the
// wait for other jobs' locks, STACK_INFO) and its arms for the statements
// lowering lets through (no host variable, no cursor, no INTO, no CALL, no
// dynamic SQL, no GET DIAGNOSTICS), sql_ddl, sql_change, run_sql,
// sql_overrides and session, with the text helpers words, find_word and
// sql_error. native_tripwire.rs watches them. sql::Session, ddl.rs and
// objlock.rs are the runtime both engines call. Generated code stores the
// outcome into the SQLCA's fields, as exec_sql assigns them.
type SqlOut = super::exec_sql::Outcome;

fn sql_ok(rows: i64) -> SqlOut {
    SqlOut {
        code: 0,
        state: "00000".into(),
        rows,
        text: String::new(),
        errd5: 0,
    }
}

fn sql_not_found() -> SqlOut {
    SqlOut {
        code: 100,
        state: "02000".into(),
        rows: 0,
        text: String::new(),
        errd5: 0,
    }
}

fn sql_err(code: i64, text: String) -> SqlOut {
    SqlOut {
        code,
        state: super::exec_sql::sqlstate(code).into(),
        rows: 0,
        text,
        errd5: 0,
    }
}

/// exec_sql.rs sql_error: an error from the SQL layer ("SQL0204: ...") as
/// its SQLCODE and text.
fn sql_error_n(e: &anyhow::Error) -> Option<SqlOut> {
    let m = format!("{e:#}");
    if std::env::var_os("AS4TAINR_TRACE_ERRORS").is_some() {
        let _ = writeln!(std::io::stderr(), "[SQL error] {m}");
    }
    let digits: String = m.strip_prefix("SQL")?.chars().take(4).collect();
    if digits.len() != 4 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n: i64 = digits.parse().ok()?;
    Some(sql_err(n.checked_neg()?, m))
}

/// exec_sql.rs words: the statement's words, upper case.
fn sql_words(sql: &str) -> Vec<String> {
    sql.split_whitespace()
        .map(|w| w.to_ascii_uppercase())
        .collect()
}

/// exec_sql.rs find_word: a keyword at nesting depth 0, outside literals,
/// as its byte offset.
fn sql_find_word(sql: &str, word: &str) -> Option<usize> {
    let up = word.to_ascii_uppercase();
    let is_word = |c: char| c.is_ascii_alphanumeric() || "_#@$".contains(c);
    let (mut depth, mut quote) = (0i32, None::<char>);
    let mut prev: Option<char> = None;
    for (at, c) in sql.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None => match c {
                '\'' | '"' => quote = Some(c),
                '(' => depth = depth.saturating_add(1),
                ')' => depth = depth.saturating_sub(1),
                _ if depth == 0
                    && is_word(c)
                    && !prev.is_some_and(is_word)
                    && sql
                        .get(at..)
                        .and_then(|r| r.get(..up.len()))
                        .is_some_and(|w| w.eq_ignore_ascii_case(&up))
                    && sql
                        .get(at..)
                        .and_then(|r| r.get(up.len()..))
                        .is_some_and(|r| r.chars().next().is_none_or(|n| !is_word(n))) =>
                {
                    return Some(at);
                }
                _ => {}
            },
        }
        prev = Some(c);
    }
    None
}

impl Rt<'_, '_> {
    /// exec_sql.rs exec_sql up to the SQLCA: the statement's outcome (an
    /// SQL error as one), kept as the last for GET DIAGNOSTICS.
    fn exec_sql_n(&mut self, pc: usize) -> NResult<SqlOut> {
        let text = match &at(self.code, pc)?.ins {
            Ins::Stmt(super::super::ast::Stmt::Sql(t)) => t.clone(),
            _ => return Err(mismatch(pc, "an EXEC SQL")),
        };
        let out = match self.sql_statement_n(text.trim()) {
            Ok(o) => o,
            Err(e) => match sql_error_n(&e) {
                Some(o) => o,
                None => return Err(e),
            },
        };
        self.m.sql.last = Some(out.clone());
        Ok(out)
    }

    /// exec_sql.rs exec_sql for a statement with host variables (S2.70):
    /// sql_statement's start, then bind_host's text (the pieces with the
    /// input literals between them), then single_row (SELECT or VALUES
    /// INTO, SET :host) or sql_change; an SQL error as the outcome, kept as
    /// the last.
    fn exec_sql_hosts_n(
        &mut self,
        pc: usize,
        pieces: &[&str],
        lits: Vec<String>,
        outs: Vec<rpgrt::native::SqlSlot>,
        into: bool,
    ) -> NResult<SqlOut> {
        let text = match &at(self.code, pc)?.ins {
            Ins::Stmt(super::super::ast::Stmt::Sql(t)) => t.clone(),
            _ => return Err(mismatch(pc, "an EXEC SQL")),
        };
        let r = (|| -> NResult<SqlOut> {
            if let Some(o) = self.sql_prologue_n(text.trim())? {
                return Ok(o);
            }
            let mut bound = String::new();
            let mut lit = lits.into_iter();
            for (k, p) in pieces.iter().enumerate() {
                if k > 0 {
                    bound.push_str(
                        &lit.next()
                            .ok_or_else(|| anyhow!("native: a host literal"))?,
                    );
                }
                bound.push_str(p);
            }
            if into {
                self.single_row_n(&bound, &outs)
            } else {
                self.sql_change_n(&bound)
            }
        })();
        let out = match r {
            Ok(o) => o,
            Err(e) => match sql_error_n(&e) {
                Some(o) => o,
                None => return Err(e),
            },
        };
        self.m.sql.last = Some(out.clone());
        Ok(out)
    }

    /// exec_sql.rs exec_sql for PREPARE, EXECUTE IMMEDIATE and EXECUTE
    /// (S2.71): sql_statement's start, then its arm: PREPARE keeps the
    /// text (statement_text: the value's characters, trimmed) under the
    /// name; EXECUTE IMMEDIATE runs the text, EXECUTE the prepared one with
    /// the USING literals for its markers (fill_markers; SQL0518 when it
    /// was not prepared), as DDL (sql_ddl) or else through sql_change. An
    /// SQL error as the outcome, kept as the last.
    fn exec_sql_dyn_n(
        &mut self,
        pc: usize,
        kind: rpgrt::native::SqlDyn,
        name: &str,
        text: Option<Value>,
        lits: Vec<String>,
    ) -> NResult<SqlOut> {
        use rpgrt::native::SqlDyn;
        let sql = match &at(self.code, pc)?.ins {
            Ins::Stmt(super::super::ast::Stmt::Sql(t)) => t.clone(),
            _ => return Err(mismatch(pc, "an EXEC SQL")),
        };
        let statement_text = |v: Option<Value>| -> NResult<String> {
            let v = v.ok_or_else(|| anyhow!("native: a statement's text"))?;
            Ok(a(&v.chars()?).trim().to_string())
        };
        let r = (|| -> NResult<SqlOut> {
            if let Some(o) = self.sql_prologue_n(sql.trim())? {
                return Ok(o);
            }
            let run = match kind {
                SqlDyn::Prepare => {
                    let t = statement_text(text)?;
                    self.m.sql.prepared.insert(name.to_string(), t);
                    return Ok(sql_ok(0));
                }
                SqlDyn::Immediate => statement_text(text)?,
                SqlDyn::Execute => {
                    let t =
                        self.m.sql.prepared.get(name).cloned().ok_or_else(|| {
                            anyhow!("SQL0518: Statement {name} was not prepared.")
                        })?;
                    super::exec_sql::fill_markers(&t, &lits)?
                }
            };
            if let Some(o) = self.sql_ddl_n(&run)? {
                return Ok(o);
            }
            self.sql_change_n(&run)
        })();
        let out = match r {
            Ok(o) => o,
            Err(e) => match sql_error_n(&e) {
                Some(o) => o,
                None => return Err(e),
            },
        };
        self.m.sql.last = Some(out.clone());
        Ok(out)
    }

    /// exec_sql.rs get_diagnostics (S2.71): about the last statement (none:
    /// SQLCODE 0), ROW_COUNT; with CONDITION, MESSAGE_TEXT (the text after
    /// its message ID), MESSAGE_LENGTH, RETURNED_SQLSTATE,
    /// DB2_RETURNED_SQLCODE, MESSAGE_ID and DB2_MESSAGE_ID; another item
    /// ends with SQLCODE -104. An SQL error as the outcome; the last is
    /// not changed.
    fn sql_diagnostics_n(
        &mut self,
        pc: usize,
        condition: bool,
        items: &[&str],
    ) -> NResult<(Vec<Value>, SqlOut)> {
        if !matches!(
            &at(self.code, pc)?.ins,
            Ins::Stmt(super::super::ast::Stmt::Sql(_))
        ) {
            return Err(mismatch(pc, "an EXEC SQL"));
        }
        let last = self.m.sql.last.clone().unwrap_or_else(|| sql_ok(0));
        let id = last
            .text
            .split(':')
            .next()
            .filter(|p| p.len() == 7 && p.starts_with("SQL"))
            .unwrap_or("")
            .to_string();
        let text = if id.is_empty() {
            last.text.clone()
        } else {
            last.text
                .get(id.len().saturating_add(1)..)
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let mut vals = Vec::new();
        for item in items {
            let v = match (condition, item.to_ascii_uppercase().as_str()) {
                (false, "ROW_COUNT") => Value::Num(Dec::from_i64(last.rows)),
                (true, "MESSAGE_TEXT") => Value::str(&text),
                (true, "MESSAGE_LENGTH") => {
                    Value::Num(Dec::from_i64(i64::try_from(text.chars().count())?))
                }
                (true, "RETURNED_SQLSTATE") => Value::str(&last.state),
                (true, "DB2_RETURNED_SQLCODE") => Value::Num(Dec::from_i64(last.code)),
                (true, "MESSAGE_ID" | "DB2_MESSAGE_ID") => Value::str(&id),
                (_, other) => {
                    return Ok((
                        vals,
                        sql_err(
                            -104,
                            format!("SQL0104: GET DIAGNOSTICS item {other} is not supported."),
                        ),
                    ))
                }
            };
            vals.push(v);
        }
        Ok((vals, sql_ok(0)))
    }

    /// exec_sql.rs exec_sql for OPEN, FETCH and CLOSE (S2.72):
    /// sql_statement's start, then `run`; an SQL error as the outcome,
    /// kept as the last.
    fn sql_cursor_n(
        &mut self,
        pc: usize,
        run: impl FnOnce(&mut Self) -> NResult<SqlOut>,
    ) -> NResult<SqlOut> {
        let sql = match &at(self.code, pc)?.ins {
            Ins::Stmt(super::super::ast::Stmt::Sql(t)) => t.clone(),
            _ => return Err(mismatch(pc, "an EXEC SQL")),
        };
        let r = match self.sql_prologue_n(sql.trim()) {
            Ok(Some(o)) => Ok(o),
            Ok(None) => run(self),
            Err(e) => Err(e),
        };
        let out = match r {
            Ok(o) => o,
            Err(e) => match sql_error_n(&e) {
                Some(o) => o,
                None => return Err(e),
            },
        };
        self.m.sql.last = Some(out.clone());
        Ok(out)
    }

    /// exec_sql.rs sql_open: -502 when the cursor is open; the declared
    /// query with its host literals, or the prepared statement (SQL0518
    /// when it was not prepared) with the USING literals for its markers;
    /// without its update or read-only clause; over one table, each row's
    /// record number too (WHERE CURRENT OF), else the query as it is. The
    /// rows are read now.
    fn sql_open_n(
        &mut self,
        name: &str,
        pieces: &[&str],
        qlits: Vec<String>,
        prepared: Option<&str>,
        lits: Vec<String>,
    ) -> NResult<SqlOut> {
        if self.m.sql.cursors.contains_key(name) {
            return Ok(sql_err(
                -502,
                format!("SQL0502: Cursor {name} is already open."),
            ));
        }
        let query = match prepared {
            None => {
                let mut bound = String::new();
                let mut lit = qlits.into_iter();
                for (k, p) in pieces.iter().enumerate() {
                    if k > 0 {
                        bound.push_str(
                            &lit.next()
                                .ok_or_else(|| anyhow!("native: a host literal"))?,
                        );
                    }
                    bound.push_str(p);
                }
                bound
            }
            Some(s) => {
                let text = self
                    .m
                    .sql
                    .prepared
                    .get(s)
                    .cloned()
                    .ok_or_else(|| anyhow!("SQL0518: Statement {s} was not prepared."))?;
                super::exec_sql::fill_markers(&text, &lits)?
            }
        };
        let query = sql_strip_cursor_clauses(&query);
        let rrn_query = sql_with_rrn(&query);
        let (rows, rrns) = match rrn_query.as_deref().map(|q| self.run_sql_n(q)) {
            Some(Ok(crate::sql::Exec::Rows { mut rows, .. })) => {
                let rrns = rows
                    .iter_mut()
                    .map(|r| {
                        r.pop()
                            .flatten()
                            .and_then(|v| v.trim().parse::<i64>().ok())
                            .unwrap_or(0)
                    })
                    .collect();
                (rows, Some(rrns))
            }
            _ => match self.run_sql_n(&query)? {
                crate::sql::Exec::Rows { rows, .. } => (rows, None),
                crate::sql::Exec::Count(..) => {
                    return Err(anyhow!("SQL0104: cursor {name} is not a query"))
                }
            },
        };
        self.m.sql.cursors.insert(
            name.to_string(),
            super::exec_sql::OpenCursor {
                rows,
                pos: -1,
                rrns,
            },
        );
        Ok(sql_ok(0))
    }

    /// exec_sql.rs sql_call (S2.75): the procedure called through sqlpl
    /// with the job's library list, user and current library on the
    /// program's SQL session (a failure as its condition's SQLCODE, and
    /// for -438 its SQLSTATE); then each OUT and INOUT parameter's value
    /// into its host variable (assign_hosts, an error stops it).
    fn sql_call_n(
        &mut self,
        name: &str,
        exprs: &[String],
        outs: &[Option<rpgrt::native::SqlSlot>],
    ) -> NResult<SqlOut> {
        let curlib = self.m.io.cl.as_ref().and_then(|c| c.curlib.clone());
        let client = self.m.io.client;
        let url = self.m.io.database_url.clone();
        let libl = self.m.io.libl.clone();
        let user = self.m.io.user.clone();
        let rt = self.m.io.rt.clone();
        let session = self.session_n()?;
        let env = crate::sqlpl::Env {
            session,
            catalog: client,
            database_url: &url,
            libl: &libl,
            user: &user,
            curlib,
        };
        let r = rt.block_on(crate::sqlpl::call_exprs(&env, name, exprs));
        let (routine, out) = match r {
            Ok(x) => x,
            Err(e) => {
                let (code, state, text) = crate::sqlpl::condition_of(&e);
                let mut o = sql_err(code, text);
                if code == -438 {
                    o.state = state;
                }
                return Ok(o);
            }
        };
        for ((p, t), v) in routine.parms.iter().zip(outs).zip(&out.values) {
            if p.mode == "IN" {
                continue;
            }
            if let Some(slot) = t {
                if let Some(o) =
                    self.assign_hosts_n(std::slice::from_ref(slot), std::slice::from_ref(v))?
                {
                    return Ok(o);
                }
            }
        }
        Ok(sql_ok(0))
    }

    /// exec_sql.rs fetch_rows (S2.74): -221 when the count is more than
    /// the array's elements, -501 when the cursor is not open; the first
    /// row by the orientation (kept between 0 and the row count); none
    /// (or a count of 0): +100, the cursor after the last row; else up to
    /// `count` rows into the elements in order, each subfield with the
    /// strict fit (an error stops it), the cursor on the last one read,
    /// SQLERRD(5) 100 when that is the last row.
    #[allow(clippy::too_many_arguments)] // fetch_rows' parts, as lowered
    fn sql_fetch_rows_n(
        &mut self,
        name: &str,
        orient: &str,
        n: i64,
        count: &Dec,
        dim: u32,
        host: &str,
        fields: &[rpgrt::native::SqlRowSlot],
    ) -> NResult<SqlOut> {
        let want = sql_row_count(count)?;
        if want > usize::try_from(dim.max(1))? {
            return Ok(sql_err(
                -221,
                format!("SQL0221: {want} rows are more than the {dim} elements of {host}."),
            ));
        }
        let Some(c) = self.m.sql.cursors.get_mut(name) else {
            return Ok(sql_err(
                -501,
                format!("SQL0501: Cursor {name} is not open."),
            ));
        };
        let len = isize::try_from(c.rows.len())?;
        let n = isize::try_from(n).unwrap_or(if n < 0 { isize::MIN } else { isize::MAX });
        let start = match orient {
            "PRIOR" => c.pos.saturating_sub(1),
            "FIRST" => 0,
            "LAST" => len.saturating_sub(1),
            "CURRENT" => c.pos,
            "RELATIVE" => c.pos.saturating_add(n),
            "ABSOLUTE" => n.saturating_sub(1),
            _ => c.pos.saturating_add(1),
        }
        .clamp(0, len);
        let end = start
            .saturating_add(isize::try_from(want).unwrap_or(isize::MAX))
            .min(len);
        if want == 0 || start >= len {
            c.pos = len;
            return Ok(sql_not_found());
        }
        let rows: Vec<Vec<Option<String>>> = c
            .rows
            .get(usize::try_from(start)?..usize::try_from(end)?)
            .ok_or_else(|| anyhow!("native: the cursor's rows"))?
            .to_vec();
        c.pos = end.saturating_sub(1);
        let last = end == len;
        for (i, row) in rows.iter().enumerate() {
            let outs: Vec<rpgrt::native::SqlSlot> = fields
                .iter()
                .map(|f| -> NResult<rpgrt::native::SqlSlot> {
                    Ok(rpgrt::native::SqlSlot {
                        buf: f.buf,
                        off: i
                            .checked_mul(f.stride)
                            .and_then(|d| f.off.checked_add(d))
                            .ok_or_else(|| anyhow!("native: an element's place"))?,
                        ty: f.ty,
                        numeric: f.numeric,
                        ind: None,
                    })
                })
                .collect::<NResult<_>>()?;
            if let Some(err) = self.assign_hosts_n(&outs, row)? {
                if err.code < 0 {
                    return Ok(err);
                }
            }
        }
        let mut out = sql_ok(i64::try_from(rows.len())?);
        if last {
            out.errd5 = 100;
        }
        Ok(out)
    }

    /// exec_sql.rs sql_insert_rows (S2.74): a count of 0 inserts nothing
    /// (SQLCODE 0); more than the array's elements: -221; else one row per
    /// element up to the count, each subfield's value as a literal (a
    /// fixed character field as CAST(... AS char(n))), after `target`,
    /// through sql_change.
    fn sql_insert_rows_n(
        &mut self,
        target: &str,
        count: &Dec,
        dim: u32,
        host: &str,
        fields: &[rpgrt::native::SqlRowSlot],
    ) -> NResult<SqlOut> {
        let want = sql_row_count(count)?;
        if want == 0 {
            return Ok(sql_ok(0));
        }
        if want > usize::try_from(dim.max(1))? {
            return Ok(sql_err(
                -221,
                format!("SQL0221: {want} rows are more than the {dim} elements of {host}."),
            ));
        }
        let mut rows = Vec::new();
        for i in 0..want {
            let mut parts = Vec::new();
            for f in fields {
                let off = i
                    .checked_mul(f.stride)
                    .and_then(|d| f.off.checked_add(d))
                    .ok_or_else(|| anyhow!("native: an element's place"))?;
                let v = rpgrt::native::load(&self.m.bufs, f.buf, off, f.ty)?;
                let lit = rpgrt::native::sql_literal(&v, None)?;
                match (f.ty, &v) {
                    (super::data::DType::Char(len), Value::Char(_)) => {
                        parts.push(format!("CAST({lit} AS char({len}))"))
                    }
                    _ => parts.push(lit),
                }
            }
            rows.push(format!("({})", parts.join(", ")));
        }
        let text = format!("{target} VALUES {}", rows.join(", "));
        self.sql_change_n(&text)
    }

    /// exec_sql.rs positioned_change (S2.73): -501 when the cursor is
    /// not open, -510 when it is not over one table, -508 when it is not on
    /// a row; else the text before WHERE (its host literals bound) with
    /// `WHERE RRN(table) = n`, the row's record number, through
    /// sql_change.
    fn sql_positioned_n(
        &mut self,
        cursor: &str,
        table: &str,
        pieces: &[&str],
        qlits: Vec<String>,
    ) -> NResult<SqlOut> {
        let Some(c) = self.m.sql.cursors.get(cursor) else {
            return Ok(sql_err(
                -501,
                format!("SQL0501: Cursor {cursor} is not open."),
            ));
        };
        let Some(rrns) = &c.rrns else {
            return Ok(sql_err(
                -510,
                format!("SQL0510: Cursor {cursor} cannot be used to change the table."),
            ));
        };
        let Some(rrn) = usize::try_from(c.pos)
            .ok()
            .and_then(|p| rrns.get(p))
            .copied()
        else {
            return Ok(sql_err(
                -508,
                format!("SQL0508: Cursor {cursor} is not on a row."),
            ));
        };
        let mut head = String::new();
        let mut lit = qlits.into_iter();
        for (k, p) in pieces.iter().enumerate() {
            if k > 0 {
                head.push_str(
                    &lit.next()
                        .ok_or_else(|| anyhow!("native: a host literal"))?,
                );
            }
            head.push_str(p);
        }
        let short = table.rsplit(['.', '/']).next().unwrap_or(table);
        let text = format!("{head} WHERE RRN({short}) = {rrn}");
        self.sql_change_n(&text)
    }

    /// exec_sql.rs sql_fetch for one row: -501 when the cursor is not
    /// open; the new position by the orientation, kept between before
    /// the first (-1) and after the last; BEFORE and AFTER end there; off
    /// the rows +100; with no INTO SQLCODE 0 and 1 row; else the row into
    /// the host variables (assign_hosts).
    fn sql_fetch_n(
        &mut self,
        name: &str,
        orient: &str,
        n: i64,
        outs: &[rpgrt::native::SqlSlot],
        into: bool,
    ) -> NResult<SqlOut> {
        let Some(c) = self.m.sql.cursors.get_mut(name) else {
            return Ok(sql_err(
                -501,
                format!("SQL0501: Cursor {name} is not open."),
            ));
        };
        let len = isize::try_from(c.rows.len())?;
        let n = isize::try_from(n).unwrap_or(if n < 0 { isize::MIN } else { isize::MAX });
        c.pos = match orient {
            "NEXT" => c.pos.saturating_add(1),
            "PRIOR" => c.pos.saturating_sub(1),
            "FIRST" => 0,
            "LAST" => len.saturating_sub(1),
            "BEFORE" => -1,
            "AFTER" => len,
            "CURRENT" => c.pos,
            "RELATIVE" => c.pos.saturating_add(n),
            _ => n.saturating_sub(1),
        }
        .clamp(-1, len);
        if orient == "BEFORE" || orient == "AFTER" {
            return Ok(sql_ok(0));
        }
        let row = match usize::try_from(c.pos).ok().and_then(|p| c.rows.get(p)) {
            Some(r) => r.clone(),
            None => return Ok(sql_not_found()),
        };
        if !into {
            return Ok(sql_ok(1));
        }
        Ok(self.assign_hosts_n(outs, &row)?.unwrap_or(sql_ok(1)))
    }

    /// exec_sql.rs single_row: exactly one row into the host variables
    /// (none: +100; more: the first row's values, then -811).
    fn single_row_n(&mut self, query: &str, outs: &[rpgrt::native::SqlSlot]) -> NResult<SqlOut> {
        let crate::sql::Exec::Rows { rows, .. } = self.run_sql_n(query)? else {
            return Err(anyhow!("SQL0104: INTO needs a query"));
        };
        match rows.as_slice() {
            [] => Ok(sql_not_found()),
            [row] => Ok(self.assign_hosts_n(outs, row)?.unwrap_or(sql_ok(1))),
            [row, ..] => {
                self.assign_hosts_n(outs, row)?;
                Ok(sql_err(
                    -811,
                    "SQL0811: The result of SELECT INTO has more than one row.".into(),
                ))
            }
        }
    }

    /// exec_sql.rs assign_hosts: a row's values into the host variables in
    /// order (a null: the indicator -1, or -305 with none; a value: the
    /// indicator 0, a number parsed exactly (-302 if it is not one),
    /// characters, stored with the strict fit (-302 if it does not fit)).
    fn assign_hosts_n(
        &mut self,
        outs: &[rpgrt::native::SqlSlot],
        row: &[Option<String>],
    ) -> NResult<Option<SqlOut>> {
        use rpgrt::rpg::data::Fit;
        let mut values = row.iter();
        for o in outs {
            let Some(v) = values.next() else {
                return Ok(None);
            };
            let set_ind = |rt: &mut Self, n: i64| -> NResult<()> {
                if let Some((b, off, t)) = o.ind {
                    rpgrt::native::store(
                        &mut rt.m.bufs,
                        b,
                        off,
                        t,
                        &Value::Num(Dec::from_i64(n)),
                        Fit::Truncate { half_adjust: false },
                        false,
                    )?;
                }
                Ok(())
            };
            match v {
                None => {
                    if o.ind.is_some() {
                        set_ind(self, -1)?;
                    } else {
                        return Ok(Some(sql_err(
                            -305,
                            "SQL0305: A null value has no indicator variable.".into(),
                        )));
                    }
                }
                Some(text) => {
                    set_ind(self, 0)?;
                    let val = if o.numeric {
                        match Dec::parse(text.trim()) {
                            Ok(d) => Value::Num(d),
                            Err(_) => {
                                return Ok(Some(sql_err(
                                    -302,
                                    format!(
                                        "SQL0302: '{text}' does not fit a numeric host variable."
                                    ),
                                )))
                            }
                        }
                    } else {
                        Value::str(text)
                    };
                    if let Err(e) = rpgrt::native::store(
                        &mut self.m.bufs,
                        o.buf,
                        o.off,
                        o.ty,
                        &val,
                        Fit::Strict { half_adjust: false },
                        false,
                    ) {
                        return Ok(Some(sql_err(-302, format!("SQL0302: {e}"))));
                    }
                }
            }
        }
        Ok(None)
    }

    /// exec_sql.rs sql_statement's start: the wait for another job's
    /// locks on the statement's tables (an SQL error as the outcome), and
    /// STACK_INFO's call stack.
    fn sql_prologue_n(&mut self, sql: &str) -> NResult<Option<SqlOut>> {
        {
            let libl = self.m.io.libl.clone();
            let wait = crate::runtime::default_record_wait();
            let owner = self.m.io.job_key.clone();
            if let Err(e) = self.m.block(crate::objlock::sql_tables_free(
                self.m.io.client,
                sql,
                &libl,
                &owner,
                wait,
            )) {
                if let Some(o) = sql_error_n(&e) {
                    return Ok(Some(o));
                }
                return Err(e);
            }
        }
        if sql.to_ascii_uppercase().contains("STACK_INFO") {
            let mut entries: Vec<serde_json::Value> = Vec::new();
            for e in crate::pgmmsg::stack_listed(&self.m.io.job_key) {
                if e.pep {
                    entries.push(serde_json::json!({
                        "pgm": e.pgm, "lib": e.lib, "module": e.module,
                        "proc": format!("_QRNP_PEP_{}", e.pgm), "actgrp": e.actgrp, "line": 0,
                    }));
                }
                entries.push(serde_json::json!({
                    "pgm": e.pgm, "lib": e.lib, "module": e.module,
                    "proc": e.proc, "actgrp": e.actgrp, "line": e.line,
                }));
            }
            let json = serde_json::Value::Array(entries).to_string();
            let rt = self.m.io.rt.clone();
            let session = self.session_n()?;
            rt.block_on(session.set_stack(&json))?;
        }
        Ok(None)
    }

    /// exec_sql.rs sql_statement, for what lowering lets through (step 1).
    fn sql_statement_n(&mut self, sql: &str) -> NResult<SqlOut> {
        if let Some(o) = self.sql_prologue_n(sql)? {
            return Ok(o);
        }
        let w = sql_words(sql);
        let first = w.first().map(String::as_str).unwrap_or("");
        let second = w.get(1).map(String::as_str).unwrap_or("");
        match first {
            "SET" if second == "OPTION" => return Ok(sql_ok(0)),
            "INCLUDE" | "WHENEVER" | "BEGIN" | "END" => return Ok(sql_ok(0)),
            "DECLARE"
                if w.get(2).map(String::as_str) == Some("CURSOR")
                    || w.iter().any(|x| x == "CURSOR") =>
            {
                return Ok(sql_ok(0))
            }
            "COMMIT" | "ROLLBACK" => {
                let d = match self.m.sql.session.as_ref().and_then(|s| s.commitment()) {
                    Some(d) => Some(d.clone()),
                    None => crate::cmtctl::find(&self.m.io.job_key, &self.m.io.actgrp),
                };
                if let Some(d) = d {
                    let r = if first == "COMMIT" {
                        self.m.block(d.commit())
                    } else {
                        self.m.block(d.rollback())
                    };
                    if let Err(e) = r {
                        return Ok(sql_err(-901, format!("SQL0901: {e}")));
                    }
                }
                return Ok(sql_ok(0));
            }
            "CREATE" | "DROP" | "LABEL" | "COMMENT" | "ALTER" | "RENAME" => {
                if let Some(o) = self.sql_ddl_n(sql)? {
                    return Ok(o);
                }
            }
            "GRANT" | "REVOKE" | "LOCK" | "MERGE" => {
                return Ok(sql_err(
                    -84,
                    format!("SQL0084: {first} statements are not supported in embedded SQL yet."),
                ))
            }
            // Lowering lets no other kind of statement through that the
            // interpreter treats on its own (OPEN, FETCH, CLOSE, PREPARE,
            // EXECUTE, SET RESULT, SET :host, INTO, CALL, INSERT ... ROWS,
            // GET).
            "OPEN" | "FETCH" | "CLOSE" | "PREPARE" | "EXECUTE" | "CALL" | "GET" => {
                return Err(anyhow!("native: {first} is not lowered"))
            }
            _ => {}
        }
        if sql_find_word(sql, "CURRENT").is_some()
            && w.windows(2)
                .any(|p| matches!(p, [a, b] if a == "CURRENT" && b == "OF"))
        {
            return Err(anyhow!("native: a positioned change is not lowered"));
        }
        // bind_host: no host variable (lowering), so the text as it is.
        self.sql_change_n(sql)
    }

    /// exec_sql.rs sql_ddl: CREATE, DROP, LABEL, COMMENT, ALTER, RENAME as
    /// RUNSQLSTM runs them; None when ddl.rs does not take it.
    fn sql_ddl_n(&mut self, sql: &str) -> NResult<Option<SqlOut>> {
        let first = sql
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        if !matches!(
            first.as_str(),
            "CREATE" | "DROP" | "LABEL" | "COMMENT" | "ALTER" | "RENAME"
        ) {
            return Ok(None);
        }
        let curlib = self.m.io.cl.as_ref().and_then(|c| c.curlib.clone());
        let rt = self.m.io.rt.clone();
        match rt.block_on(crate::ddl::try_run(
            &self.m.io.database_url,
            &self.m.io.user,
            &self.m.io.libl,
            curlib,
            &mut None,
            sql,
        )) {
            Ok(Some(_)) => Ok(Some(sql_ok(0))),
            Ok(None) => Ok(None),
            Err(e) => match sql_error_n(&e) {
                Some(o) => Ok(Some(o)),
                None => Err(e),
            },
        }
    }

    /// exec_sql.rs sql_change: the statement run; a change of no rows is
    /// SQLCODE +100.
    fn sql_change_n(&mut self, text: &str) -> NResult<SqlOut> {
        Ok(match self.run_sql_n(text)? {
            crate::sql::Exec::Count(_, 0) => sql_not_found(),
            crate::sql::Exec::Count(_, n) => sql_ok(i64::try_from(n)?),
            crate::sql::Exec::Rows { rows, .. } => sql_ok(i64::try_from(rows.len())?),
        })
    }

    /// exec_sql.rs run_sql.
    fn run_sql_n(&mut self, sql: &str) -> NResult<crate::sql::Exec> {
        self.sql_overrides_n(sql)?;
        let client = self.m.io.client;
        let rt = self.m.io.rt.clone();
        let s = self.session_n()?;
        rt.block_on(s.exec(client, sql))
    }

    /// exec_sql.rs sql_overrides: database file overrides for the names the
    /// statement uses, as temporary views of those names.
    fn sql_overrides_n(&mut self, sql: &str) -> NResult<()> {
        let mut all: Vec<crate::ovr::Override> = self
            .m
            .io
            .cl
            .as_ref()
            .map(|c| c.overrides.clone())
            .unwrap_or_default();
        crate::ovr::merge_scoped(&self.m.io.job_key, &mut all);
        let ovr: Vec<crate::ovr::Override> = all
            .into_iter()
            .filter(|o| o.kind == crate::ovr::Kind::Db)
            .collect();
        if ovr.is_empty() && self.m.sql.ovr_views.is_empty() {
            return Ok(());
        }
        let up = sql.to_ascii_uppercase();
        let uses = |name: &str| sql_find_word(&up, name).is_some();
        let mut names: Vec<String> = ovr.iter().map(|o| o.file.to_ascii_uppercase()).collect();
        names.extend(self.m.sql.ovr_views.keys().cloned());
        names.sort();
        names.dedup();
        for name in names {
            if !uses(&name) {
                continue;
            }
            let body = match crate::ovr::effective(&ovr, crate::ovr::Kind::Db, &name) {
                Some(o) => {
                    let to = o.tofile.clone().unwrap_or_else(|| name.clone());
                    let (lib, file) = match to.split_once('/') {
                        Some((l, f)) if l != "*LIBL" => {
                            (Some(crate::qtemp::real(l)), f.to_string())
                        }
                        Some((_, f)) => (None, f.to_string()),
                        None => (None, to.clone()),
                    };
                    let libs = match &lib {
                        Some(l) => vec![l.clone()],
                        None => self.m.io.libl.clone(),
                    };
                    let rt = self.m.io.rt.clone();
                    let client = self.m.io.client;
                    let mut found = None;
                    for l in &libs {
                        let attr: Option<String> = rt
                            .block_on(client.query_opt(
                                "SELECT attribute FROM as4tainr.objects WHERE library=$1 AND name=$2 AND objtype='*FILE'",
                                &[l, &file],
                            ))?
                            .map(|r| r.get(0));
                        if let Some(a) = attr {
                            found = Some((l.clone(), a));
                            break;
                        }
                    }
                    let Some((l, attr)) = found else { continue };
                    let q = |v: &str| format!("'{}'", v.replace('\'', "''"));
                    if attr == "PF-SRC" {
                        let mbr = match &o.mbr {
                            Some(m) if !m.starts_with('*') => m.clone(),
                            _ => rt
                                .block_on(client.query_opt(
                                    "SELECT member FROM as4tainr.source_members WHERE library=$1 AND file=$2 ORDER BY added, member LIMIT 1",
                                    &[&l, &file],
                                ))?
                                .map(|r| r.get(0))
                                .unwrap_or_default(),
                        };
                        Some(format!(
                            "SELECT srcseq, srcdat, srcdta FROM as4tainr_svc.source_lines({}, {}, {})",
                            q(&l),
                            q(&file),
                            q(&mbr)
                        ))
                    } else {
                        let f = rt.block_on(crate::catalog::resolve_file(
                            client,
                            std::slice::from_ref(&l),
                            &file,
                        ))?;
                        let cols: Vec<String> = f.file.fields.iter().map(|x| x.column()).collect();
                        Some(format!(
                            "SELECT {} FROM {}.{}",
                            cols.join(", "),
                            crate::catalog::ident(&l),
                            crate::catalog::ident(&file)
                        ))
                    }
                }
                None => None,
            };
            let had = self.m.sql.ovr_views.get(&name).cloned();
            if had.as_deref() == body.as_deref() {
                continue;
            }
            let rt = self.m.io.rt.clone();
            let s = self.session_n()?;
            rt.block_on(s.temp_view(&name, body.as_deref()))?;
            match body {
                Some(b) => {
                    self.m.sql.ovr_views.insert(name, b);
                }
                None => {
                    self.m.sql.ovr_views.remove(&name);
                }
            }
        }
        Ok(())
    }

    /// exec_sql.rs session: the program's SQL session, opened the first
    /// time (under the commitment definition of its activation group or
    /// job when its commit level is not *NONE).
    fn session_n(&mut self) -> NResult<&crate::sql::Session> {
        if self.m.sql.session.is_none() {
            let level = self.m.prog.sql_commit.clone();
            let s = if level == "*NONE" {
                self.m.block(crate::sql::Session::open(
                    self.m.io.client,
                    &self.m.io.database_url,
                    &self.m.io.libl,
                ))?
            } else {
                let d = match crate::cmtctl::find(&self.m.io.job_key, &self.m.io.actgrp) {
                    Some(d) => d,
                    None => self.m.block(crate::cmtctl::start(
                        self.m.io.client,
                        &self.m.io.database_url,
                        &self.m.io.job_key,
                        &self.m.io.actgrp,
                        &level,
                        false,
                    ))?,
                };
                self.m.block(crate::sql::Session::open_in(
                    self.m.io.client,
                    &self.m.io.database_url,
                    &self.m.io.libl,
                    d,
                ))?
            };
            let mut s = s;
            let job = self.m.io.job.as_ref().map(|j| j.to_string());
            let user = self.m.io.user.clone();
            self.m.block(s.set_context(&user, None, job))?;
            if let Some((l, n)) = self.m.io.program.split_once('/') {
                let (l, n) = (l.to_string(), n.to_string());
                self.m.block(s.set_program(&n, &l))?;
            }
            self.m.sql.session = Some(s);
        }
        self.m
            .sql
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("native: no SQL session"))
    }
}

/// An outcome as generated code takes it (S2.71).
fn sql_result(o: SqlOut) -> rpgrt::native::SqlResult {
    rpgrt::native::SqlResult {
        code: o.code,
        state: o.state,
        text: o.text,
        rows: o.rows,
        errd5: o.errd5,
    }
}

/// exec_sql.rs strip_cursor_clauses (S2.72): a cursor's query without its
/// FOR UPDATE [OF ...], FOR READ ONLY or FOR FETCH ONLY, which runs to
/// the end or to OPTIMIZE or WITH after it.
fn sql_strip_cursor_clauses(query: &str) -> String {
    let mut q = query.trim().trim_end_matches(';').to_string();
    while let Some(at) = sql_find_word(&q, "FOR") {
        let rest: Vec<String> = q
            .get(at.saturating_add(3)..)
            .unwrap_or("")
            .split_whitespace()
            .take(2)
            .map(|w| w.to_ascii_uppercase())
            .collect();
        let is_clause = match rest.first().map(String::as_str) {
            Some("UPDATE") => true,
            Some("READ") | Some("FETCH") => rest.get(1).map(String::as_str) == Some("ONLY"),
            _ => false,
        };
        if !is_clause {
            break;
        }
        let tail = q.get(at..).unwrap_or("").to_string();
        let end = ["OPTIMIZE", "WITH"]
            .iter()
            .filter_map(|w| sql_find_word(&tail, w))
            .min()
            .unwrap_or(tail.len());
        q = format!(
            "{} {}",
            q.get(..at).unwrap_or("").trim_end(),
            tail.get(end..).unwrap_or("")
        )
        .trim()
        .to_string();
    }
    q
}

/// exec_sql.rs with_rrn (S2.72): a query over one table with each row's
/// record number last (RRN of the table's last name part or its
/// correlation name); None for a union, a group, a join, DISTINCT, or a
/// table reference it cannot tell.
fn sql_with_rrn(query: &str) -> Option<String> {
    let from = sql_find_word(query, "FROM")?;
    for w in ["UNION", "EXCEPT", "INTERSECT", "GROUP", "JOIN", "DISTINCT"] {
        if sql_find_word(query, w).is_some() {
            return None;
        }
    }
    let rest = query.get(from.checked_add(4)?..)?;
    let end = ["WHERE", "ORDER", "FOR", "FETCH", "OPTIMIZE", "WITH"]
        .iter()
        .filter_map(|w| sql_find_word(rest, w))
        .min()
        .unwrap_or(rest.len());
    let table_ref = rest.get(..end)?.trim();
    if table_ref.is_empty() || table_ref.contains(',') || table_ref.contains('(') {
        return None;
    }
    let parts: Vec<&str> = table_ref.split_whitespace().collect();
    let corr = match parts.as_slice() {
        [t] => t.rsplit(['.', '/']).next().unwrap_or(t),
        [_, a] => *a,
        [_, as_, a] if as_.eq_ignore_ascii_case("AS") => *a,
        _ => return None,
    };
    Some(format!(
        "{}, RRN({corr}) {}",
        query.get(..from)?.trim_end(),
        query.get(from..)?
    ))
}

/// A multiple-row statement's row count (S2.74), as exec_sql.rs takes it:
/// the number's integer value (`to_i128`), at least 0.
fn sql_row_count(count: &Dec) -> NResult<usize> {
    let n = count.to_i128()?.max(0);
    Ok(usize::try_from(n).unwrap_or(usize::MAX))
}

// ---- printer files: EXCEPT (S2.78) ------------------------------------

impl Rt<'_, '_> {
    /// interp.rs conditions_met: indicator groups joined by OR, each met
    /// when every indicator is as wanted.
    fn conditions_met_n(&self, cond: &super::super::ast::Conditions) -> bool {
        cond.split(|(_, i)| i == crate::rpg::parse::OR_MARK)
            .any(|g| g.iter().all(|(neg, i)| self.m.ind(i) != *neg))
    }

    /// interp.rs out_cond: a record with no conditions, or one group met.
    fn out_cond_n(&self, groups: &[super::super::ast::Conditions]) -> bool {
        groups.iter().all(|g| g.is_empty())
            || groups
                .iter()
                .any(|g| !g.is_empty() && self.conditions_met_n(g))
    }

    /// interp.rs except: the exception records with that name (or none)
    /// whose conditions are met, printed in order.
    fn except_n(&mut self, name: Option<&str>, fields: &[rpgrt::native::OutSlot]) -> NResult<()> {
        let prog = self.m.prog;
        for r in &prog.outputs {
            if r.kind == 'E' && r.name.as_deref() == name && self.out_cond_n(&r.cond) {
                self.print_record_n(r, fields)?;
            }
        }
        Ok(())
    }

    /// interp.rs overflow_ind: a printer file's OFLIND indicator.
    fn overflow_ind_n(&self, file: &str) -> Option<String> {
        let i = self.m.prog.file_index(file)?;
        let f = self.m.prog.files.get(i)?;
        super::super::ast::keyword(&f.decl.keywords, "OFLIND")
            .and_then(|k| k.args.first())
            .map(|a| {
                let a = a.trim().to_ascii_uppercase();
                a.strip_prefix("*IN").map(str::to_string).unwrap_or(a)
            })
    }

    /// interp.rs overflow_line: a heading, detail or total line
    /// conditioned by its printer's overflow indicator, which is on.
    fn overflow_line_n(&self, r: &super::super::ast::OutRec) -> bool {
        if !matches!(r.kind, 'H' | 'D' | 'T') {
            return false;
        }
        let Some(of) = self.overflow_ind_n(&r.file).map(|o| ind_name(&o)) else {
            return false;
        };
        self.m.ind(&of)
            && r.cond
                .iter()
                .any(|g| g.iter().any(|(neg, i)| !neg && ind_name(i) == of))
    }

    /// interp.rs overflow_routine: for each printer (or the one given)
    /// whose overflow indicator is on and whose overflow lines were not
    /// printed yet, those lines whose conditions are met; once each time
    /// overflow occurs.
    fn overflow_routine_n(
        &mut self,
        only: Option<&str>,
        fields: &[rpgrt::native::OutSlot],
    ) -> NResult<()> {
        let prog = self.m.prog;
        let files: Vec<String> = self
            .m
            .ofl_inds
            .iter()
            .filter(|(ind, f)| {
                self.m.ind(ind) && !self.m.ofl_written.contains(f) && only.is_none_or(|o| o == f)
            })
            .map(|(_, f)| f.clone())
            .collect();
        for f in files {
            self.m.ofl_written.insert(f.clone());
            for r in &prog.outputs {
                if r.file == f && self.overflow_line_n(r) && self.out_cond_n(&r.cond) {
                    self.print_line_n(r, fields)?;
                }
            }
        }
        Ok(())
    }

    /// interp.rs print_record: fetch overflow first (the overflow lines,
    /// when overflow has occurred, unless the record is conditioned by
    /// the overflow indicator itself), then the line.
    fn print_record_n(
        &mut self,
        r: &super::super::ast::OutRec,
        fields: &[rpgrt::native::OutSlot],
    ) -> NResult<()> {
        if r.fetch_overflow {
            let of = self.overflow_ind_n(&r.file).map(|o| ind_name(&o));
            let on_of = of.as_ref().is_some_and(|of| {
                r.cond
                    .iter()
                    .any(|g| g.iter().any(|(_, i)| ind_name(i) == *of))
            });
            if !on_of {
                self.overflow_routine_n(Some(&r.file), fields)?;
            }
        }
        self.print_line_n(r, fields)
    }

    /// printer.rs form_of: FORMLEN and FORMOFL on the file, else the
    /// printer file's page size and overflow line, else 66 and 60.
    fn form_of_n(&self, i: usize) -> NResult<(i32, i32)> {
        let f = self
            .m
            .prog
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: a printer file"))?;
        let kw = |k: &str| -> Option<i32> {
            super::super::ast::keyword(&f.decl.keywords, k)
                .and_then(|k| k.args.first())
                .and_then(|a| a.trim().parse().ok())
        };
        let (len, ovf) = match f.page {
            Some((l, _, o)) => (l, o),
            None => (crate::spool::PAGE_LENGTH, crate::spool::OVERFLOW_LINE),
        };
        Ok((kw("FORMLEN").unwrap_or(len), kw("FORMOFL").unwrap_or(ovf)))
    }

    /// interp.rs print_line for a printer file: space and skip before (a
    /// new page when the skip goes back, or past the page), the fields
    /// where they end (an end position, +n after the previous one, or
    /// right after it), a field cleared after it is printed (blank
    /// after), the line kept, then space and skip after (nothing given:
    /// one line); the overflow indicator on when the line was printed on
    /// or past the overflow line or the spacing passed it; the buffer
    /// written to the spooled file every 50 lines.
    fn print_line_n(
        &mut self,
        r: &super::super::ast::OutRec,
        fields: &[rpgrt::native::OutSlot],
    ) -> NResult<()> {
        let i = self
            .m
            .prog
            .file_index(&r.file)
            .ok_or_else(|| anyhow!("output to {}: not a file of this program", r.file))?;
        let width = {
            let decl = &self
                .m
                .prog
                .files
                .get(i)
                .ok_or_else(|| anyhow!("native: a printer file"))?
                .decl;
            if decl.device != "PRINTER" {
                return Err(anyhow!(
                    "native: output to {} ({}) is not lowered",
                    r.file,
                    decl.device
                ));
            }
            usize::try_from(decl.record_len.unwrap_or(132))?
        };
        let (page_length, overflow_line) = self.form_of_n(i)?;
        self.ensure_printer_n(&r.file)?;
        fn new_page(p: &mut super::Printer) {
            p.page = p.page.saturating_add(1);
            p.line = 1;
            p.printed_on_page = false;
        }
        let page = {
            let p = self
                .m
                .printers
                .get_mut(&r.file)
                .ok_or_else(|| anyhow!("native: the printer of {}", r.file))?;
            if let Some(s) = r.skip_before.filter(|s| *s > 0) {
                let s = i32::try_from(s)?;
                if s < p.line || s == p.line && p.printed_on_page {
                    new_page(p);
                }
                p.line = s;
            }
            if let Some(n) = r.space_before.filter(|n| *n > 0) {
                p.line = p.line.saturating_add(i32::try_from(n)?);
            }
            if p.line > page_length {
                new_page(p);
            }
            p.page
        };
        let mut buf = vec![' '; width];
        let mut prev_end = 0usize;
        for f in &r.fields {
            if !f.cond.is_empty() && !self.conditions_met_n(&f.cond) {
                continue;
            }
            let text: Vec<char> = self.out_text_n(f, page, fields)?.chars().collect();
            let end = match (f.end, f.relative) {
                (Some(e), _) => usize::try_from(e)?,
                (None, Some(n)) => prev_end
                    .saturating_add(usize::try_from(n)?)
                    .saturating_add(text.len()),
                (None, None) => prev_end.saturating_add(text.len()),
            };
            let start = end.saturating_sub(text.len());
            for (k, c) in text.iter().enumerate() {
                if let Some(slot) = start.checked_add(k).and_then(|x| buf.get_mut(x)) {
                    *slot = *c;
                }
            }
            prev_end = end;
            if f.blank_after {
                if let Some(n) = &f.name {
                    self.out_clear_n(n, fields)?;
                }
            }
        }
        let text: String = buf.into_iter().collect::<String>().trim_end().to_string();
        let overflow_ind = self.overflow_ind_n(&r.file);
        let p = self
            .m
            .printers
            .get_mut(&r.file)
            .ok_or_else(|| anyhow!("native: the printer of {}", r.file))?;
        p.buffer.push((p.page, p.line, text));
        p.printed_on_page = true;
        let (printed_page, printed_line) = (p.page, p.line);
        let mut moved = false;
        if let Some(s) = r.skip_after.filter(|s| *s > 0) {
            let s = i32::try_from(s)?;
            if s <= p.line {
                new_page(p);
            }
            p.line = s;
            moved = true;
        }
        if let Some(n) = r.space_after {
            p.line = p.line.saturating_add(i32::try_from(n)?);
            moved = true;
        }
        if !moved && r.skip_before.is_none() && r.space_before.is_none() {
            p.line = p.line.saturating_add(1);
        }
        let overflow =
            printed_line >= overflow_line || p.page == printed_page && p.line > overflow_line;
        if p.line > page_length {
            new_page(p);
        }
        let full = p.buffer.len() >= 50;
        if overflow {
            if let Some(of) = overflow_ind {
                self.m.set_ind(&of, true);
            }
        }
        if full {
            self.flush_printer_n(&r.file)?;
        }
        Ok(())
    }

    /// A field of an output specification as lowering placed it.
    fn out_slot<'f>(
        name: &str,
        fields: &'f [rpgrt::native::OutSlot],
    ) -> NResult<&'f rpgrt::native::OutSlot> {
        fields
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| anyhow!("O spec field {name} is not defined"))
    }

    /// The field a blank-after output field names, cleared (interp.rs
    /// print_line's CLEAR of it, the Sym::Var arm): its type's initial
    /// image over all its storage (blanks, zeros; a varying field's
    /// length and every byte after it, as the review of 2026-11-18 found
    /// an empty value alone left them).
    fn out_clear_n(&mut self, name: &str, fields: &[rpgrt::native::OutSlot]) -> NResult<()> {
        let f = Self::out_slot(name, fields)?;
        let img = f.ty.initial();
        let end = f
            .off
            .checked_add(img.len())
            .ok_or_else(|| anyhow!("native: {name} past the end of storage"))?;
        let dst = self
            .m
            .bufs
            .get_mut(f.buf)
            .and_then(|b| b.get_mut(f.off..end))
            .ok_or_else(|| anyhow!("native: {name} past the end of its storage"))?;
        dst.copy_from_slice(&img);
        Ok(())
    }

    /// interp.rs out_text: a literal; PAGE (to PAGE7), UDATE and *DATE;
    /// a character field's characters; a number edited by its edit word
    /// (a named constant when it is not quoted), its edit code, or none
    /// (its zoned digits, the sign in the last digit's zone).
    fn out_text_n(
        &mut self,
        f: &super::super::ast::OutField,
        page: i32,
        fields: &[rpgrt::native::OutSlot],
    ) -> NResult<String> {
        if let Some(lit) = &f.literal {
            return Ok(a(lit));
        }
        let name = f.name.clone().unwrap_or_default();
        let (value, digits, decs) = match name.as_str() {
            "PAGE" | "PAGE1" | "PAGE2" | "PAGE3" | "PAGE4" | "PAGE5" | "PAGE6" | "PAGE7" => {
                (Dec::from_i64(i64::from(page)), 4, 0)
            }
            "UDATE" => (Dec::from_i64(super::today_num("UDATE")), 6, 0),
            "*DATE" => (Dec::from_i64(super::today_num("*DATE")), 8, 0),
            _ => {
                let slot = Self::out_slot(&name, fields)?;
                let v = rpgrt::native::load(&self.m.bufs, slot.buf, slot.off, slot.ty)?;
                if !slot.ty.is_numeric() {
                    return Ok(a(&v.chars()?));
                }
                let (p, s) = slot.ty.precision();
                (v.num()?, p, s)
            }
        };
        if let Some(w) = &f.edit_word {
            let word = match w.strip_prefix('\u{1}') {
                Some(n) => match self.m.prog.syms.get(n) {
                    Some(super::super::program::Sym::Const(v)) => a(&v.chars()?),
                    _ => return Err(anyhow!("edit word {n} is not a named constant")),
                },
                None => w.clone(),
            };
            return super::edit_word(&value, digits, decs, &word);
        }
        Ok(match f.edit_code {
            Some(c) => super::edit_code_with(&value, digits, decs, c, self.m.prog.decedit()),
            None => {
                let t = super::super::data::DType::Zoned(digits, decs);
                let mut b = t.initial();
                data::store(
                    &t,
                    &Value::Num(value),
                    &mut b,
                    Fit::Truncate { half_adjust: false },
                    false,
                )?;
                a(&b)
            }
        })
    }

    /// printer.rs ensure_printer: the printer of a file, its spooled file
    /// opened on first use (OVRPRTF SPLFNAME and USRDTA in effect, of
    /// this activation group or none; with a job of as4tainr's, in it;
    /// in print-job mode, the user's print job, its lines also echoed;
    /// else none), the job's last spooled file noted for QSPRILSP.
    fn ensure_printer_n(&mut self, file: &str) -> NResult<()> {
        if self.m.printers.contains_key(file) {
            return Ok(());
        }
        let mut ovr = self
            .m
            .io
            .cl
            .as_ref()
            .map(|c| c.overrides.clone())
            .unwrap_or_default();
        crate::ovr::merge_scoped(&self.m.io.job_key, &mut ovr);
        let group = self.m.io.actgrp.clone();
        ovr.retain(|o| o.actgrp.as_ref().is_none_or(|g| *g == group));
        let o = crate::ovr::effective(&ovr, crate::ovr::Kind::Printer, file);
        let name = o
            .and_then(|o| o.splfname.clone())
            .unwrap_or_else(|| file.to_string());
        let usrdta = o
            .and_then(|o| o.usrdta.clone())
            .unwrap_or_else(|| self.m.prog.name.clone());
        let rt = self.m.io.rt.clone();
        let client = self.m.io.client;
        let mut echo = false;
        let mut print_job = None;
        let spl = match &self.m.io.job {
            Some(job) => Some(rt.block_on(crate::spool::open(client, job, &name, &usrdta))?),
            None if crate::spool::print_job_mode() => {
                echo = true;
                let user = self.m.io.user.clone();
                let pj = rt.block_on(crate::spool::print_job(client, &user))?;
                let id = rt.block_on(crate::spool::open(client, &pj, &name, &usrdta))?;
                crate::spool::note_temporary(id);
                print_job = Some(pj);
                Some(id)
            }
            None => None,
        };
        let number = match spl {
            Some(id) => rt
                .block_on(async {
                    Ok::<i32, anyhow::Error>(
                        client
                            .query_one(
                                "SELECT number FROM as4tainr.spooled_files WHERE id=$1",
                                &[&id],
                            )
                            .await?
                            .get::<_, i32>(0),
                    )
                })
                .unwrap_or(1),
            None => crate::rpg::last_spooled_file(&self.m.io.job_key)
                .map_or(1, |l| l.4.saturating_add(1)),
        };
        let (jname, juser, jnbr) = match self.m.io.job.as_ref().or(print_job.as_ref()) {
            Some(j) => (j.name.clone(), j.user.clone(), format!("{:06}", j.number)),
            None => (String::new(), self.m.io.user.clone(), String::new()),
        };
        crate::rpg::set_last_spooled_file(
            &self.m.io.job_key,
            (name.clone(), jname, juser, jnbr, number),
        );
        self.m.printers.insert(
            file.to_string(),
            super::Printer {
                spl,
                page: 1,
                line: 1,
                printed_on_page: false,
                buffer: Vec::new(),
                out_pos: (1, 0),
                echo,
            },
        );
        Ok(())
    }

    /// interp.rs flush_printer: the lines kept written to the spooled
    /// file; without one (or echoed), to the program's printed output:
    /// a form feed for a new page, blank lines up to the line, a line
    /// printed again over the last one.
    fn flush_printer_n(&mut self, file: &str) -> NResult<()> {
        let client = self.m.io.client;
        let rt = self.m.io.rt.clone();
        let Some(p) = self.m.printers.get_mut(file) else {
            return Ok(());
        };
        let lines = std::mem::take(&mut p.buffer);
        if let Some(id) = p.spl {
            rt.block_on(crate::spool::write(client, id, &lines))?;
        }
        match (p.spl, p.echo) {
            (Some(_), false) => {}
            _ => {
                for (page, line, text) in lines {
                    if page > p.out_pos.0 {
                        self.m.printed.push("\x0c".to_string());
                        p.out_pos = (page, 0);
                    }
                    if line == p.out_pos.1 && line > 0 {
                        if let Some(last) = self.m.printed.last_mut() {
                            *last = super::overprint(last, &text);
                            continue;
                        }
                    }
                    while p.out_pos.1.saturating_add(1) < line {
                        self.m.printed.push(String::new());
                        p.out_pos.1 = p.out_pos.1.saturating_add(1);
                    }
                    self.m.printed.push(text);
                    p.out_pos.1 = line;
                }
            }
        }
        Ok(())
    }
}

// ---- printer files: WRITE of a record format (S2.79) ------------------

impl Rt<'_, '_> {
    /// errors.rs file_op for WRITE of a printer file's record format: the
    /// file's error status off, a savepoint under commitment control,
    /// printer.rs printer_write, then file_op's end (file_end_n).
    fn printer_write_n(
        &mut self,
        pc: usize,
        fields: &[rpgrt::native::OutSlot],
        indds: Option<&rpgrt::native::OutSlot>,
    ) -> NResult<()> {
        let (op, file, _, _, _) = self.file_stmt(pc)?;
        let file = file.to_string();
        if op != FileOp::Write {
            return Err(mismatch(pc, "a WRITE"));
        }
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
        let r = self.printer_write_inner_n(i, &file, fields, indds);
        self.file_end_n(i, op, &file, cmt.as_deref(), r.as_ref().map(|_| ()))?;
        r
    }

    /// errors.rs file_op for OPEN, CLOSE or FEOD of a printer file around
    /// printer.rs printer_op: OPEN and FEOD write the lines kept; CLOSE
    /// writes them, and the spooled file is complete (an OPEN after it
    /// starts a new one).
    fn printer_op_n(&mut self, pc: usize) -> NResult<()> {
        let (op, file, _, _, _) = self.file_stmt(pc)?;
        let file = file.to_string();
        if !matches!(op, FileOp::Open | FileOp::Close | FileOp::Feod) {
            return Err(mismatch(pc, "an OPEN, CLOSE or FEOD"));
        }
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
        let name = self
            .m
            .prog
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?
            .decl
            .name
            .clone();
        let r = (|| -> NResult<()> {
            self.flush_printer_n(&name)?;
            if op == FileOp::Close {
                if let Some(p) = self.m.printers.remove(&name) {
                    if let Some(id) = p.spl {
                        let client = self.m.io.client;
                        self.m.block(crate::spool::close(client, id))?;
                    }
                }
            }
            Ok(())
        })();
        self.file_end_n(i, op, &file, cmt.as_deref(), r.as_ref().map(|_| ()))?;
        r
    }

    /// printer.rs printer_write for an externally described printer file:
    /// the record format, the printer, the indicators (INDDS's bytes or
    /// *IN01-*IN99), the page the record prints on after its SKIPB or
    /// SPACEB (PAGNBR), each printed field's text in order, the layout
    /// (prtf::layout), then print_ops.
    fn printer_write_inner_n(
        &mut self,
        i: usize,
        fmt: &str,
        fields: &[rpgrt::native::OutSlot],
        indds: Option<&rpgrt::native::OutSlot>,
    ) -> NResult<()> {
        let prog = self.m.prog;
        let fi = prog
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?;
        let Some(dds) = fi.printer.as_ref() else {
            return Err(anyhow!(
                "WRITE {fmt}: printer file {} is program-described; it is written with EXCEPT and output specifications",
                fi.decl.name
            ));
        };
        let rec = dds.record(fmt).ok_or_else(|| {
            anyhow!(
                "WRITE {fmt}: printer file {} has no record format {fmt}",
                fi.decl.name
            )
        })?;
        let file = fi.decl.name.clone();
        self.ensure_printer_n(&file)?;
        let mut ind = [false; 100];
        match indds {
            Some(d) => {
                let v = rpgrt::native::load(&self.m.bufs, d.buf, d.off, d.ty)?;
                let bytes = v.chars()?;
                for (n, slot) in ind.iter_mut().enumerate().skip(1) {
                    *slot = n.checked_sub(1).and_then(|k| bytes.get(k)) == Some(&0xF1);
                }
            }
            None => {
                for (n, slot) in ind.iter_mut().enumerate().skip(1) {
                    *slot = self.m.ind(&format!("{n:02}"));
                }
            }
        }
        let (page_length, _) = self.form_of_n(i)?;
        let page = {
            let p = self
                .m
                .printers
                .get(&file)
                .ok_or_else(|| anyhow!("native: the printer of {file}"))?;
            let first = |v: &[(Option<crate::dspf::Cond>, u32)]| -> Option<i32> {
                v.iter()
                    .find(|(c, _)| crate::dspf::cond_true(c, &ind))
                    .and_then(|(_, n)| i32::try_from(*n).ok())
            };
            let mut line = p.line;
            let mut page = p.page;
            if let Some(n) = first(&rec.spacing.skip_before) {
                if n < line || n == line && p.printed_on_page {
                    page = page.saturating_add(1);
                }
                line = n;
            }
            if let Some(n) = first(&rec.spacing.space_before) {
                if line.saturating_add(n) > page_length {
                    page = page.saturating_add(1);
                }
            }
            page
        };
        let mut texts: Vec<String> = Vec::new();
        for f in &rec.fields {
            if !f.printed
                || !crate::dspf::cond_true(&f.cond, &ind)
                || matches!(f.content, crate::prtf::Content::Literal(_))
            {
                continue;
            }
            texts.push(self.printer_text_n(f, page, fields)?);
        }
        let mut it = texts.into_iter();
        let ops = crate::prtf::layout(rec, &ind, &mut |_f: &crate::prtf::PField| {
            Ok(it.next().unwrap_or_default())
        })?;
        self.print_ops_n(i, &file, &ops)
    }

    /// printer.rs printer_text: a literal; DATE (MMDDYY or MMDDYYYY), TIME
    /// (HHMMSS of the clock) and PAGNBR; a field's value (a signed
    /// numeric field with decimals as a number, anything else as its
    /// characters padded or cut to the field's length); a number with its
    /// edit word, its edit code, or its zoned digits.
    fn printer_text_n(
        &mut self,
        f: &crate::prtf::PField,
        page: i32,
        fields: &[rpgrt::native::OutSlot],
    ) -> NResult<String> {
        use crate::prtf::Content;
        let (value, digits, decs) = match &f.content {
            Content::Literal(t) => return Ok(t.clone()),
            Content::Date { four_digit } => {
                let (y, m, d) = super::today();
                let (y, m, d) = (i64::from(y), i64::from(m), i64::from(d));
                let v = if *four_digit {
                    m.saturating_mul(1_000_000)
                        .saturating_add(d.saturating_mul(10_000))
                        .saturating_add(y)
                } else {
                    m.saturating_mul(10_000)
                        .saturating_add(d.saturating_mul(100))
                        .saturating_add(y.rem_euclid(100))
                };
                (Dec::from_i64(v), if *four_digit { 8 } else { 6 }, 0)
            }
            Content::Time => {
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
                    % 86_400;
                let v = (secs / 3600)
                    .saturating_mul(10_000)
                    .saturating_add((secs / 60 % 60).saturating_mul(100))
                    .saturating_add(secs % 60);
                (Dec::from_i64(i64::try_from(v)?), 6, 0)
            }
            Content::PageNumber => (Dec::from_i64(i64::from(page)), 4, 0),
            Content::Field => {
                if matches!(f.dtype, 'L' | 'T') {
                    return Err(anyhow!(
                        "native: a date or time printer field is not lowered"
                    ));
                }
                let slot = Self::out_slot(&f.name, fields)?;
                let v = rpgrt::native::load(&self.m.bufs, slot.buf, slot.off, slot.ty)?;
                match f.decimals {
                    Some(d) if f.dtype == 'S' => (v.num()?, u32::from(f.length), u32::from(d)),
                    _ => {
                        let t = a(&v.chars()?);
                        let n = usize::from(f.length);
                        return Ok(format!("{:<n$}", t.chars().take(n).collect::<String>()));
                    }
                }
            }
        };
        if let Some(w) = &f.edit_word {
            return super::edit_word(&value, digits, decs, w);
        }
        Ok(match f.edit_code {
            Some(c) => super::edit_code(&value, digits, decs, c),
            None => {
                let t = super::super::data::DType::Zoned(digits, decs);
                let mut b = t.initial();
                data::store(
                    &t,
                    &Value::Num(value),
                    &mut b,
                    Fit::Truncate { half_adjust: false },
                    false,
                )?;
                a(&b)
            }
        })
    }

    /// printer.rs print_ops: skips and lines (a new page when not below
    /// the current line), spacing (a new page past the page's end), text
    /// lines kept; the overflow indicator on when the overflow line is
    /// reached or passed; the lines written every 50.
    fn print_ops_n(&mut self, i: usize, file: &str, ops: &[crate::prtf::PrintOp]) -> NResult<()> {
        use crate::prtf::PrintOp;
        let (page_length, overflow_line) = self.form_of_n(i)?;
        let overflow_ind = self.overflow_ind_n(file);
        let p = self
            .m
            .printers
            .get_mut(file)
            .ok_or_else(|| anyhow!("native: the printer of {file}"))?;
        fn new_page(p: &mut super::Printer) {
            p.page = p.page.saturating_add(1);
            p.line = 1;
            p.printed_on_page = false;
        }
        let mut overflow = false;
        for op in ops {
            match op {
                PrintOp::Skip(n) | PrintOp::Line(n) => {
                    let n = i32::try_from(*n)?;
                    if n < p.line || n == p.line && p.printed_on_page {
                        new_page(p);
                    } else if p.line <= overflow_line && n > overflow_line {
                        overflow = true;
                    }
                    p.line = n;
                }
                PrintOp::Space(n) => {
                    let before = p.line;
                    p.line = p.line.saturating_add(i32::try_from(*n)?);
                    if before <= overflow_line && p.line > overflow_line {
                        overflow = true;
                    }
                    if p.line > page_length {
                        new_page(p);
                    }
                }
                PrintOp::Text(t) => {
                    if p.line > page_length {
                        new_page(p);
                    }
                    p.buffer.push((p.page, p.line, t.clone()));
                    p.printed_on_page = true;
                    if p.line >= overflow_line {
                        overflow = true;
                    }
                }
            }
        }
        let full = p.buffer.len() >= 50;
        if overflow {
            if let Some(of) = overflow_ind {
                self.m.set_ind(&of, true);
            }
        }
        if full {
            self.flush_printer_n(file)?;
        }
        Ok(())
    }
}

// ---- data areas: IN and OUT (S2.80) -----------------------------------

impl Rt<'_, '_> {
    /// interp.rs data_area: the data area the field names (the program's
    /// DEFINE or DTAARA; *VAR: the name in a field); *LDA (the local data
    /// area); *GDA and *PDA not available; else found on the library
    /// list. IN: *USE authority, *LOCK's *EXCLRD object lock (waiting
    /// the job's default wait, MCH5802), the value (a number parsed, or
    /// characters) into the field; OUT: *CHANGE authority, the field's
    /// value written, the program's and the job's locks given back.
    fn data_area_n(&mut self, input: bool, lock: bool, f: &rpgrt::native::DtaSlot) -> NResult<()> {
        let field = f.field.name.as_str();
        let name = self
            .m
            .prog
            .dtaaras
            .get(field)
            .cloned()
            .ok_or_else(|| anyhow!("{field} is not defined as a data area (DEFINE *DTAARA)"))?;
        let name = match name.strip_prefix("*VAR:") {
            Some(_) => {
                let v = f
                    .var
                    .as_ref()
                    .ok_or_else(|| anyhow!("native: the *VAR field of {field}"))?;
                let n = rpgrt::native::load(&self.m.bufs, v.buf, v.off, v.ty)?.chars()?;
                a(&n).trim().to_ascii_uppercase()
            }
            None => name,
        };
        if name == "*LDA" {
            return self.local_data_area_n(input, &f.field);
        }
        if name == "*GDA" || name == "*PDA" {
            return Err(anyhow!("RNX0402: Data area {name} is not available: this job is not a group or prestart job."));
        }
        let libl = self.m.io.libl.clone();
        let cur = crate::commands::DEFAULT_CURRENT_LIBRARY;
        let client = self.m.io.client;
        let d = self
            .m
            .block(crate::dtaara::resolve(client, &name, &libl, cur))?;
        let key = (d.library.clone(), d.name.clone());
        let held = self.m.dtaara_locks.contains(&key);
        if input {
            self.m.block(crate::authority::check(
                client,
                &self.m.io.user,
                &d.library,
                &d.name,
                "*DTAARA",
                crate::authority::Aut::Use,
            ))?;
            let take = lock && !held;
            if take {
                let t = crate::objlock::Target {
                    library: d.library.clone(),
                    name: d.name.clone(),
                    objtype: "*DTAARA".into(),
                };
                let wait = crate::objlock::default_wait(&self.m.io.job_key);
                let owner = self.m.io.job_key.clone();
                if !self.m.block(crate::objlock::allocate(
                    client,
                    &t,
                    "*EXCLRD",
                    Some(wait),
                    &owner,
                ))? {
                    return Err(anyhow!(
                        "MCH5802: Data area {} in library {} is locked by another job; IN *LOCK could not lock it.",
                        d.name,
                        crate::qtemp::logical(&d.library)
                    ));
                }
            }
            let d = self
                .m
                .block(crate::dtaara::read(client, &name, &libl, cur, take))?;
            if take {
                self.m.dtaara_locks.push(key);
            }
            let slot = &f.field;
            let v = if slot.ty.is_numeric() {
                Value::Num(Dec::parse(d.value.trim())?)
            } else {
                Value::str(&d.value)
            };
            rpgrt::native::store(
                &mut self.m.bufs,
                slot.buf,
                slot.off,
                slot.ty,
                &v,
                Fit::Truncate { half_adjust: false },
                false,
            )?;
        } else {
            self.m.block(crate::authority::check(
                client,
                &self.m.io.user,
                &d.library,
                &d.name,
                "*DTAARA",
                crate::authority::Aut::Change,
            ))?;
            let slot = &f.field;
            let v = rpgrt::native::load(&self.m.bufs, slot.buf, slot.off, slot.ty)?;
            let text = match v {
                Value::Num(n) => n.to_string(),
                other => a(&other.chars()?).trim_end().to_string(),
            };
            self.m
                .block(crate::dtaara::write(client, &d, &text, held))?;
            self.dtaara_unlock_object_n(&d.library, &d.name);
            self.m.dtaara_locks.retain(|k| *k != key);
        }
        Ok(())
    }

    /// interp.rs dtaara_unlock_object: the *EXCLRD object lock of IN
    /// *LOCK given back, and the job's kept one (errors ignored, as there).
    fn dtaara_unlock_object_n(&mut self, library: &str, name: &str) {
        let t = crate::objlock::Target {
            library: library.to_string(),
            name: name.to_string(),
            objtype: "*DTAARA".into(),
        };
        let _ = self
            .m
            .block(crate::objlock::deallocate(self.m.io.client, &t, "*EXCLRD"));
        let key = self.m.io.job_key.clone();
        let _ = self
            .m
            .block(crate::objlock::release_kept(&key, &t, "*EXCLRD"));
    }

    /// interp.rs local_data_area: the local data area's bytes into the
    /// field (as long as it is), or the field's bytes over its start.
    fn local_data_area_n(&mut self, input: bool, slot: &rpgrt::native::OutSlot) -> NResult<()> {
        let job = self.m.io.job.as_ref().map(|j| j.number);
        let client = self.m.io.client;
        let mut lda = self.m.block(crate::dtaara::lda_read(client, job))?;
        let n = slot.ty.size().min(lda.len());
        let end = slot
            .off
            .checked_add(n)
            .ok_or_else(|| anyhow!("native: the local data area's field"))?;
        let buf = self
            .m
            .bufs
            .get_mut(slot.buf)
            .and_then(|b| b.get_mut(slot.off..end))
            .ok_or_else(|| anyhow!("native: the local data area's field"))?;
        let part = lda
            .get_mut(..n)
            .ok_or_else(|| anyhow!("native: the local data area"))?;
        if input {
            buf.copy_from_slice(part);
        } else {
            part.copy_from_slice(buf);
            self.m.block(crate::dtaara::lda_write(client, job, &lda))?;
        }
        Ok(())
    }
}

// ---- fixed-form string operations (S2.83) -----------------------------

impl Rt<'_, '_> {
    /// ops.rs op_error: the error indicator on, or the error.
    fn op_error_n(&mut self, lo: &Option<String>, msg: String) -> NResult<()> {
        match lo {
            Some(lo) => {
                self.m.set_ind(lo, true);
                Ok(())
            }
            None => Err(anyhow!(msg)),
        }
    }

    /// ops.rs put_positions: every position found into a numeric array
    /// (the rest zero), or the first (or zero) into the field.
    fn put_positions_n(&mut self, result: &rpgrt::native::StrOut, found: &[usize]) -> NResult<()> {
        let rpgrt::native::StrOut::Positions { slot, array } = result else {
            return Ok(());
        };
        let fit = Fit::Strict { half_adjust: false };
        match array {
            Some((stride, dim)) => {
                for k in 0..usize::try_from(*dim)? {
                    let v = found.get(k).copied().unwrap_or(0);
                    let off = k
                        .checked_mul(*stride)
                        .and_then(|d| slot.off.checked_add(d))
                        .ok_or_else(|| anyhow!("native: an element's place"))?;
                    rpgrt::native::store(
                        &mut self.m.bufs,
                        slot.buf,
                        off,
                        slot.ty,
                        &Value::Num(Dec::from_i64(i64::try_from(v)?)),
                        fit,
                        false,
                    )?;
                }
            }
            None => {
                let v = found.first().copied().unwrap_or(0);
                rpgrt::native::store(
                    &mut self.m.bufs,
                    slot.buf,
                    slot.off,
                    slot.ty,
                    &Value::Num(Dec::from_i64(i64::try_from(v)?)),
                    fit,
                    false,
                )?;
            }
        }
        Ok(())
    }

    /// ops.rs put_left: the text over the start of the result's current
    /// value (blanks first with P), assigned with the truncating fit.
    fn put_left_n(&mut self, slot: &rpgrt::native::OutSlot, text: &[u8], pad: bool) -> NResult<()> {
        let mut cur = rpgrt::native::load(&self.m.bufs, slot.buf, slot.off, slot.ty)?.chars()?;
        if pad {
            cur.fill(data::BLANK);
        }
        let n = text.len().min(cur.len());
        if let (Some(dst), Some(src)) = (cur.get_mut(..n), text.get(..n)) {
            dst.copy_from_slice(src);
        }
        rpgrt::native::store(
            &mut self.m.bufs,
            slot.buf,
            slot.off,
            slot.ty,
            &Value::Char(cur),
            Fit::Truncate { half_adjust: false },
            false,
        )
    }

    /// ops.rs string_op for SCAN, CHECK, CHECKR, SUBST, XLATE and CAT, on
    /// the factors' values as lowering split them.
    fn str_op_n(
        &mut self,
        op: rpgrt::native::StrOpIn,
        result: &rpgrt::native::StrOut,
    ) -> NResult<()> {
        use rpgrt::native::StrCode;
        let to_i128 = |d: &Option<Dec>| -> NResult<Option<i128>> {
            d.as_ref().map(|d| d.to_i128()).transpose()
        };
        let lo_off = |rt: &mut Self| {
            if let Some(lo) = &op.lo {
                rt.m.set_ind(lo, false);
            }
        };
        let need = |v: &Option<Vec<u8>>, what: &str| -> NResult<Vec<u8>> {
            v.clone().ok_or_else(|| anyhow!("native: {what}"))
        };
        match op.code {
            StrCode::Scan => {
                lo_off(self);
                let cmp = need(&op.f1, "a compare string")?;
                let len = match to_i128(&op.f1_num)? {
                    Some(l) => usize::try_from(l).unwrap_or(usize::MAX),
                    None => cmp.len(),
                };
                let base = op.f2.clone();
                let start = to_i128(&op.f2_num)?.unwrap_or(1);
                let start_ok = usize::try_from(start)
                    .ok()
                    .is_some_and(|s| s >= 1 && s <= base.len());
                if len == 0 || len > cmp.len() || !start_ok {
                    return self.op_error_n(
                        &op.lo,
                        "RNQ0100: SCAN: the length or start position is not valid.".into(),
                    );
                }
                let needle = cmp.get(..len).unwrap_or(&[]);
                let mut found = Vec::new();
                let mut at = usize::try_from(start)?.saturating_sub(1);
                while at.checked_add(len).is_some_and(|e| e <= base.len()) {
                    if base.get(at..at.saturating_add(len)) == Some(needle) {
                        found.push(at.saturating_add(1));
                        if matches!(result, rpgrt::native::StrOut::None) {
                            break;
                        }
                    }
                    at = at.saturating_add(1);
                }
                self.put_positions_n(result, &found)?;
                if let Some(eq) = &op.eq {
                    self.m.set_ind(eq, !found.is_empty());
                }
            }
            StrCode::Check | StrCode::CheckR => {
                lo_off(self);
                let comparator = need(&op.f1, "a comparator string")?;
                let base = op.f2.clone();
                let start = match to_i128(&op.f2_num)? {
                    Some(s) => s,
                    None if op.code == StrCode::Check => 1,
                    None => i128::try_from(base.len())?,
                };
                let start_ok = usize::try_from(start)
                    .ok()
                    .is_some_and(|s| s >= 1 && s <= base.len());
                if !start_ok {
                    let code = if op.code == StrCode::Check {
                        "CHECK"
                    } else {
                        "CHECKR"
                    };
                    return self.op_error_n(
                        &op.lo,
                        format!("RNQ0100: {code}: the start position is not valid."),
                    );
                }
                let s0 = usize::try_from(start)?.saturating_sub(1);
                let multi = matches!(
                    result,
                    rpgrt::native::StrOut::Positions { array: Some(_), .. }
                );
                let mut found = Vec::new();
                if op.code == StrCode::Check {
                    for (k, b) in base.iter().enumerate().skip(s0) {
                        if !comparator.contains(b) {
                            found.push(k.saturating_add(1));
                            if !multi {
                                break;
                            }
                        }
                    }
                } else {
                    for k in (0..=s0).rev() {
                        if base.get(k).is_some_and(|b| !comparator.contains(b)) {
                            found.push(k.saturating_add(1));
                            if !multi {
                                break;
                            }
                        }
                    }
                }
                self.put_positions_n(result, &found)?;
                if let Some(eq) = &op.eq {
                    self.m.set_ind(eq, !found.is_empty());
                }
            }
            StrCode::Subst => {
                lo_off(self);
                let base = op.f2.clone();
                let blen = i128::try_from(base.len())?;
                let start = to_i128(&op.f2_num)?.unwrap_or(1);
                let len = match to_i128(&op.f1_num)? {
                    Some(l) => l,
                    None => blen.saturating_sub(start).saturating_add(1),
                };
                if start < 1 || len < 1 || start.saturating_sub(1).saturating_add(len) > blen {
                    return self.op_error_n(
                        &op.lo,
                        "RNQ0100: SUBST: the start position or length is outside the string."
                            .into(),
                    );
                }
                let from = usize::try_from(start.saturating_sub(1))?;
                let to = usize::try_from(start.saturating_add(len).saturating_sub(1))?;
                let piece = base
                    .get(from..to)
                    .ok_or_else(|| anyhow!("native: SUBST's piece"))?
                    .to_vec();
                let rpgrt::native::StrOut::Chars(r) = result else {
                    return Err(anyhow!("SUBST: a result field is needed"));
                };
                self.put_left_n(r, &piece, op.pad)?;
            }
            StrCode::Xlate => {
                lo_off(self);
                let from = need(&op.f1, "the from string")?;
                let to = need(&op.f1_chars, "the to string")?;
                let mut text = op.f2.clone();
                let start = to_i128(&op.f2_num)?.unwrap_or(1);
                let start_ok = usize::try_from(start)
                    .ok()
                    .is_some_and(|s| s >= 1 && s <= text.len());
                if !start_ok {
                    return self.op_error_n(
                        &op.lo,
                        "RNQ0100: XLATE: the start position is not valid.".into(),
                    );
                }
                for b in text
                    .iter_mut()
                    .skip(usize::try_from(start)?.saturating_sub(1))
                {
                    if let Some(p) = from.iter().position(|f| f == b) {
                        if let Some(t) = to.get(p) {
                            *b = *t;
                        }
                    }
                }
                let rpgrt::native::StrOut::Chars(r) = result else {
                    return Err(anyhow!("XLATE: a result field is needed"));
                };
                self.put_left_n(r, &text, op.pad)?;
            }
            StrCode::Cat => {
                let rpgrt::native::StrOut::Chars(r) = result else {
                    return Err(anyhow!("CAT: a result field is needed"));
                };
                let mut s = match &op.f1 {
                    Some(x) => x.clone(),
                    None => rpgrt::native::load(&self.m.bufs, r.buf, r.off, r.ty)?.chars()?,
                };
                if let Some(n) = to_i128(&op.f2_num)? {
                    let n = usize::try_from(n).unwrap_or(0);
                    while s.last() == Some(&data::BLANK) {
                        s.pop();
                    }
                    s.extend(std::iter::repeat_n(data::BLANK, n));
                }
                s.extend(op.f2.iter().copied());
                self.put_left_n(r, &s, op.pad)?;
            }
        }
        Ok(())
    }
}

// ---- fixed-form bit and test operations (S2.84) -----------------------

impl Rt<'_, '_> {
    /// interp.rs set_result_inds: each indicator given set.
    fn set_result_inds_n(
        &mut self,
        hi_i: &Option<String>,
        lo_i: &Option<String>,
        eq_i: &Option<String>,
        (hi, lo, eq): (bool, bool, bool),
    ) {
        if let Some(i) = hi_i {
            self.m.set_ind(i, hi);
        }
        if let Some(i) = lo_i {
            self.m.set_ind(i, lo);
        }
        if let Some(i) = eq_i {
            self.m.set_ind(i, eq);
        }
    }

    /// ops.rs bit_op for TESTN (digits, a sign zone on the last; leading
    /// blanks low; all blanks equal), TESTZ (the first character's zone:
    /// '&' and A-I high, '-' and J-R low), TESTB (bits off high, mixed
    /// low, on equal), BITON and BITOFF (the one byte changed, the strict
    /// fit).
    fn bit_op_n(&mut self, op: &rpgrt::native::BitOpIn) -> NResult<()> {
        use rpgrt::native::BitCode;
        let slot = &op.slot;
        let b = rpgrt::native::load(&self.m.bufs, slot.buf, slot.off, slot.ty)?.chars()?;
        match op.code {
            BitCode::TestN => {
                let digit = |c: u8, last: bool| {
                    (c & 0x0F) <= 9 && (c >> 4 == 0xF || (last && (c >> 4 == 0xC || c >> 4 == 0xD)))
                };
                let lead = b.iter().take_while(|c| **c == data::BLANK).count();
                let all_blank = lead == b.len();
                let rest = b.get(lead..).unwrap_or(&[]);
                let numeric = !rest.is_empty()
                    && rest
                        .iter()
                        .enumerate()
                        .all(|(i, c)| digit(*c, i.saturating_add(1) == rest.len()));
                self.set_result_inds_n(
                    &op.hi,
                    &op.lo,
                    &op.eq,
                    (numeric && lead == 0, numeric && lead > 0, all_blank),
                );
            }
            BitCode::TestZ => {
                let c = b.first().copied().unwrap_or(data::BLANK);
                let hi = c == 0x50 || c >> 4 == 0xC;
                let lo = c == 0x60 || c >> 4 == 0xD;
                self.set_result_inds_n(&op.hi, &op.lo, &op.eq, (hi, lo, !hi && !lo));
            }
            BitCode::TestB | BitCode::BitOn | BitCode::BitOff => {
                let mask = match (&op.mask, &op.mask_bytes) {
                    (Some(m), _) => *m,
                    (None, Some(bytes)) => bytes.first().copied().unwrap_or(0),
                    (None, None) => return Err(anyhow!("native: the bits")),
                };
                let mut b = b;
                let code = match op.code {
                    BitCode::TestB => "TESTB",
                    BitCode::BitOn => "BITON",
                    _ => "BITOFF",
                };
                let [byte] = b.as_mut_slice() else {
                    return Err(anyhow!("{code}: the result field must be one byte long"));
                };
                match op.code {
                    BitCode::BitOn => *byte |= mask,
                    BitCode::BitOff => *byte &= !mask,
                    _ => {
                        let on = *byte & mask;
                        self.set_result_inds_n(
                            &op.hi,
                            &op.lo,
                            &op.eq,
                            (on == 0, on != 0 && on != mask, on == mask && mask != 0),
                        );
                        return Ok(());
                    }
                }
                rpgrt::native::store(
                    &mut self.m.bufs,
                    slot.buf,
                    slot.off,
                    slot.ty,
                    &Value::Char(b),
                    Fit::Strict { half_adjust: false },
                    false,
                )?;
            }
        }
        Ok(())
    }
}

// ---- fixed-form LOOKUP and XFOOT (S2.85) ------------------------------

impl Rt<'_, '_> {
    /// An array's element k (1-based), as load_var reads it.
    fn arr_elem_n(
        &self,
        arr: &(usize, usize, usize, u32, super::super::data::DType),
        k: usize,
    ) -> NResult<Value> {
        let (buf, off, stride, _, ty) = *arr;
        let at = k
            .checked_sub(1)
            .and_then(|k| k.checked_mul(stride))
            .and_then(|d| off.checked_add(d))
            .ok_or_else(|| anyhow!("native: an element's place"))?;
        rpgrt::native::load(&self.m.bufs, buf, at, ty)
    }

    /// ops.rs lookup for an array (not a table), and array_op's XFOOT.
    fn array_op_n(&mut self, op: &rpgrt::native::ArrOpIn) -> NResult<()> {
        use std::cmp::Ordering;
        let dim = usize::try_from(op.arr.3)?;
        if let Some((slot, half)) = &op.xfoot {
            // XFOOT: the elements' sum, the truncating fit (half adjust
            // with H), the indicators by the value stored.
            let mut sum = Dec::ZERO;
            for k in 1..=dim {
                sum = sum.add(&self.arr_elem_n(&op.arr, k)?.num()?)?;
            }
            rpgrt::native::store(
                &mut self.m.bufs,
                slot.buf,
                slot.off,
                slot.ty,
                &Value::Num(sum),
                Fit::Truncate { half_adjust: *half },
                false,
            )?;
            let stored = rpgrt::native::load(&self.m.bufs, slot.buf, slot.off, slot.ty)?.num()?;
            if op.hi.is_some() || op.lo.is_some() || op.eq.is_some() {
                self.set_result_inds_n(
                    &op.hi,
                    &op.lo,
                    &op.eq,
                    (
                        stored.mantissa() > 0,
                        stored.mantissa() < 0,
                        stored.is_zero(),
                    ),
                );
            }
            return Ok(());
        }
        let Some((arg, index)) = &op.lookup else {
            return Err(anyhow!("native: an array operation"));
        };
        // LOOKUP: from the index's element (RNQ0121 outside the array),
        // the first equal one when EQ is asked for, else the nearest above
        // (HI) or below (LO); the indicators; the index set to the element
        // found, or 1.
        let start = match index {
            Some(ix) => {
                let s = rpgrt::native::load(&self.m.bufs, ix.buf, ix.off, ix.ty)?
                    .num()?
                    .to_i128()?;
                if s < 1 || s > i128::try_from(dim)? {
                    return Err(anyhow!(
                        "RNQ0121: An array index is out of range (index {s}, {dim} elements)."
                    ));
                }
                usize::try_from(s)?
            }
            None => 1,
        };
        let (want_hi, want_lo, want_eq) = (op.hi.is_some(), op.lo.is_some(), op.eq.is_some());
        if !(want_eq || want_hi || want_lo) {
            return Err(anyhow!("LOOKUP needs a resulting indicator"));
        }
        let mut found: Option<(usize, Ordering)> = None;
        let mut best: Option<(usize, Value)> = None;
        for k in start..=dim {
            let v = self.arr_elem_n(&op.arr, k)?;
            let o = data::compare(&v, arg)?;
            if o == Ordering::Equal && want_eq {
                found = Some((k, o));
                break;
            }
            let fits = (o == Ordering::Greater && want_hi) || (o == Ordering::Less && want_lo);
            if fits {
                let closer = match &best {
                    None => true,
                    Some((_, b)) => {
                        let c = data::compare(&v, b)?;
                        if o == Ordering::Greater {
                            c == Ordering::Less
                        } else {
                            c == Ordering::Greater
                        }
                    }
                };
                if closer {
                    best = Some((k, v));
                }
            }
        }
        if found.is_none() {
            if let Some((k, v)) = best {
                found = Some((k, data::compare(&v, arg)?));
            }
        }
        for (i, on) in [
            (&op.hi, matches!(found, Some((_, Ordering::Greater)))),
            (&op.lo, matches!(found, Some((_, Ordering::Less)))),
            (&op.eq, matches!(found, Some((_, Ordering::Equal)))),
        ] {
            if let Some(i) = i {
                self.m.set_ind(i, on);
            }
        }
        if let Some(ix) = index {
            let at = found.map(|(k, _)| k).unwrap_or(1);
            rpgrt::native::store(
                &mut self.m.bufs,
                ix.buf,
                ix.off,
                ix.ty,
                &Value::Num(Dec::from_i64(i64::try_from(at)?)),
                Fit::Strict { half_adjust: false },
                false,
            )?;
        }
        Ok(())
    }
}

// ---- fixed-form SORTA and MOVEA (S2.86) -------------------------------

/// A part of a MOVEA side (S2.86): its bytes, and where they go back (a
/// buffer, an offset and a type), none for a literal.
type MoveaPart = (Vec<u8>, Option<(usize, usize, super::data::DType)>);

impl Rt<'_, '_> {
    /// An element's place (k from 0).
    fn arr_at_n(arr: &rpgrt::native::ArrPlace, k: usize) -> NResult<usize> {
        k.checked_mul(arr.stride)
            .and_then(|d| arr.off.checked_add(d))
            .ok_or_else(|| anyhow!("native: an element's place"))
    }

    /// ops.rs sorta: the elements sorted by data::compare (a stable sort),
    /// reversed for DESCEND; for an array overlaying an array of a data
    /// structure, whole overlaid elements moved by this key.
    fn sorta_n(&mut self, arr: &rpgrt::native::ArrPlace, descend: bool) -> NResult<()> {
        use std::cmp::Ordering;
        let n = usize::try_from(arr.dim)?;
        let mut keyed: Vec<(Value, Vec<u8>)> = Vec::with_capacity(n);
        let whole = arr.stride != arr.ty.size() || arr.lead > 0;
        for k in 0..n {
            let at = Self::arr_at_n(arr, k)?;
            let key = rpgrt::native::load(&self.m.bufs, arr.buf, at, arr.ty)?;
            let bytes = if whole {
                let start = at
                    .checked_sub(arr.lead)
                    .ok_or_else(|| anyhow!("native: an overlaid element"))?;
                let end = start
                    .checked_add(arr.stride)
                    .ok_or_else(|| anyhow!("native: an overlaid element"))?;
                self.m
                    .bufs
                    .get(arr.buf)
                    .and_then(|b| b.get(start..end))
                    .ok_or_else(|| anyhow!("native: an overlaid element"))?
                    .to_vec()
            } else {
                Vec::new()
            };
            keyed.push((key, bytes));
        }
        let mut err = None;
        keyed.sort_by(|x, y| {
            data::compare(&x.0, &y.0).unwrap_or_else(|e| {
                err = Some(e);
                Ordering::Equal
            })
        });
        if let Some(e) = err {
            return Err(e);
        }
        if descend {
            keyed.reverse();
        }
        for (k, (v, bytes)) in keyed.into_iter().enumerate() {
            let at = Self::arr_at_n(arr, k)?;
            if whole {
                let start = at
                    .checked_sub(arr.lead)
                    .ok_or_else(|| anyhow!("native: an overlaid element"))?;
                let end = start
                    .checked_add(arr.stride)
                    .ok_or_else(|| anyhow!("native: an overlaid element"))?;
                self.m
                    .bufs
                    .get_mut(arr.buf)
                    .and_then(|b| b.get_mut(start..end))
                    .ok_or_else(|| anyhow!("native: an overlaid element"))?
                    .copy_from_slice(&bytes);
            } else {
                rpgrt::native::store(
                    &mut self.m.bufs,
                    arr.buf,
                    at,
                    arr.ty,
                    &v,
                    Fit::Strict { half_adjust: false },
                    false,
                )?;
            }
        }
        Ok(())
    }

    /// ops.rs movea_parts for character data: an array's elements from
    /// its index's (RNQ0121 outside the array), or one part; with where
    /// they go.
    fn movea_parts_n(&self, side: &rpgrt::native::MoveaSide) -> NResult<Vec<MoveaPart>> {
        use rpgrt::native::MoveaSide;
        Ok(match side {
            MoveaSide::Array { arr, index } => {
                let dim = usize::try_from(arr.dim)?;
                let first = match index {
                    Some(ix) => {
                        let i = rpgrt::native::load(&self.m.bufs, ix.buf, ix.off, ix.ty)?
                            .num()?
                            .to_i128()?;
                        if i < 1 || i > i128::try_from(dim)? {
                            return Err(anyhow!(
                                "RNQ0121: An array index is out of range (index {i}, {dim} elements)."
                            ));
                        }
                        usize::try_from(i)?
                    }
                    None => 1,
                };
                let mut out = Vec::new();
                for k in first..=dim {
                    let at = Self::arr_at_n(arr, k.saturating_sub(1))?;
                    let b = rpgrt::native::load(&self.m.bufs, arr.buf, at, arr.ty)?.chars()?;
                    out.push((b, Some((arr.buf, at, arr.ty))));
                }
                out
            }
            MoveaSide::Bytes(b) => vec![(b.clone(), None)],
            MoveaSide::Field(f) => {
                let b = rpgrt::native::load(&self.m.bufs, f.buf, f.off, f.ty)?.chars()?;
                vec![(b, Some((f.buf, f.off, f.ty)))]
            }
        })
    }

    /// ops.rs movea for character data: factor 2's bytes over the result's
    /// (an array from its index, or a field), left to right until one runs
    /// out; P blanks the rest first; each part assigned with the
    /// truncating fit.
    fn movea_n(
        &mut self,
        from: &rpgrt::native::MoveaSide,
        to: &rpgrt::native::MoveaSide,
        pad: bool,
    ) -> NResult<()> {
        let src: Vec<u8> = self
            .movea_parts_n(from)?
            .into_iter()
            .flat_map(|(b, _)| b)
            .collect();
        let dst = self.movea_parts_n(to)?;
        let mut flat: Vec<u8> = dst.iter().flat_map(|(b, _)| b.clone()).collect();
        if pad {
            flat.fill(data::BLANK);
        }
        let n = src.len().min(flat.len());
        if let (Some(d), Some(s)) = (flat.get_mut(..n), src.get(..n)) {
            d.copy_from_slice(s);
        }
        let mut at = 0usize;
        for (part, place) in &dst {
            let end = at
                .checked_add(part.len())
                .ok_or_else(|| anyhow!("native: MOVEA's parts"))?;
            let bytes = flat
                .get(at..end)
                .ok_or_else(|| anyhow!("native: MOVEA's parts"))?
                .to_vec();
            at = end;
            let (buf, off, ty) = place.ok_or_else(|| anyhow!("native: MOVEA's result"))?;
            rpgrt::native::store(
                &mut self.m.bufs,
                buf,
                off,
                ty,
                &Value::Char(bytes),
                Fit::Truncate { half_adjust: false },
                false,
            )?;
        }
        Ok(())
    }
}

// ---- procedure pointers (S2.87) ---------------------------------------

/// Where a call through a procedure pointer goes (S2.87).
enum ProcTarget {
    /// Called in another program, module or service program.
    Called(Option<Value>),
    /// A procedure of this activation: its name, and the arguments.
    Here(String, Vec<CallArg>),
}

impl Rt<'_, '_> {
    /// interp.rs bif's PADDR arm, after its name: where the procedure is
    /// (this module: its service program, with the module when it is not
    /// exported; a module bound into a program: *MODULE; a program's
    /// entry module: *PGM, the program and its activation group; else the
    /// service program or module the binder found), as a procedure
    /// pointer.
    fn paddr_n(&mut self, n: &str) -> NResult<Value> {
        let bound_module = self
            .m
            .io
            .program
            .strip_prefix("*BOUND/")
            .map(str::to_string);
        let home = if let Some(pi) = self.m.prog.procs.get(n) {
            let exported = pi.export;
            match self.m.current_srvpgm() {
                Some((l, sp)) => Some(if exported {
                    (l, sp)
                } else {
                    (l, format!("{sp}/{}", self.m.prog.name))
                }),
                None => match bound_module {
                    Some(m) => Some(("*MODULE".to_string(), m)),
                    None if self.m.io.program.contains('/')
                        && !self.m.io.program.starts_with('*') =>
                    {
                        Some((
                            "*PGM".to_string(),
                            format!("{} {}", self.m.io.program, self.m.io.actgrp),
                        ))
                    }
                    None => None,
                },
            }
        } else {
            match self.m.io.binding.as_ref().and_then(|b| b.imports.get(n)) {
                Some(crate::ile::Target::SrvPgm { library, name }) => {
                    Some((library.clone(), name.clone()))
                }
                Some(crate::ile::Target::Module(m)) => Some(("*MODULE".to_string(), m.clone())),
                _ => None,
            }
        };
        Ok(Value::Ptr(super::super::mem::Ptr {
            kind: super::super::mem::KIND_PROC,
            owner: 0,
            buf: super::super::mem::proc_id(n, home),
            off: 0,
        }))
    }

    /// interp.rs call_named_inner for EXTPROC(procedure pointer): the
    /// pointer field's value (MCH3601 when it is not a valid procedure
    /// pointer or not set), the procedure and where it is; one of another
    /// program (*PGM), module (*MODULE) or service program is called there
    /// with the arguments as args_for gives them; one of this activation
    /// goes on as a call by its name.
    fn procptr_target_n(
        &mut self,
        name: &str,
        var: &str,
        args: Vec<CallArg>,
    ) -> NResult<ProcTarget> {
        let v = match self.m.sym(var) {
            Some(super::super::program::Sym::Var(vi)) => {
                rpgrt::native::load(&self.m.bufs, vi.buf, vi.off, vi.dtype)?
            }
            _ => return Err(anyhow!("native: the procedure pointer {var}")),
        };
        let (pname, home) = match v {
            Value::Ptr(p) if p.kind == super::super::mem::KIND_PROC => {
                super::super::mem::proc_ref(p.buf)
                    .ok_or_else(|| anyhow!("MCH3601: procedure pointer {var} is not valid."))?
            }
            Value::Ptr(_) => return Err(anyhow!("MCH3601: {var} is not a procedure pointer.")),
            _ => {
                return Err(anyhow!(
                    "MCH3601: procedure pointer {var} is not set (*NULL)."
                ))
            }
        };
        let as_args = |args: Vec<CallArg>| -> Vec<super::Arg> {
            args.into_iter()
                .map(|arg| match arg {
                    CallArg::Ref { buf, off, dtype } => super::Arg::Ref(super::Loc {
                        buf,
                        off,
                        size: dtype.size(),
                        dtype,
                    }),
                    CallArg::Val(v) => super::Arg::Val(v),
                })
                .collect()
        };
        let _ = name;
        let prog = self.m.prog;
        match home {
            Some((l, spec)) if l == "*PGM" => {
                let (pgm, group) = spec.split_once(' ').unwrap_or((spec.as_str(), ""));
                let here = self.m.io.program == pgm && prog.procs.contains_key(&pname);
                if !here {
                    let (library, pn) = pgm.split_once('/').unwrap_or(("", pgm));
                    let r = self.m.call_bound_in(
                        &pname,
                        &crate::ile::Target::Pgm {
                            library: library.to_string(),
                            name: pn.to_string(),
                            group: group.to_string(),
                        },
                        Some(pn),
                        as_args(args),
                    )?;
                    return Ok(ProcTarget::Called(r));
                }
            }
            Some((l, m)) if l == "*MODULE" => {
                let here =
                    self.m.io.program == format!("*BOUND/{m}") && prog.procs.contains_key(&pname);
                if !here {
                    let r = self.m.call_bound_in(
                        &pname,
                        &crate::ile::Target::Module(m.clone()),
                        Some(&m),
                        as_args(args),
                    )?;
                    return Ok(ProcTarget::Called(r));
                }
            }
            Some((library, sp)) => {
                let (sp, module) = match sp.split_once('/') {
                    Some((s, m)) => (s.to_string(), Some(m.to_string())),
                    None => (sp, None),
                };
                let here = self.m.current_srvpgm() == Some((library.clone(), sp.clone()))
                    && module.as_ref().is_none_or(|m| *m == prog.name)
                    && prog.procs.contains_key(&pname);
                if !here {
                    let r = self.m.call_bound_in(
                        &pname,
                        &crate::ile::Target::SrvPgm { library, name: sp },
                        module.as_deref(),
                        as_args(args),
                    )?;
                    return Ok(ProcTarget::Called(r));
                }
            }
            None => {}
        }
        Ok(ProcTarget::Here(pname, args))
    }
}

// ---- display files (S2.114) ------------------------------------------
//
// Written again from errors.rs file_op around interp.rs workstn_op and
// its ws_values, ws_fill, ws_inds, ws_set_ind, ws_interact and
// message_subfile, in their order; native_tripwire.rs watches them. The
// program fields of the file's formats come as places (OutSlot), as for
// printer files (S2.79); the display itself is workstn.rs's and the
// job's display channel, as the interpreter's.
impl Rt<'_, '_> {
    fn workstn_n(
        &mut self,
        pc: usize,
        fields: &[rpgrt::native::OutSlot],
        indds: Option<&rpgrt::native::OutSlot>,
        key: Option<&rpgrt::native::OutSlot>,
    ) -> NResult<()> {
        let (op, file, _, _, ind) = self.file_stmt(pc)?;
        let file = file.to_string();
        let ind = ind.clone();
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
        let r = self.workstn_inner_n(op, i, &file, key, &ind, fields, indds);
        self.file_end_n(i, op, &file, cmt.as_deref(), r.as_ref().map(|_| ()))?;
        r
    }

    fn ws_slot<'f>(
        fields: &'f [rpgrt::native::OutSlot],
        name: &str,
    ) -> NResult<&'f rpgrt::native::OutSlot> {
        fields
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| anyhow!("native: display field {name} has no place"))
    }

    /// interp.rs workstn_op, for the operations lowering takes.
    #[allow(clippy::too_many_arguments)] // workstn_op's own, and the places
    fn workstn_inner_n(
        &mut self,
        op: FileOp,
        i: usize,
        name: &str,
        key: Option<&rpgrt::native::OutSlot>,
        ind: &super::super::ast::ResultInd,
        fields: &[rpgrt::native::OutSlot],
        indds: Option<&rpgrt::native::OutSlot>,
    ) -> NResult<()> {
        let prog: &super::super::program::Program = self.m.prog;
        let fi = prog
            .files
            .get(i)
            .ok_or_else(|| anyhow!("native: no file {i}"))?;
        let d = fi
            .display
            .as_ref()
            .ok_or_else(|| anyhow!("{name} is not a display file"))?;
        if !self.file_n(i)?.ws_open {
            anyhow::bail!("CPF4135: File {} is not open.", fi.decl.name);
        }
        if op == FileOp::Read {
            let named = d.fields(name).is_some().then_some(name);
            let file = d.file.clone();
            let (frame, format) = self.m.ws.read_frame(&file, named)?;
            self.ws_interact_n(i, d, &file, &format, frame, fields, indds)?;
            let st = super::Status {
                found: true,
                ..super::Status::default()
            };
            self.file_n(i)?.status = st.clone();
            self.m.last = st;
            if let Some(eq) = &ind.eq {
                self.m.set_ind(eq, false);
            }
            return Ok(());
        }
        if d.fields(name).is_none() {
            anyhow::bail!(
                "{op:?} {name}: name a record format of display file {}",
                d.file.name
            );
        }
        let format = name.to_string();
        let file = d.file.clone();
        let inds = self.ws_inds_n(d, indds);
        match op {
            FileOp::Write => {
                let values = self.ws_values_n(d, &format, fields)?;
                match d.sfile.get(&format) {
                    Some(rrn_field) => {
                        let slot = Self::ws_slot(fields, rrn_field)?;
                        let rrn = rpgrt::native::load(&self.m.bufs, slot.buf, slot.off, slot.ty)?
                            .num()?
                            .to_i128()?;
                        let rrn = i64::try_from(rrn).unwrap_or(-1);
                        self.m.ws.sfl_write(&file, &format, rrn, values, inds)?;
                    }
                    None => {
                        let values = self.message_subfile_n(&file, &format, values, &inds);
                        self.m.ws.write(&file, &format, values, inds)?
                    }
                }
                self.ws_status_n(i, super::Status::default())?;
            }
            FileOp::Update => {
                let values = self.ws_values_n(d, &format, fields)?;
                self.m.ws.sfl_update(&file, &format, values, inds)?;
                self.ws_status_n(i, super::Status::default())?;
            }
            FileOp::Exfmt => {
                if self.m.io.display.is_none() {
                    anyhow::bail!(
                        "Display file {} cannot be used here: this job has no display (it is not interactive).",
                        d.file.name
                    );
                }
                let values = self.ws_values_n(d, &format, fields)?;
                let frame = self.m.ws.frame(&file, &format, values, inds)?;
                self.ws_interact_n(i, d, &file, &format, frame, fields, indds)?;
                self.ws_status_n(i, super::Status::default())?;
            }
            FileOp::ReadC => {
                let r = self.m.ws.sfl_readc(&file, &format)?;
                let found = r.is_some();
                if let Some((rrn, values)) = r {
                    self.ws_fill_n(d, &format, &values, fields)?;
                    if let Some(f) = d.sfile.get(&format) {
                        let slot = Self::ws_slot(fields, f)?;
                        rpgrt::native::store(
                            &mut self.m.bufs,
                            slot.buf,
                            slot.off,
                            slot.ty,
                            &Value::Num(Dec::from_i64(i64::from(rrn))),
                            Fit::Strict { half_adjust: false },
                            false,
                        )?;
                    }
                }
                self.ws_status_n(
                    i,
                    super::Status {
                        found,
                        eof: !found,
                        ..super::Status::default()
                    },
                )?;
                if let Some(eq) = &ind.eq {
                    self.m.set_ind(eq, !found);
                }
            }
            FileOp::Chain => {
                let k = key.ok_or_else(|| {
                    anyhow!("CHAIN {format}: a relative record number is needed")
                })?;
                let rrn = rpgrt::native::load(&self.m.bufs, k.buf, k.off, k.ty)?
                    .num()?
                    .to_i128()?;
                let r = self
                    .m
                    .ws
                    .sfl_chain(&file, &format, i64::try_from(rrn).unwrap_or(0))?;
                let found = r.is_some();
                if let Some(values) = r {
                    self.ws_fill_n(d, &format, &values, fields)?;
                }
                self.ws_status_n(
                    i,
                    super::Status {
                        found,
                        ..super::Status::default()
                    },
                )?;
                if let Some(hi) = &ind.hi {
                    self.m.set_ind(hi, !found);
                }
            }
            other => anyhow::bail!(
                "{other:?} of display file record {format} is not supported by the native engine yet"
            ),
        }
        Ok(())
    }

    /// workstn_op's set_status: the file's status and the last one.
    fn ws_status_n(&mut self, i: usize, st: super::Status) -> NResult<()> {
        self.file_n(i)?.status = st.clone();
        self.m.last = st;
        Ok(())
    }

    /// interp.rs ws_inds: indicators 01-99 from the INDDS data
    /// structure's bytes, or *IN01-*IN99.
    fn ws_inds_n(
        &self,
        d: &super::super::program::Display,
        indds: Option<&rpgrt::native::OutSlot>,
    ) -> [bool; 100] {
        let mut out = [false; 100];
        match (d.indds.as_ref(), indds) {
            (Some(_), Some(slot)) => {
                let size = slot.ty.size();
                let bytes = self
                    .m
                    .bufs
                    .get(slot.buf)
                    .and_then(|b| b.get(slot.off..slot.off.saturating_add(size)))
                    .unwrap_or(&[]);
                for (n, v) in out.iter_mut().enumerate().skip(1) {
                    *v = n.checked_sub(1).and_then(|k| bytes.get(k)) == Some(&0xF1);
                }
            }
            _ => {
                for (n, v) in out.iter_mut().enumerate().skip(1) {
                    *v = self.m.ind(&format!("{n:02}"));
                }
            }
        }
        out
    }

    /// interp.rs ws_set_ind.
    fn ws_set_ind_n(
        &mut self,
        d: &super::super::program::Display,
        indds: Option<&rpgrt::native::OutSlot>,
        n: u8,
        v: bool,
    ) {
        if !(1..=99).contains(&n) {
            return;
        }
        match (d.indds.as_ref(), indds) {
            (Some(_), Some(slot)) => {
                let size = slot.ty.size();
                let k = usize::from(n).saturating_sub(1);
                if k < size {
                    if let Some(b) = self
                        .m
                        .bufs
                        .get_mut(slot.buf)
                        .and_then(|b| b.get_mut(slot.off.saturating_add(k)))
                    {
                        *b = if v { 0xF1 } else { 0xF0 };
                    }
                }
            }
            _ => self.m.set_ind(&format!("{n:02}"), v),
        }
    }

    /// interp.rs ws_interact: the frame shown on the job's display, the
    /// operator's input back into the fields, the response indicators,
    /// *INKA-*INKY.
    #[allow(clippy::too_many_arguments)] // ws_interact's own, and the places
    fn ws_interact_n(
        &mut self,
        i: usize,
        d: &super::super::program::Display,
        file: &std::sync::Arc<crate::dspf::DisplayFile>,
        format: &str,
        frame: crate::workstn::Frame,
        fields: &[rpgrt::native::OutSlot],
        indds: Option<&rpgrt::native::OutSlot>,
    ) -> NResult<()> {
        let Some(display) = self.m.io.display.clone() else {
            anyhow::bail!(
                "Display file {} cannot be used here: this job has no display (it is not interactive).",
                d.file.name
            );
        };
        let (tx, rx) = tokio::sync::oneshot::channel();
        let resp = self.m.block(async move {
            display
                .send(crate::runtime::DisplayRequest { frame, reply: tx })
                .await
                .map_err(|_| anyhow!("the job's display is gone"))?;
            rx.await
                .map_err(|_| anyhow!("the job's display is gone"))?
                .map_err(|e| anyhow!(e))
        })?;
        self.m.ws.apply(file, format, &resp);
        self.file_n(i)?.aid = Some(resp.aid.clone());
        self.ws_fill_n(d, format, &resp.values, fields)?;
        for (n, v) in &resp.indicators {
            self.ws_set_ind_n(d, indds, *n, *v);
        }
        for (k, letter) in super::FKEYS.iter().enumerate() {
            let on = resp.aid == format!("F{}", k.saturating_add(1));
            self.m.set_ind(&format!("K{}", *letter as char), on);
        }
        Ok(())
    }

    /// interp.rs message_subfile: a message subfile's control record
    /// with SFLINZ shows the messages of its SFLPGMQ program queue.
    fn message_subfile_n(
        &self,
        file: &crate::dspf::DisplayFile,
        format: &str,
        mut values: std::collections::BTreeMap<String, String>,
        inds: &[bool; 100],
    ) -> std::collections::BTreeMap<String, String> {
        use crate::dspf::Keyword;
        let Some(ctl) = file.record(format) else {
            return values;
        };
        let Some(sfl) = ctl.subfile_control().and_then(|s| file.record(s)) else {
            return values;
        };
        let Some(line) = sfl.message_line() else {
            return values;
        };
        if !ctl.active(inds, |k| *k == Keyword::SflInz) {
            return values;
        }
        let queue = ctl
            .pgmq_field()
            .and_then(|f| values.get(&f.name))
            .map(|q| q.trim().to_string())
            .unwrap_or_default();
        let queue = if queue.is_empty() || queue == "*" {
            self.m.prog.name.clone()
        } else {
            queue
        };
        let msgs = crate::pgmmsg::named_messages(&self.m.io.job_key, &queue);
        let pag = usize::from(ctl.sflpag());
        values.insert(crate::dspf::MSG_LINE.into(), line.to_string());
        for (k, m) in msgs.iter().take(pag).enumerate() {
            values.insert(format!("{}{k}", crate::dspf::MSG_TEXT), m.text.clone());
        }
        if msgs.len() > pag {
            values.insert(crate::dspf::MSG_MORE.into(), "1".into());
        }
        values
    }

    /// interp.rs ws_values: the format's program fields as display text
    /// by display field name.
    fn ws_values_n(
        &mut self,
        d: &super::super::program::Display,
        format: &str,
        fields: &[rpgrt::native::OutSlot],
    ) -> NResult<std::collections::BTreeMap<String, String>> {
        let rec = d.file.record(format);
        let mut out = std::collections::BTreeMap::new();
        for (pname, dname) in d.fields(format).unwrap_or_default() {
            let decimals = rec
                .and_then(|r| r.fields.iter().find(|f| &f.name == dname))
                .and_then(|f| f.decimals);
            let slot = Self::ws_slot(fields, pname)?;
            let v = rpgrt::native::load(&self.m.bufs, slot.buf, slot.off, slot.ty)?;
            let text = match (v, decimals) {
                (Value::Num(n), Some(dec)) => n.rescale(u8::try_from(dec)?, false)?.to_string(),
                (Value::Num(n), None) => n.to_string(),
                (other, _) => a(&other.chars()?),
            };
            out.insert(dname.clone(), text);
        }
        Ok(out)
    }

    /// interp.rs ws_fill: display values into the format's program fields.
    fn ws_fill_n(
        &mut self,
        d: &super::super::program::Display,
        format: &str,
        values: &std::collections::BTreeMap<String, String>,
        fields: &[rpgrt::native::OutSlot],
    ) -> NResult<()> {
        let rec = d.file.record(format);
        for (pname, dname) in d.fields(format).unwrap_or_default() {
            let Some(text) = values.get(dname) else {
                continue;
            };
            let numeric = rec
                .and_then(|r| r.fields.iter().find(|f| &f.name == dname))
                .is_some_and(|f| f.is_numeric());
            let v = if numeric {
                Value::Num(Dec::parse(text.trim())?)
            } else {
                Value::str(text)
            };
            let slot = Self::ws_slot(fields, pname)?;
            rpgrt::native::store(
                &mut self.m.bufs,
                slot.buf,
                slot.off,
                slot.ty,
                &v,
                Fit::Truncate { half_adjust: false },
                false,
            )?;
        }
        Ok(())
    }
}
