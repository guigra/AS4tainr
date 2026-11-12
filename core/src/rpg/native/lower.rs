//! A compiled routine (`code::Code`) as the generator's input
//! (`rpgcg::ir`). The match over `Ins` has no wildcard arm: an
//! instruction added upstream stops this building until the generator
//! handles it (docs/native/adr/0001, 0005).

use super::super::ast::{BinOp, Expr, FileOp, Stmt};
use super::super::code::{Code, Ins};
use super::super::data::DType;
use super::super::program::{Program, Sym};
use super::Place;
use rpgcg::ir;

/// The routine's instructions as the generator sees them, and the storage
/// layout its lowered code depends on (one line per field, for the
/// fingerprint: docs/native/adr/0002, 0006).
pub fn lower(prog: &Program, place: &Place, code: &Code) -> (Vec<ir::Ins>, Vec<String>) {
    // Stage 2 lowers the main program's routines only (ADR 0006).
    let main = matches!(place, Place::Detail | Place::Total | Place::Sub(_));
    let mut layout = Vec::new();
    let ins = code
        .iter()
        .map(|i| {
            let (kind, what) = kind(&i.ins);
            let lowered = if main {
                lowered(prog, &i.ins, &mut layout)
            } else {
                None
            };
            ir::Ins {
                kind,
                line: i.line,
                cond: !i.cond.is_empty(),
                what,
                lowered,
            }
        })
        .collect();
    layout.sort();
    layout.dedup();
    (ins, layout)
}

/// What an instruction evaluates, lowered, when all of it is eligible.
fn lowered(prog: &Program, i: &Ins, layout: &mut Vec<String>) -> Option<ir::Lowered> {
    let mut used = Vec::new();
    let l = match i {
        Ins::JumpIfNot(c, _) => ir::Lowered::Cond(cond(prog, c, &mut used)?),
        Ins::LoopTest {
            var, limit, down, ..
        } => ir::Lowered::LoopTest {
            var: variable(prog, var, &mut used)?,
            limit: match limit {
                Some(l) => Some(operand(prog, l, &mut used)?),
                None => None,
            },
            down: *down,
        },
        Ins::LoopStep { var, by, down } => ir::Lowered::LoopStep {
            var: variable(prog, var, &mut used)?,
            by: match by {
                Some(b) => Some(operand(prog, b, &mut used)?),
                None => None,
            },
            down: *down,
        },
        // EVAL with no E extender (H is fine): stmt_e is then plain stmt.
        Ins::Stmt(Stmt::Eval {
            target,
            value,
            ext,
            right,
        }) if !ext.error && !ext.no_lock && ext.other.chars().all(|c| c == 'H') => {
            ir::Lowered::Eval {
                target: variable(prog, target, &mut used)?,
                value: num_expr(prog, value, &mut used, Vec::new())?,
                half_adjust: ext.half_adjust,
                right: *right,
            }
        }
        _ => return None,
    };
    layout.extend(used);
    Some(l)
}

/// A numeric expression of +, - and * over literals and eligible fields
/// (S2.2), each operation with its place in the statement's expression.
fn num_expr(
    prog: &Program,
    e: &Expr,
    used: &mut Vec<String>,
    path: Vec<u8>,
) -> Option<ir::NumExpr> {
    match e {
        Expr::Bin(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), l, r) => {
            let mut lp = path.clone();
            lp.push(0);
            let mut rp = path.clone();
            rp.push(1);
            Some(ir::NumExpr::Bin {
                op: match op {
                    BinOp::Add => ir::ArOp::Add,
                    BinOp::Sub => ir::ArOp::Sub,
                    _ => ir::ArOp::Mul,
                },
                left: Box::new(num_expr(prog, l, used, lp)?),
                right: Box::new(num_expr(prog, r, used, rp)?),
                path,
            })
        }
        _ => operand(prog, e, used).map(ir::NumExpr::Leaf),
    }
}

fn cond(prog: &Program, e: &Expr, used: &mut Vec<String>) -> Option<ir::Cond> {
    let Expr::Bin(op, l, r) = e else {
        return None;
    };
    let cmp = match op {
        BinOp::And | BinOp::Or => {
            let (a, b) = (cond(prog, l, used)?, cond(prog, r, used)?);
            return Some(if *op == BinOp::And {
                ir::Cond::And(Box::new(a), Box::new(b))
            } else {
                ir::Cond::Or(Box::new(a), Box::new(b))
            });
        }
        BinOp::Eq => ir::CmpOp::Eq,
        BinOp::Ne => ir::CmpOp::Ne,
        BinOp::Lt => ir::CmpOp::Lt,
        BinOp::Le => ir::CmpOp::Le,
        BinOp::Gt => ir::CmpOp::Gt,
        BinOp::Ge => ir::CmpOp::Ge,
        _ => return None,
    };
    Some(ir::Cond::Cmp(
        cmp,
        operand(prog, l, used)?,
        operand(prog, r, used)?,
    ))
}

/// A numeric literal or an eligible field.
fn operand(prog: &Program, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    match e {
        Expr::Num(d) => {
            let m: i128 = d.mantissa().to_string().parse().ok()?;
            Some(ir::Operand::Num { m, s: d.scale() })
        }
        _ => variable(prog, e, used),
    }
}

/// An eligible field (ADR 0006): a global numeric scalar of the main
/// program that is not an array, not based, not in a multiple-occurrence
/// data structure, not varying, not imported or exported, not a field of
/// one of the program's files.
fn variable(prog: &Program, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    let Expr::Name { parts, args: None } = e else {
        return None;
    };
    let [name] = parts.as_slice() else {
        return None;
    };
    let Some(Sym::Var(v)) = prog.syms.get(name) else {
        return None;
    };
    let ty = match v.dtype {
        DType::Packed(d, s) => ir::Ty::Packed(d, s),
        DType::Zoned(d, s) => ir::Ty::Zoned(d, s),
        DType::Binary(d, s) => ir::Ty::Binary(d, s),
        DType::Int(n) => ir::Ty::Int(n),
        DType::Uns(n) => ir::Ty::Uns(n),
        _ => return None,
    };
    let excluded = v.local
        || v.dim != 0
        || prog.based.contains_key(&v.buf)
        || prog.occurs.iter().any(|o| o.buf == v.buf)
        || prog.varying.contains_key(name)
        || prog.data_imports.iter().any(|(n, _)| n == name)
        || prog.data_exports.iter().any(|(n, _)| n == name)
        || prog
            .files
            .iter()
            .any(|f| f.fields.iter().any(|f| f == name));
    if excluded {
        return None;
    }
    used.push(format!("layout {name} {} {} {:?}", v.buf, v.off, v.dtype));
    Some(ir::Operand::Var {
        name: name.clone(),
        buf: v.buf,
        off: v.off,
        ty,
    })
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
