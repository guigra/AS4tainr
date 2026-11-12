//! as4tainr: the RPG code generator (docs/native/adr/0001-0003).
//!
//! In: a compiled routine as [`ir::Routine`] (as4tainr-core lowers each
//! `code::Code` of a checked program to it). Out: the Rust source of one
//! generated routine, a function `run(rt, start)` over
//! `rpgrt::native::Runtime`, and its source map. Generation is pure and
//! deterministic: the same routine gives byte-identical source.
//!
//! Stage 1 (ADR 0001): the routine's control flow is compiled to a
//! `loop { match pc { … } }` state machine; statements, the expressions
//! instructions evaluate, conditioning indicators and error routing are
//! calls into the runtime.

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

use std::fmt::Write as _;

/// This generator's version: part of every routine's fingerprint
/// (docs/native/adr/0002). Change it whenever generated code changes.
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+stage2.2");

/// The most instructions one routine may have: far above any program, a
/// bound on generation.
pub const MAX_INSTRUCTIONS: usize = 1_000_000;

pub mod ir {
    //! The generator's input: a compiled routine's instructions, as much
    //! of them as the generated code depends on.

    /// One compiled routine.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Routine {
        /// SHA-256 of the compiled routine (64 hexadecimal digits).
        pub fingerprint: String,
        pub ins: Vec<Ins>,
    }

    /// One instruction.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Ins {
        pub kind: Kind,
        /// Its RPG source line (0: none).
        pub line: usize,
        /// It has conditioning indicators.
        pub cond: bool,
        /// What it is, for the comment (an operation code); any text.
        pub what: String,
        /// Stage 2 (docs/native/adr/0006): what the instruction evaluates,
        /// lowered to direct Rust; None: the runtime evaluates it.
        pub lowered: Option<Lowered>,
    }

    /// A numeric type lowered code reads and writes (the `DType`s of
    /// rpgrt::rpg::data with these names).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Ty {
        Packed(u32, u32),
        Zoned(u32, u32),
        Binary(u32, u32),
        Int(u32),
        Uns(u32),
    }

    /// An operand of lowered code.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Operand {
        /// A field: its buffer, offset and type (and name, for comments).
        Var {
            name: String,
            buf: usize,
            off: usize,
            ty: Ty,
        },
        /// A numeric literal: mantissa and scale (`Dec::new`).
        Num { m: i128, s: u8 },
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CmpOp {
        Eq,
        Ne,
        Lt,
        Le,
        Gt,
        Ge,
    }

    /// A lowered condition.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Cond {
        Cmp(CmpOp, Operand, Operand),
        And(Box<Cond>, Box<Cond>),
        Or(Box<Cond>, Box<Cond>),
    }

    /// What a lowered instruction evaluates.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Lowered {
        /// The condition of a jump-if-not.
        Cond(Cond),
        /// FOR/DO: the index past its limit.
        LoopTest {
            var: Operand,
            limit: Option<Operand>,
            down: bool,
        },
        /// FOR/DO: step the index.
        LoopStep {
            var: Operand,
            by: Option<Operand>,
            down: bool,
        },
        /// EVAL (S2.2): a numeric field gets a numeric expression.
        Eval {
            target: Operand,
            value: NumExpr,
            half_adjust: bool,
            right: bool,
        },
    }

    /// +, - and * in lowered code.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ArOp {
        Add,
        Sub,
        Mul,
    }

    /// A numeric expression of lowered code.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum NumExpr {
        Leaf(Operand),
        /// An operation, and its place in the statement's expression (0
        /// left, 1 right from the root): where the runtime finds it to give
        /// its precision.
        Bin {
            op: ArOp,
            left: Box<NumExpr>,
            right: Box<NumExpr>,
            path: Vec<u8>,
        },
    }

    /// The instruction kinds of `code::Ins`, with the targets the
    /// generated code jumps to. An index equal to the routine's length
    /// ends the routine.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Kind {
        Stmt,
        Jump(usize),
        /// Jump when the condition is false.
        JumpIfNot(usize),
        /// FOR/DO: past the limit, go to `end`.
        LoopTest {
            end: usize,
        },
        LoopStep,
        EachInit,
        /// FOR-EACH: no next value, go to `end`.
        EachNext {
            end: usize,
        },
        ExSr,
        Return,
        LeaveSr,
        /// CABxx: `to` is its TAG in this routine (None: none, or a TAG
        /// outside the routine, which the runtime signals).
        Cab {
            to: Option<usize>,
        },
        Label,
        GotoOut,
        EndSr,
        Monitor {
            end_body: usize,
            end: usize,
        },
        EndMonitor {
            end: usize,
        },
        Nop,
    }
}

