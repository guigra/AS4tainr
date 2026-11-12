//! A compiled routine (`code::Code`) as the generator's input
//! (`rpgcg::ir`). The match over `Ins` has no wildcard arm: an
//! instruction added upstream stops this building until the generator
//! handles it (docs/native/adr/0001, 0005).

use super::super::ast::{FileOp, Stmt};
use super::super::code::{Code, Ins};
use rpgcg::ir;

/// The routine as the generator sees it.
pub fn lower(code: &Code, fingerprint: String) -> ir::Routine {
    let ins = code
        .iter()
        .map(|i| {
            let (kind, what) = kind(&i.ins);
            ir::Ins {
                kind,
                line: i.line,
                cond: !i.cond.is_empty(),
                what,
            }
        })
        .collect();
    ir::Routine { fingerprint, ins }
}

fn kind(i: &Ins) -> (ir::Kind, String) {
    use ir::Kind as K;
    match i {
        Ins::Stmt(s) => (K::Stmt, describe(s)),
        Ins::Jump(t) => (K::Jump(*t), "GOTO/ITER/LEAVE/ELSE".into()),
        Ins::JumpIfNot(_, t) => (K::JumpIfNot(*t), "IF/DOW/DOU/WHEN".into()),
        Ins::LoopTest { end, .. } => (K::LoopTest { end: *end }, "FOR/DO test".into()),
        Ins::LoopStep { .. } => (K::LoopStep, "FOR/DO step".into()),
        Ins::EachInit { .. } => (K::EachInit, "FOR-EACH".into()),
        Ins::EachNext { end, .. } => (K::EachNext { end: *end }, "FOR-EACH next".into()),
        Ins::ExSr(name) => (K::ExSr, format!("EXSR {name}")),
        Ins::Return(_) => (K::Return, "RETURN".into()),
        Ins::LeaveSr => (K::LeaveSr, "LEAVESR".into()),
        // A TAG outside the routine is the runtime's GOTO signal; no TAG
        // (usize::MAX) only sets the indicators.
        Ins::Cab { to, out, .. } => (
            K::Cab {
                to: if out.is_some() || *to == usize::MAX {
                    None
                } else {
                    Some(*to)
                },
            },
            "CABxx".into(),
        ),
        Ins::Label(name) => (K::Label, format!("TAG {name}")),
        Ins::GotoOut(name) => (K::GotoOut, format!("GOTO {name} (out)")),
        Ins::EndSr(_) => (K::EndSr, "ENDSR return point".into()),
        Ins::Monitor { end_body, end, .. } => (
            K::Monitor {
                end_body: *end_body,
                end: *end,
            },
            "MONITOR".into(),
        ),
        Ins::EndMonitor { end } => (K::EndMonitor { end: *end }, "ON-ERROR/ENDMON".into()),
        Ins::Nop => (K::Nop, String::new()),
    }
}

/// The operation a statement is, for the generated comment.
fn describe(s: &Stmt) -> String {
    match s {
        Stmt::Sql(_) => "EXEC SQL".into(),
        Stmt::Eval { right: true, .. } => "EVALR".into(),
        Stmt::Eval { .. } => "EVAL".into(),
        Stmt::CallP(_) => "CALLP".into(),
        Stmt::Call { .. } => "CALL".into(),
        Stmt::File { op, file, .. } => format!("{} {file}", file_op(op)),
        Stmt::Dsply { .. } => "DSPLY".into(),
        Stmt::SetInd { on: true, .. } => "SETON".into(),
        Stmt::SetInd { .. } => "SETOFF".into(),
        Stmt::Arith { op, .. } => format!("{op:?}").to_ascii_uppercase(),
        Stmt::Move { left: true, .. } => "MOVEL".into(),
        Stmt::Move { .. } => "MOVE".into(),
        Stmt::Comp { .. } => "COMP".into(),
        Stmt::Clear(_) => "CLEAR".into(),
        Stmt::Reset(_) => "RESET".into(),
        Stmt::Nop => String::new(),
        Stmt::Unsupported(op) => format!("{op} (not supported: fails when reached)"),
        Stmt::Op { code, .. } => code.clone(),
    }
}

fn file_op(op: &FileOp) -> String {
    format!("{op:?}").to_ascii_uppercase()
}
