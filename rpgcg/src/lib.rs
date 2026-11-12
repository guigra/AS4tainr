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
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+stage1.1");

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
        let _ = writeln!(arm, "                {}", step(pc, next, &i.kind));
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
    fn comments_and_map_are_safe() {
        let r = Routine {
            fingerprint: fp(),
            ins: vec![Ins {
                kind: Kind::Stmt,
                line: 1,
                cond: false,
                what: "A\n} fn x() {\"q\"".to_string(),
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