/// Why a routine cannot be generated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The fingerprint is not 64 lower-case hexadecimal digits.
    Fingerprint(String),
    TooLong(usize),
    /// A jump target past the end of the routine (at instruction, line).
    Target {
        at: usize,
        line: usize,
        to: usize,
    },
    /// A lowered evaluation that does not fit its instruction's kind.
    Lowered {
        at: usize,
        line: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Fingerprint(fp) => write!(f, "not a routine fingerprint: {fp:?}"),
            Error::TooLong(n) => write!(
                f,
                "a routine of {n} instructions (at most {MAX_INSTRUCTIONS})"
            ),
            Error::Target { at, line, to } => write!(
                f,
                "instruction {at} (line {line}) jumps to {to}, past the end of the routine"
            ),
            Error::Lowered { at, line } => write!(
                f,
                "instruction {at} (line {line}): its lowered form is not for its kind"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A generated routine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generated {
    /// The Rust source of the module `<fingerprint>.rs`.
    pub source: String,
    /// Its source map, as JSON: each instruction's index, source line and
    /// kind, and the line of the generated source its arm starts at.
    pub map: String,
}

fn valid_fingerprint(fp: &str) -> bool {
    fp.len() == 64 && fp.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Text that is safe inside a `//` comment: one line of printable
/// ASCII (no control or bidirectional characters, which rustc rejects).
fn comment(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ' '..='~' => c,
            c if c.is_control() => ' ',
            _ => '?',
        })
        .take(80)
        .collect()
}

/// The JSON string for a text.
fn json_str(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn kind_name(k: &ir::Kind) -> &'static str {
    use ir::Kind::*;
    match k {
        Stmt => "stmt",
        Jump(_) => "jump",
        JumpIfNot(_) => "jump-if-not",
        LoopTest { .. } => "loop-test",
        LoopStep => "loop-step",
        EachInit => "each-init",
        EachNext { .. } => "each-next",
        ExSr => "exsr",
        Return => "return",
        LeaveSr => "leavesr",
        Cab { .. } => "cab",
        Label => "tag",
        GotoOut => "goto-out",
        EndSr => "endsr",
        Monitor { .. } => "monitor",
        EndMonitor { .. } => "end-monitor",
        Nop => "nop",
    }
}

/// The expression giving the instruction's result (`NResult<usize>`: the
/// next instruction), `next` being the one after it.
fn step(pc: usize, next: usize, kind: &ir::Kind) -> String {
    use ir::Kind::*;
    match kind {
        Stmt => format!("rt.stmt({pc}).map(|()| {next})"),
        Jump(t) => format!("Ok({t})"),
        JumpIfNot(t) => format!("rt.truth({pc}).map(|c| if c {{ {next} }} else {{ {t} }})"),
        LoopTest { end } => {
            format!("rt.loop_past({pc}).map(|past| if past {{ {end} }} else {{ {next} }})")
        }
        LoopStep => format!("rt.loop_step({pc}).map(|()| {next})"),
        EachInit => format!("rt.each_init({pc}).map(|()| {next})"),
        EachNext { end } => {
            format!("rt.each_next({pc}).map(|got| if got {{ {next} }} else {{ {end} }})")
        }
        ExSr => format!("rt.exsr({pc}).map(|()| {next})"),
        Return => format!("rt.ret({pc}).map(|()| {next})"),
        LeaveSr => format!("rt.leave_sr({pc}).map(|()| {next})"),
        Cab { to: Some(t) } => {
            format!("rt.cab({pc}).map(|branch| if branch {{ {t} }} else {{ {next} }})")
        }
        Cab { to: None } => format!("rt.cab({pc}).map(|_| {next})"),
        Label | Nop => format!("Ok({next})"),
        GotoOut => format!("rt.goto_out({pc}).map(|()| {next})"),
        EndSr => format!("rt.end_sr({pc}).map(|()| {next})"),
        Monitor { end_body, end } => format!(
            "{{\n                monitors.push(rpgrt::native::MonitorRef {{ at: {pc}, end_body: {end_body}, end: {end} }});\n                Ok({next})\n            }}"
        ),
        EndMonitor { end } => {
            format!("{{\n                monitors.pop();\n                Ok({end})\n            }}")
        }
    }
}

fn targets(kind: &ir::Kind) -> Vec<usize> {
    use ir::Kind::*;
    match kind {
        Jump(t) | JumpIfNot(t) => vec![*t],
        LoopTest { end } | EachNext { end } | EndMonitor { end } => vec![*end],
        Cab { to } => to.iter().copied().collect(),
        Monitor { end_body, end } => vec![*end_body, *end],
        Stmt | LoopStep | EachInit | ExSr | Return | LeaveSr | Label | GotoOut | EndSr | Nop => {
            vec![]
        }
    }
}

fn ty(t: &ir::Ty) -> String {
    use ir::Ty::*;
    let p = "rpgrt::rpg::data::DType";
    match t {
        Packed(d, s) => format!("{p}::Packed({d}, {s})"),
        Zoned(d, s) => format!("{p}::Zoned({d}, {s})"),
        Binary(d, s) => format!("{p}::Binary({d}, {s})"),
        Int(n) => format!("{p}::Int({n})"),
        Uns(n) => format!("{p}::Uns({n})"),
    }
}

/// An operand as a `Value` expression (inside a helper with `rt`).
fn value(o: &ir::Operand) -> String {
    match o {
        ir::Operand::Var {
            buf, off, ty: t, ..
        } => {
            format!("rpgrt::native::load(rt.mem(), {buf}, {off}, {})?", ty(t))
        }
        ir::Operand::Num { m, s } => {
            format!("rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::new({m}, {s}))")
        }
    }
}

/// An operand as a `Dec` expression.
fn number(o: &ir::Operand) -> String {
    match o {
        ir::Operand::Var {
            buf, off, ty: t, ..
        } => {
            format!(
                "rpgrt::native::load_num(rt.mem(), {buf}, {off}, {})?",
                ty(t)
            )
        }
        ir::Operand::Num { m, s } => format!("rpgrt::rpg::dec::Dec::new({m}, {s})"),
    }
}

fn name_of(o: &ir::Operand) -> String {
    match o {
        ir::Operand::Var { name, .. } => comment(name),
        ir::Operand::Num { m, s } => format!("{m}e-{s}"),
    }
}

/// Helper functions of a lowered condition: `c<pc>` for the whole, then
/// `c<pc>_<k>` for its parts.
fn cond_fns(pc: usize, c: &ir::Cond, id: &str, out: &mut String, n: &mut usize) {
    let _ = writeln!(out, "\nfn {id}(rt: &mut dyn Runtime) -> NResult<bool> {{");
    match c {
        ir::Cond::Cmp(op, a, b) => {
            let op = match op {
                ir::CmpOp::Eq => "Eq",
                ir::CmpOp::Ne => "Ne",
                ir::CmpOp::Lt => "Lt",
                ir::CmpOp::Le => "Le",
                ir::CmpOp::Gt => "Gt",
                ir::CmpOp::Ge => "Ge",
            };
            let _ = writeln!(out, "    // {} {op} {}", name_of(a), name_of(b));
            let _ = writeln!(out, "    let x = {};", value(a));
            let _ = writeln!(out, "    let y = {};", value(b));
            let _ = writeln!(
                out,
                "    rpgrt::native::cmp(rpgrt::native::CmpOp::{op}, &x, &y)\n}}"
            );
        }
        ir::Cond::And(l, r) | ir::Cond::Or(l, r) => {
            let both = if matches!(c, ir::Cond::And(..)) {
                "&&"
            } else {
                "||"
            };
            *n = n.saturating_add(1);
            let li = format!("c{pc}_{n}");
            *n = n.saturating_add(1);
            let ri = format!("c{pc}_{n}");
            let _ = writeln!(out, "    Ok({li}(rt)? {both} {ri}(rt)?)\n}}");
            cond_fns(pc, l, &li, out, n);
            cond_fns(pc, r, &ri, out, n);
        }
    }
}

/// The arm of an instruction whose evaluation is lowered, and its
/// helpers: the lowered code runs unless a procedure is running.
fn lowered_step(
    pc: usize,
    next: usize,
    i: &ir::Ins,
    l: &ir::Lowered,
    helpers: &mut String,
) -> Result<String, Error> {
    let bad = || Error::Lowered {
        at: pc,
        line: i.line,
    };
    let guarded = |interp: String, native: String| {
        format!(
            "if rt.in_procedure() {{\n                    {interp}\n                }} else {{\n                    {native}\n                }}"
        )
    };
    Ok(match (&i.kind, l) {
        (ir::Kind::JumpIfNot(t), ir::Lowered::Cond(c)) => {
            let mut n = 0;
            cond_fns(pc, c, &format!("c{pc}"), helpers, &mut n);
            guarded(
                format!("rt.truth({pc}).map(|c| if c {{ {next} }} else {{ {t} }})"),
                format!("c{pc}(rt).map(|c| if c {{ {next} }} else {{ {t} }})"),
            )
        }
        (ir::Kind::LoopTest { end }, ir::Lowered::LoopTest { var, limit, down }) => {
            let _ = writeln!(
                helpers,
                "\nfn lt{pc}(rt: &mut dyn Runtime) -> NResult<bool> {{"
            );
            let _ = writeln!(helpers, "    let v = {};", number(var));
            match limit {
                Some(lim) => {
                    let _ = writeln!(helpers, "    let l = {};", number(lim));
                    let _ = writeln!(helpers, "    Ok(v {} l)\n}}", if *down { "<" } else { ">" });
                }
                None => {
                    let _ = writeln!(helpers, "    let _ = v;\n    Ok(false)\n}}");
                }
            }
            guarded(
                format!("rt.loop_past({pc}).map(|past| if past {{ {end} }} else {{ {next} }})"),
                format!("lt{pc}(rt).map(|past| if past {{ {end} }} else {{ {next} }})"),
            )
        }
        (ir::Kind::LoopStep, ir::Lowered::LoopStep { var, by, down }) => {
            let ir::Operand::Var {
                buf, off, ty: t, ..
            } = var
            else {
                return Err(bad());
            };
            let _ = writeln!(
                helpers,
                "\nfn ls{pc}(rt: &mut dyn Runtime) -> NResult<()> {{"
            );
            let step = by
                .as_ref()
                .map(number)
                .unwrap_or_else(|| "rpgrt::rpg::dec::Dec::from_i64(1)".to_string());
            let _ = writeln!(helpers, "    let step = {step};");
            let _ = writeln!(helpers, "    let v = {};", number(var));
            let _ = writeln!(
                helpers,
                "    let n = v.{}(&step)?;",
                if *down { "sub" } else { "add" }
            );
            let _ = writeln!(
                helpers,
                "    rpgrt::native::store(\n        rt.mem(),\n        {buf},\n        {off},\n        {},\n        &rpgrt::rpg::data::Value::Num(n),\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: false }},\n        false,\n    )\n}}",
                ty(t)
            );
            guarded(
                format!("rt.loop_step({pc}).map(|()| {next})"),
                format!("ls{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::Eval {
                target,
                value,
                half_adjust,
                right,
            },
        ) => {
            let ir::Operand::Var {
                buf, off, ty: t, ..
            } = target
            else {
                return Err(bad());
            };
            // The expression's helpers first, then this one whole.
            let v = num_expr(pc, value, helpers);
            let _ = writeln!(
                helpers,
                "\nfn ev{pc}(rt: &mut dyn Runtime) -> NResult<()> {{"
            );
            let _ = writeln!(helpers, "    let v = {v};");
            let _ = writeln!(
                helpers,
                "    rpgrt::native::store(\n        rt.mem(),\n        {buf},\n        {off},\n        {},\n        &rpgrt::rpg::data::Value::Num(v),\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: {half_adjust} }},\n        {right},\n    )\n}}",
                ty(t)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ev{pc}(rt).map(|()| {next})"),
            )
        }
        _ => return Err(bad()),
    })
}

/// A numeric expression as a `Dec` expression; each operation becomes a
/// helper `e<pc>_<path>` that keeps its precision in a `OnceLock` (the
/// precision depends only on the operands' types, which the fingerprint
/// covers).
fn num_expr(pc: usize, e: &ir::NumExpr, helpers: &mut String) -> String {
    match e {
        ir::NumExpr::Leaf(o) => number(o),
        ir::NumExpr::Bin {
            op,
            left,
            right,
            path,
        } => {
            let id: String = path
                .iter()
                .map(|b| if *b == 0 { 'l' } else { 'r' })
                .collect();
            let name = format!("e{pc}_{id}");
            let path_lit: Vec<String> = path.iter().map(|b| b.to_string()).collect();
            let mut body = String::new();
            let _ = writeln!(
                body,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{"
            );
            let a = num_expr(pc, left, helpers);
            let b = num_expr(pc, right, helpers);
            let _ = writeln!(body, "    let a = {a};");
            let _ = writeln!(body, "    let b = {b};");
            let _ = writeln!(
                body,
                "    static P: std::sync::OnceLock<Option<(u32, u32)>> = std::sync::OnceLock::new();"
            );
            let _ = writeln!(
                body,
                "    let p = *P.get_or_init(|| rt.precision({pc}, &[{}]));",
                path_lit.join(", ")
            );
            let f = match op {
                ir::ArOp::Add => "add",
                ir::ArOp::Sub => "sub",
                ir::ArOp::Mul => "mul",
            };
            let _ = writeln!(body, "    Ok(rpgrt::native::rule_scale(p, a.{f}(&b)?))\n}}");
            helpers.push_str(&body);
            format!("{name}(rt)?")
        }
    }
}

/// Generate a routine.
pub fn generate(r: &ir::Routine) -> Result<Generated, Error> {
    if !valid_fingerprint(&r.fingerprint) {
        return Err(Error::Fingerprint(r.fingerprint.clone()));
    }
    let len = r.ins.len();
    if len > MAX_INSTRUCTIONS {
        return Err(Error::TooLong(len));
    }
    for (at, i) in r.ins.iter().enumerate() {
        if let Some(&to) = targets(&i.kind).iter().find(|&&t| t > len) {
            return Err(Error::Target {
                at,
                line: i.line,
                to,
            });
        }
    }
    let mut map = String::new();
    let _ = write!(
        map,
        "{{\"generator\":{},\"fingerprint\":{},\"instructions\":[",
        json_str(VERSION),
        json_str(&r.fingerprint)
    );
    if r.ins.is_empty() {
        // An empty routine ends at once, wherever it starts.
        let source = format!(
            "// Generated by rpgcg {VERSION}. Do not edit.\n\
             // Routine {fp} (0 instructions). Source map: {fp}.map.json\n\
             pub fn run(_rt: &mut dyn rpgrt::native::Runtime, _start: usize) -> rpgrt::native::NResult<rpgrt::native::Flow> {{\n\
             \x20   Ok(rpgrt::native::Flow::Normal)\n\
             }}\n",
            fp = r.fingerprint
        );
        map.push_str("]}\n");
        return Ok(Generated { source, map });
    }
    let mut src = String::new();
    let _ = write!(
        src,
        "// Generated by rpgcg {VERSION}. Do not edit.\n\
         // Routine {fp} ({len} instructions). Source map: {fp}.map.json\n\
         use rpgrt::native::{{Flow, MonitorStack, NResult, Runtime, Settled}};\n\
         \n\
         pub fn run(rt: &mut dyn Runtime, start: usize) -> NResult<Flow> {{\n\
         \x20   let mut monitors = MonitorStack::new();\n\
         \x20   let mut pc = start;\n\
         \x20   loop {{\n\
         \x20       let step: NResult<usize> = match pc {{\n",
        fp = r.fingerprint
    );
    // Helper functions of lowered instructions, after `run`.
    let mut helpers = String::new();
    // The 1-based line of the generated source the next text starts on.
    let mut src_line = src
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        .saturating_add(1);
    for (pc, i) in r.ins.iter().enumerate() {
        // `pc < len <= MAX_INSTRUCTIONS`, so this cannot overflow.
        let next = pc.saturating_add(1);
        if pc > 0 {
            map.push(',');
        }
        let _ = write!(
            map,
            "{{\"pc\":{pc},\"line\":{},\"kind\":{},\"what\":{},\"rust_line\":{src_line}}}",
            i.line,
            json_str(kind_name(&i.kind)),
            json_str(&i.what)
        );
        let mut arm = String::new();
        let _ = writeln!(
            arm,
            "            // RPG line {}: {} {}",
            i.line,
            kind_name(&i.kind),
            comment(&i.what)
        );
        let _ = writeln!(arm, "            {pc} => {{");
        if i.cond {
            let _ = writeln!(
                arm,
                "                if !rt.cond({pc}) {{\n                    pc = {next};\n                    continue;\n                }}"
            );
        }
        let _ = writeln!(arm, "                rt.line({pc});");
        let body = match &i.lowered {
            Some(l) => lowered_step(pc, next, i, l, &mut helpers)?,
            None => step(pc, next, &i.kind),
        };
        let _ = writeln!(arm, "                {body}");
        let _ = writeln!(arm, "            }}");
        src_line = src_line.saturating_add(arm.bytes().filter(|&b| b == b'\n').count());
        src.push_str(&arm);
    }
    src.push_str(
        "            _ => return Ok(Flow::Normal),\n\
         \x20       };\n\
         \x20       match rt.settle(pc, step, &mut monitors) {\n\
         \x20           Settled::Go(to) => pc = to,\n\
         \x20           Settled::Exit(r) => return r,\n\
         \x20       }\n\
         \x20   }\n\
         }\n",
    );
    src.push_str(&helpers);
    map.push_str("]}\n");
    Ok(Generated { source: src, map })
}

#[cfg(test)]
mod tests {
    use super::ir::{Ins, Kind, Routine};
    use super::*;

    fn fp() -> String {
        "0123456789abcdef".repeat(4)
    }

    fn ins(kind: Kind, line: usize, cond: bool) -> Ins {
        Ins {
            kind,
            line,
            cond,
            what: "EVAL".to_string(),
            lowered: None,
        }
    }

    #[test]
    fn deterministic() {
        let r = Routine {
            fingerprint: fp(),
            ins: vec![ins(Kind::Stmt, 3, true), ins(Kind::JumpIfNot(0), 4, false)],
        };
        assert_eq!(generate(&r), generate(&r.clone()));
    }

    #[test]
    fn rejects_bad_input() {
        let bad = Routine {
            fingerprint: "x".into(),
            ins: vec![],
        };
        assert!(matches!(generate(&bad), Err(Error::Fingerprint(_))));
        let past = Routine {
            fingerprint: fp(),
            ins: vec![ins(Kind::Jump(5), 7, false)],
        };
        assert_eq!(
            generate(&past),
            Err(Error::Target {
                at: 0,
                line: 7,
                to: 5
            })
        );
        // A target equal to the length ends the routine: valid.
        let end = Routine {
            fingerprint: fp(),
            ins: vec![ins(Kind::Jump(1), 7, false)],
        };
        assert!(generate(&end).is_ok());
    }

    #[test]
    fn lowered_conditions_and_loops() {
        use super::ir::{CmpOp, Cond, Lowered, Operand, Ty};
        let var = |name: &str, buf| Operand::Var {
            name: name.to_string(),
            buf,
            off: 0,
            ty: Ty::Packed(5, 0),
        };
        let mut test = ins(Kind::LoopTest { end: 3 }, 1, false);
        test.lowered = Some(Lowered::LoopTest {
            var: var("I", 2),
            limit: Some(Operand::Num { m: 10, s: 0 }),
            down: false,
        });
        let mut cond = ins(Kind::JumpIfNot(3), 2, false);
        cond.lowered = Some(Lowered::Cond(Cond::And(
            Box::new(Cond::Cmp(
                CmpOp::Gt,
                var("I", 2),
                Operand::Num { m: 0, s: 0 },
            )),
            Box::new(Cond::Cmp(
                CmpOp::Ne,
                var("N", 3),
                Operand::Num { m: 105, s: 2 },
            )),
        )));
        let mut step = ins(Kind::LoopStep, 3, false);
        step.lowered = Some(Lowered::LoopStep {
            var: var("I", 2),
            by: None,
            down: false,
        });
        let r = Routine {
            fingerprint: fp(),
            ins: vec![test, cond, step],
        };
        let Ok(g) = generate(&r) else {
            return assert!(generate(&r).is_ok());
        };
        // The runtime still runs it inside a procedure; the helpers run it
        // otherwise.
        for want in [
            "if rt.in_procedure() {",
            "rt.loop_past(0)",
            "lt0(rt)",
            "fn lt0(rt: &mut dyn Runtime) -> NResult<bool> {",
            "Ok(v > l)",
            "fn c1(rt: &mut dyn Runtime) -> NResult<bool> {",
            "Ok(c1_1(rt)? && c1_2(rt)?)",
            "rpgrt::rpg::dec::Dec::new(105, 2)",
            "fn ls2(rt: &mut dyn Runtime) -> NResult<()> {",
            "rpgrt::rpg::dec::Dec::from_i64(1)",
            "let n = v.add(&step)?;",
        ] {
            assert!(g.source.contains(want), "{want} not in\n{}", g.source);
        }
        assert_eq!(generate(&r), generate(&r.clone()));
        // A lowered form that does not fit its instruction is refused.
        let mut bad = ins(Kind::Stmt, 9, false);
        bad.lowered = Some(Lowered::LoopStep {
            var: var("I", 2),
            by: None,
            down: false,
        });
        let r = Routine {
            fingerprint: fp(),
            ins: vec![bad],
        };
        assert_eq!(generate(&r), Err(Error::Lowered { at: 0, line: 9 }));
    }

    #[test]
    fn lowered_eval() {
        use super::ir::{ArOp, Lowered, NumExpr, Operand, Ty};
        let var = |name: &str, buf| Operand::Var {
            name: name.to_string(),
            buf,
            off: 0,
            ty: Ty::Packed(5, 2),
        };
        let mut ev = ins(Kind::Stmt, 4, true);
        // r = x * y - 0.001
        ev.lowered = Some(Lowered::Eval {
            target: var("R", 1),
            value: NumExpr::Bin {
                op: ArOp::Sub,
                left: Box::new(NumExpr::Bin {
                    op: ArOp::Mul,
                    left: Box::new(NumExpr::Leaf(var("X", 2))),
                    right: Box::new(NumExpr::Leaf(var("Y", 3))),
                    path: vec![0],
                }),
                right: Box::new(NumExpr::Leaf(Operand::Num { m: 1, s: 3 })),
                path: vec![],
            },
            half_adjust: true,
            right: false,
        });
        let r = Routine {
            fingerprint: fp(),
            ins: vec![ev],
        };
        let Ok(g) = generate(&r) else {
            return assert!(generate(&r).is_ok());
        };
        for want in [
            "rt.stmt(0).map(|()| 1)",
            "ev0(rt).map(|()| 1)",
            "fn ev0(rt: &mut dyn Runtime) -> NResult<()> {",
            "let v = e0_(rt)?;",
            "fn e0_l(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {",
            "rt.precision(0, &[0])",
            "rt.precision(0, &[])",
            "a.mul(&b)?",
            "a.sub(&b)?",
            "half_adjust: true",
        ] {
            assert!(g.source.contains(want), "{want} not in\n{}", g.source);
        }
    }

    #[test]
    fn comments_and_map_are_safe() {
        let r = Routine {
            fingerprint: fp(),
            ins: vec![Ins {
                kind: Kind::Stmt,
                line: 1,
                cond: false,
                what: "A\n} fn x() {\"q\"".to_string(),
                lowered: None,
            }],
        };
        let g = generate(&r);
        let Ok(g) = g else {
            return assert!(g.is_ok());
        };
        assert!(g
            .source
            .lines()
            .any(|l| l.trim_start().starts_with("// RPG line 1: stmt A } fn x()")));
        assert!(g.map.contains("\"what\":\"A\\u000a} fn x() {\\\"q\\\"\""));
    }
}
