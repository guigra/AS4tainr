//! A compiled routine (`code::Code`) as the generator's input
//! (`rpgcg::ir`). The match over `Ins` has no wildcard arm: an
//! instruction added upstream stops this building until the generator
//! handles it (docs/native/adr/0001, 0005).

use super::super::ast::{Arith, BinOp, Expr, FileOp, Stmt};
use super::super::code::{Code, Ins};
use super::super::data::{DType, Fig, Value};
use super::super::program::{Program, Sym};
use super::Place;
use rpgcg::ir;

/// The routine's instructions as the generator sees them, and the storage
/// layout its lowered code depends on (one line per field, for the
/// fingerprint: docs/native/adr/0002, 0006).
pub fn lower(prog: &Program, place: &Place, code: &Code) -> (Vec<ir::Ins>, Vec<String>) {
    // The names the routine sees: the main program's, or a procedure's
    // (its locals, and the globals bound into it); S2.8.
    let cx = match place {
        Place::Detail | Place::Total | Place::Sub(_) => Some(Cx {
            prog,
            syms: &prog.syms,
            based: None,
            protos: None,
        }),
        Place::Proc(name, _) => prog.procs.get(name).map(|pi| Cx {
            prog,
            syms: &pi.syms,
            based: Some(&pi.based),
            protos: Some(&pi.protos),
        }),
    };
    let mut layout = Vec::new();
    let ins = code
        .iter()
        .map(|i| {
            let (kind, what) = kind(&i.ins);
            let lowered = cx.as_ref().and_then(|cx| lowered(cx, &i.ins, &mut layout));
            if lowered.is_none() {
                if let Some(cx) = &cx {
                    note_why(cx, &i.ins);
                }
            }
            ir::Ins {
                kind,
                line: i.line,
                cond: !i.cond.is_empty(),
                what,
                lowered,
                refused: past_limits(&i.ins),
            }
        })
        .collect();
    layout.sort();
    layout.dedup();
    (ins, layout)
}

/// What lowering looks names up in.
struct Cx<'p> {
    prog: &'p Program,
    syms: &'p super::super::program::SymMap,
    /// A procedure's based locals (frame buffer → pointer).
    based: Option<&'p std::collections::HashMap<usize, String>>,
    /// A procedure's own prototypes (S2.46), looked up before the
    /// program's, as the interpreter's `proto`.
    protos: Option<&'p std::collections::HashMap<String, super::super::program::Proto>>,
}

/// IBM i's limit on levels of nesting in an expression: "Maximum of 100"
/// (ILE RPG Reference, Appendix A. RPG IV Restrictions, IBM i 7.4 and 7.5,
/// https://www.ibm.com/docs/ssw_ibm_i_75/rzasd/rpgrest.htm, read
/// 2026-11-15). A chain of the same kind of operation (a + b + c, a || b
/// || c, x AND y AND z) is one level: lowering walks it with a loop.
pub const MAX_NESTING: usize = 100;

/// The most expression nodes one instruction may have (a resource limit,
/// CLAUDE.native.md section 5): far above what a program written by hand
/// has, and what bounds the paths `rpgrt::native::path_rle` builds.
pub const MAX_NODES: usize = 2048;

/// The nodes of an expression, counted with an explicit stack (no
/// recursion), up to one past `MAX_NODES`.
fn node_count(e: &Expr) -> usize {
    let mut stack = vec![e];
    let mut n = 0usize;
    while let Some(x) = stack.pop() {
        n = n.saturating_add(1);
        if n > MAX_NODES {
            return n;
        }
        match x {
            Expr::Unary(_, a) => stack.push(a),
            Expr::Bin(_, l, r) => {
                stack.push(r);
                stack.push(l);
            }
            Expr::Bif(_, args) => stack.extend(args.iter()),
            Expr::Name {
                args: Some(args), ..
            } => stack.extend(args.iter()),
            Expr::Path { idx, args, .. } => {
                stack.extend(idx.iter().flatten().map(|b| &**b));
                stack.extend(args.iter().flatten());
            }
            Expr::IndArray(Some(a)) => stack.push(a),
            _ => {}
        }
    }
    n
}

/// Whether an operation chains with its left operand into one level.
fn chains(op: &BinOp, left: &Expr) -> bool {
    match left {
        Expr::Bin(lop, _, _) => match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul => {
                matches!(lop, BinOp::Add | BinOp::Sub | BinOp::Mul)
            }
            BinOp::And | BinOp::Or => lop == op,
            _ => false,
        },
        _ => false,
    }
}

/// Whether an expression nests at most `MAX_NESTING` levels: a chain's
/// left operands are walked with a loop; the recursion is into nested
/// operands only, and it stops past the limit.
fn nests_within(e: &Expr, level: usize) -> bool {
    if level > MAX_NESTING {
        return false;
    }
    let d = level.saturating_add(1);
    match e {
        Expr::Bin(..) => {
            // The chain down the left operands is this level: walked with
            // a loop, its right operands and its first operand one deeper.
            let mut rights = Vec::new();
            let mut cur: &Expr = e;
            while let Expr::Bin(o, l, r) = cur {
                rights.push(&**r);
                let more = chains(o, l);
                cur = l;
                if !more {
                    break;
                }
            }
            nests_within(cur, d) && rights.into_iter().all(|x| nests_within(x, d))
        }
        Expr::Unary(_, x) => nests_within(x, d),
        Expr::Bif(_, args) => args.iter().all(|a| nests_within(a, d)),
        Expr::Name {
            args: Some(args), ..
        } => args.iter().all(|a| nests_within(a, d)),
        Expr::Path { idx, args, .. } => {
            idx.iter().flatten().all(|a| nests_within(a, d))
                && args.iter().flatten().all(|a| nests_within(a, d))
        }
        Expr::IndArray(Some(x)) => nests_within(x, d),
        _ => true,
    }
}

/// The expressions of an instruction that lowering may walk.
fn ins_exprs(i: &Ins) -> Vec<&Expr> {
    let mut v = Vec::new();
    match i {
        Ins::JumpIfNot(c, _) => v.push(c),
        Ins::LoopTest { var, limit, .. } => {
            v.push(var);
            v.extend(limit.iter());
        }
        Ins::LoopStep { var, by, .. } => {
            v.push(var);
            v.extend(by.iter());
        }
        Ins::Return(x) => v.extend(x.iter()),
        Ins::Stmt(Stmt::Eval { target, value, .. }) => {
            v.push(target);
            v.push(value);
        }
        Ins::Stmt(Stmt::CallP(e)) => v.push(e),
        Ins::Stmt(Stmt::Dsply { message, .. }) => v.extend(message.iter()),
        _ => {}
    }
    v
}

/// Why an instruction is past IBM i's limits, if it is (S2.41): the
/// routine is then refused with a diagnostic naming the line, not run
/// by the interpreter instruction by instruction.
fn past_limits(i: &Ins) -> Option<String> {
    for e in ins_exprs(i) {
        let n = node_count(e);
        if n > MAX_NODES {
            return Some(format!(
                "an expression of more than {MAX_NODES} nodes (the native engine's limit)"
            ));
        }
        if !nests_within(e, 1) {
            return Some(format!(
                "an expression nested more than {MAX_NESTING} levels (IBM i's limit, ILE RPG Reference, RPG IV Restrictions)"
            ));
        }
    }
    None
}

/// What an instruction evaluates, lowered, when all of it is eligible.
fn lowered(cx: &Cx<'_>, i: &Ins, layout: &mut Vec<String>) -> Option<ir::Lowered> {
    if past_limits(i).is_some() {
        return None;
    }
    let mut used = Vec::new();
    let l = match i {
        Ins::JumpIfNot(c, _) => ir::Lowered::Cond(cond(cx, c, &mut used)?),
        Ins::LoopTest {
            var, limit, down, ..
        } => ir::Lowered::LoopTest {
            var: num_variable(cx, var, &mut used)?,
            limit: match limit {
                Some(l) => Some(operand(cx, l, &mut used)?),
                None => None,
            },
            down: *down,
        },
        Ins::LoopStep { var, by, down } => ir::Lowered::LoopStep {
            var: num_variable(cx, var, &mut used)?,
            by: match by {
                Some(b) => Some(operand(cx, b, &mut used)?),
                None => None,
            },
            down: *down,
        },
        // Fixed-form ADD, SUB, MULT, Z-ADD, Z-SUB with no E extender.
        Ins::Stmt(Stmt::Arith {
            op,
            f1,
            f2,
            result,
            ext,
            ind,
        }) if !ext.error && !ext.no_lock && ext.other.chars().all(|c| c == 'H') => {
            let op = match op {
                Arith::ZAdd => ir::FixOp::ZAdd,
                Arith::ZSub => ir::FixOp::ZSub,
                Arith::Add => ir::FixOp::Add,
                Arith::Sub => ir::FixOp::Sub,
                Arith::Mult => ir::FixOp::Mult,
                // DIV's quotient gets the result field's decimal places
                // (`type_of(result).precision()`, as the interpreter).
                Arith::Div => {
                    let Expr::Name { parts, args: None } = result else {
                        return None;
                    };
                    let [name] = parts.as_slice() else {
                        return None;
                    };
                    let Some(Sym::Var(v)) = cx.syms.get(name) else {
                        return None;
                    };
                    ir::FixOp::Div {
                        decs: u8::try_from(v.dtype.precision().1).ok()?,
                    }
                }
                Arith::Mvr => ir::FixOp::Mvr,
            };
            ir::Lowered::Arith {
                op,
                f1: match f1 {
                    Some(x) => Some(operand(cx, x, &mut used)?),
                    None => None,
                },
                // MVR has no factor 2 (the generated code does not read it).
                f2: if op == ir::FixOp::Mvr {
                    ir::Operand::Num { m: 0, s: 0 }
                } else {
                    operand(cx, f2, &mut used)?
                },
                result: num_variable(cx, result, &mut used)?,
                half_adjust: ext.half_adjust,
                inds: !ind.is_empty(),
            }
        }
        // MOVE and MOVEL with no date format and no result indicators
        // (S2.11); eligible fields are never dates, so `move_temporal`
        // does nothing for them.
        Ins::Stmt(Stmt::Move {
            left,
            pad,
            fmt: None,
            from,
            to,
            ind,
        }) if ind.is_empty()
            && !matches!(from, Expr::Fig(_))
            // No pointer in MOVE (S2.47): the interpreter's do_move.
            && !is_pointer_name(cx, to)
            && !is_pointer_name(cx, from) =>
        {
            ir::Lowered::Move {
                left: *left,
                pad: *pad,
                from: any_operand(cx, from, &mut used)?,
                to: variable(cx, to, &mut used)?,
            }
        }
        // %ELEM of a global varying-dimension array set (S2.45):
        // assign_value's "ELEM" arm.
        Ins::Stmt(Stmt::Eval {
            target: Expr::Bif(b, args),
            value,
            ext,
            ..
        }) if b == "ELEM" && !ext.error && !ext.no_lock && args.len() == 1 => {
            let Some(Expr::Name { parts, args: None }) = args.first() else {
                return None;
            };
            let name = parts.join(".");
            let Some(Sym::Var(v)) = cx.syms.get(&name) else {
                return None;
            };
            let prog = cx.prog;
            if v.local
                || v.dim == 0
                || !prog.varying.contains_key(&name)
                || prog.based.contains_key(&v.buf)
                || prog.occurs.iter().any(|o| o.buf == v.buf)
            {
                return None;
            }
            let ty = param_ty(v.dtype)?;
            used.push(format!(
                "set elems {name} {} {} {} {} {:?}",
                v.buf, v.off, v.stride, v.dim, v.dtype
            ));
            ir::Lowered::SetElems {
                name,
                buf: v.buf,
                off: v.off,
                stride: v.stride,
                dim: v.dim,
                ty,
                value: Box::new(num_expr(cx, value, &mut used, Vec::new())?),
            }
        }
        // EVAL with %SUBST as its target (S2.44): assign_value's "SUBST"
        // arm, on an eligible character field, with a character value.
        Ins::Stmt(Stmt::Eval {
            target: Expr::Bif(b, args),
            value,
            ext,
            right,
        }) if b == "SUBST" && !ext.error && !ext.no_lock && ext.other.chars().all(|c| c == 'H') => {
            let (base, start, len) = match args.as_slice() {
                [f, s] => (f, s, None),
                [f, s, l] => (f, s, Some(l)),
                _ => return None,
            };
            let base = variable(cx, base, &mut used)?;
            if !is_char(&base) {
                return None;
            }
            let tp = rpgrt::native::TARGET_PATH;
            ir::Lowered::EvalSubst {
                base,
                start: Box::new(num_expr(cx, start, &mut used, vec![tp, 1])?),
                len: match len {
                    Some(l) => Some(Box::new(num_expr(cx, l, &mut used, vec![tp, 2])?)),
                    None => None,
                },
                value: Box::new(char_expr(cx, value, &mut used, Vec::new())?),
                half_adjust: ext.half_adjust,
                right: *right,
            }
        }
        // EVAL of an indicator (S2.39): Stmt::Eval then assign's
        // `set_ind(n, val.truth()?)`, for *ON, *OFF or a condition.
        Ins::Stmt(Stmt::Eval {
            target: Expr::Ind(n),
            value,
            ext,
            ..
        }) if !ext.error && !ext.no_lock => ir::Lowered::EvalInd {
            name: n.clone(),
            value: match value {
                Expr::Fig(Fig::On) => ir::IndValue::On,
                Expr::Fig(Fig::Off) => ir::IndValue::Off,
                v => ir::IndValue::Cond(cond_at(cx, v, &mut used, Vec::new())?),
            },
        },
        // SETON / SETOFF (S2.39): Stmt::SetInd's `set_ind(i, on)` for each.
        Ins::Stmt(Stmt::SetInd { on, inds }) => ir::Lowered::SetInd {
            on: *on,
            inds: inds.clone(),
        },
        // RETURN (S2.38) of an eligible numeric or character expression,
        // or of nothing (a whole data structure array is not eligible).
        Ins::Return(v) => ir::Lowered::Return(match v {
            None => None,
            Some(x) => Some(match char_expr(cx, x, &mut used, Vec::new()) {
                Some(c) => ir::ValExpr::Char(c),
                None => ir::ValExpr::Num(num_expr(cx, x, &mut used, Vec::new())?),
            }),
        }),
        // DSPLY with no response field (S2.36): Stmt::Dsply's batch path.
        // A float, date or time message is not an eligible expression.
        Ins::Stmt(Stmt::Dsply {
            message,
            response: None,
        }) => ir::Lowered::Dsply(match message {
            None => None,
            Some(m) => Some(match char_expr(cx, m, &mut used, Vec::new()) {
                Some(c) => ir::ValExpr::Char(c),
                None => ir::ValExpr::Num(num_expr(cx, m, &mut used, Vec::new())?),
            }),
        }),
        // CALLP of a procedure of this module (S2.35, ADR 0007).
        Ins::Stmt(Stmt::CallP(Expr::Name { parts, args })) => ir::Lowered::Call(call_of(
            cx,
            parts,
            args.as_deref().unwrap_or(&[]),
            &mut used,
            Vec::new(),
        )?),
        // EVAL of a pointer field (S2.47): another pointer field, or %ADDR
        // of a global field (ptr_to of its place; *DATA skips a varying
        // field's length). *NULL is EVAL of a figurative constant below.
        Ins::Stmt(Stmt::Eval {
            target, value, ext, ..
        }) if !ext.error
            && !ext.no_lock
            && is_pointer_name(cx, target)
            && !matches!(value, Expr::Fig(_)) =>
        {
            let t = variable(cx, target, &mut used)?;
            let value = ptr_val(cx, value, &mut used)?;
            ir::Lowered::EvalPtr { target: t, value }
        }
        // EVAL of a figurative constant (S2.22): stored as the interpreter
        // stores Value::Fig (data::store fills the field by its type).
        Ins::Stmt(Stmt::Eval {
            target,
            value: Expr::Fig(f),
            ext,
            right,
        }) if !ext.error && !ext.no_lock && ext.other.chars().all(|c| c == 'H') => {
            ir::Lowered::EvalFig {
                target: variable(cx, target, &mut used)?,
                fig: fig_op(f)?,
                half_adjust: ext.half_adjust,
                right: *right,
            }
        }
        // EVAL with no E extender (H is fine): stmt_e is then plain stmt.
        Ins::Stmt(Stmt::Eval {
            target,
            value,
            ext,
            right,
        }) if !ext.error && !ext.no_lock && ext.other.chars().all(|c| c == 'H') => {
            let target = variable(cx, target, &mut used)?;
            if is_pointer(&target) {
                return None;
            }
            if is_char(&target) {
                ir::Lowered::EvalChar {
                    target,
                    value: char_expr(cx, value, &mut used, Vec::new())?,
                    half_adjust: ext.half_adjust,
                    right: *right,
                }
            } else {
                ir::Lowered::Eval {
                    target,
                    value: num_expr(cx, value, &mut used, Vec::new())?,
                    half_adjust: ext.half_adjust,
                    right: *right,
                }
            }
        }
        _ => return None,
    };
    layout.extend(used);
    Some(l)
}

/// A character expression (S2.4): character literals and eligible
/// character fields, joined with `+`.
fn char_expr(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>, path: Vec<u8>) -> Option<ir::CharExpr> {
    match e {
        // A procedure of this module returning a character value (S2.35).
        Expr::Name {
            parts,
            args: Some(args),
        } if is_call(cx, parts) => {
            if !matches!(
                call_returns(cx, &parts.join(".")),
                Some(DType::Char(_) | DType::Varchar(_))
            ) {
                return None;
            }
            Some(ir::CharExpr::Call(call_of(cx, parts, args, used, path)?))
        }
        // A chain of concatenations down the left operands (S2.41), walked
        // with a loop.
        Expr::Bin(BinOp::Add, l, _) if matches!(&**l, Expr::Bin(BinOp::Add, _, _)) => {
            let mut rev = Vec::new();
            let mut cur: &Expr = e;
            let mut p = path.clone();
            while let Expr::Bin(BinOp::Add, l2, r2) = cur {
                let mut rp = p.clone();
                rp.push(1);
                rev.push((&**r2, rp));
                cur = l2;
                p.push(0);
            }
            let mut parts = Vec::with_capacity(rev.len().saturating_add(1));
            parts.push(char_expr(cx, cur, used, p)?);
            for (r, rp) in rev.into_iter().rev() {
                parts.push(char_expr(cx, r, used, rp)?);
            }
            Some(ir::CharExpr::Chain { parts, path })
        }
        Expr::Bin(BinOp::Add, l, r) => {
            let mut lp = path.clone();
            lp.push(0);
            let mut rp = path.clone();
            rp.push(1);
            Some(ir::CharExpr::Cat {
                left: Box::new(char_expr(cx, l, used, lp)?),
                right: Box::new(char_expr(cx, r, used, rp)?),
                path,
            })
        }
        // %SUBST of a character expression, with a start and a length that
        // are numeric literals or fields (S2.10).
        Expr::Bif(b, args) if b == "SUBST" => {
            let (inner, start, len) = match args.as_slice() {
                [i, s] => (i, s, None),
                [i, s, l] => (i, s, Some(l)),
                _ => return None,
            };
            let mut ip = path.clone();
            ip.push(0);
            Some(ir::CharExpr::Subst {
                inner: Box::new(char_expr(cx, inner, used, ip)?),
                start: operand(cx, start, used)?,
                len: match len {
                    Some(l) => Some(operand(cx, l, used)?),
                    None => None,
                },
                path,
            })
        }
        // %CHAR of a numeric expression (S2.14). The program's DECEDIT is
        // part of the code, so it goes into the fingerprint's layout.
        Expr::Bif(b, args) if b == "CHAR" => {
            let [inner] = args.as_slice() else {
                return None;
            };
            let mut ip = path.clone();
            ip.push(0);
            let inner = num_expr(cx, inner, used, ip)?;
            let (point, zero) = cx.prog.decedit();
            used.push(format!("decedit {point} {zero}"));
            Some(ir::CharExpr::CharOfNum {
                inner,
                point,
                zero,
                path,
            })
        }
        // %EDITC of a numeric expression with a literal edit code (S2.26):
        // the code's first character (Z when it is empty), the program's
        // DECEDIT in the fingerprint, the interpreter's precision rule.
        Expr::Bif(b, args) if b == "EDITC" => {
            let [x, Expr::Str(code)] = args.as_slice() else {
                return None;
            };
            let mut ip = path.clone();
            ip.push(0);
            let inner = num_expr(cx, x, used, ip)?;
            let code = super::super::data::a(code).chars().next().unwrap_or('Z');
            let (point, zero) = cx.prog.decedit();
            used.push(format!("decedit {point} {zero}"));
            Some(ir::CharExpr::EditC {
                own: matches!(x, Expr::Num(_)),
                inner,
                code,
                point,
                zero,
                path,
            })
        }
        // %EDITW of a numeric expression with a literal edit word (S2.29):
        // a field's type gives the digits, as the interpreter's type_of;
        // any other expression its own digits.
        Expr::Bif(b, args) if b == "EDITW" => {
            let [x, Expr::Str(word)] = args.as_slice() else {
                return None;
            };
            let mut ip = path.clone();
            ip.push(0);
            let inner = num_expr(cx, x, used, ip)?;
            let prec = match &inner {
                ir::NumExpr::Leaf(ir::Operand::Var { ty, .. }) => match ty {
                    ir::Ty::Packed(d, s) | ir::Ty::Zoned(d, s) | ir::Ty::Binary(d, s) => {
                        Some((*d, *s))
                    }
                    ir::Ty::Int(n) | ir::Ty::Uns(n) => Some((*n, 0)),
                    _ => return None,
                },
                _ => None,
            };
            Some(ir::CharExpr::EditW {
                inner,
                word: super::super::data::a(word),
                prec,
                path,
            })
        }
        // %SCANRPL of character expressions, with a start and a length that
        // are numeric literals or fields (S2.28).
        Expr::Bif(b, args) if b == "SCANRPL" => {
            let (pat, rep, s, start, len) = match args.as_slice() {
                [a, r, s] => (a, r, s, None, None),
                [a, r, s, st] => (a, r, s, Some(st), None),
                [a, r, s, st, n] => (a, r, s, Some(st), Some(n)),
                _ => return None,
            };
            let child = |k: u8| {
                let mut c = path.clone();
                c.push(k);
                c
            };
            Some(ir::CharExpr::ScanRpl {
                pat: Box::new(char_expr(cx, pat, used, child(0))?),
                rep: Box::new(char_expr(cx, rep, used, child(1))?),
                s: Box::new(char_expr(cx, s, used, child(2))?),
                start: match start {
                    Some(x) => Some(operand(cx, x, used)?),
                    None => None,
                },
                len: match len {
                    Some(x) => Some(operand(cx, x, used)?),
                    None => None,
                },
                path,
            })
        }
        // %REPLACE of character expressions, with a start and a length
        // that are numeric literals or fields (S2.23).
        Expr::Bif(b, args) if b == "REPLACE" => {
            let (rep, s, start, len) = match args.as_slice() {
                [r, s] => (r, s, None, None),
                [r, s, st] => (r, s, Some(st), None),
                [r, s, st, n] => (r, s, Some(st), Some(n)),
                _ => return None,
            };
            let child = |k: u8| {
                let mut c = path.clone();
                c.push(k);
                c
            };
            Some(ir::CharExpr::Replace {
                rep: Box::new(char_expr(cx, rep, used, child(0))?),
                s: Box::new(char_expr(cx, s, used, child(1))?),
                start: match start {
                    Some(x) => Some(operand(cx, x, used)?),
                    None => None,
                },
                len: match len {
                    Some(x) => Some(operand(cx, x, used)?),
                    None => None,
                },
                path,
            })
        }
        // %XLATE of character expressions, with a start that is a numeric
        // literal or field (S2.12).
        Expr::Bif(b, args) if b == "XLATE" => {
            let (from, to, s, start) = match args.as_slice() {
                [f, o, s] => (f, o, s, None),
                [f, o, s, st] => (f, o, s, Some(st)),
                _ => return None,
            };
            let child = |k: u8| {
                let mut c = path.clone();
                c.push(k);
                c
            };
            Some(ir::CharExpr::Xlate {
                from: Box::new(char_expr(cx, from, used, child(0))?),
                to: Box::new(char_expr(cx, to, used, child(1))?),
                s: Box::new(char_expr(cx, s, used, child(2))?),
                start: match start {
                    Some(x) => Some(operand(cx, x, used)?),
                    None => None,
                },
                path,
            })
        }
        // %TRIM, %TRIML, %TRIMR of one character expression (S2.9).
        Expr::Bif(b, args) if matches!(b.as_str(), "TRIM" | "TRIML" | "TRIMR") => {
            let [inner] = args.as_slice() else {
                return None;
            };
            let mut ip = path.clone();
            ip.push(0);
            Some(ir::CharExpr::Trim {
                inner: Box::new(char_expr(cx, inner, used, ip)?),
                left: b != "TRIMR",
                right: b != "TRIML",
                path,
            })
        }
        _ => {
            let o = any_operand(cx, e, used)?;
            is_char(&o).then_some(ir::CharExpr::Leaf(o))
        }
    }
}

/// A numeric expression of +, - and * over literals and eligible fields
/// (S2.2), each operation with its place in the statement's expression.
fn num_expr(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>, path: Vec<u8>) -> Option<ir::NumExpr> {
    match e {
        // %ELEM of a global varying-dimension array (S2.45): its elements
        // in use, bifs.rs bif_override "ELEM".
        Expr::Bif(b, args) if b == "ELEM" && args.len() == 1 => {
            let Some(Expr::Name { parts, args: None }) = args.first() else {
                return None;
            };
            let name = parts.join(".");
            match cx.syms.get(&name) {
                Some(Sym::Var(v))
                    if !v.local && v.dim > 0 && cx.prog.varying.contains_key(&name) =>
                {
                    used.push(format!("elems {name}"));
                    Some(ir::NumExpr::Elems(name))
                }
                _ => operand(cx, e, used).map(ir::NumExpr::Leaf),
            }
        }
        // A procedure of this module returning a number (S2.35).
        Expr::Name {
            parts,
            args: Some(args),
        } if is_call(cx, parts) => {
            if !matches!(
                call_returns(cx, &parts.join(".")),
                Some(
                    DType::Packed(..)
                        | DType::Zoned(..)
                        | DType::Binary(..)
                        | DType::Int(_)
                        | DType::Uns(_)
                )
            ) {
                return None;
            }
            Some(ir::NumExpr::Call(call_of(cx, parts, args, used, path)?))
        }
        // A chain of +, - and * down the left operands (S2.41): walked with
        // a loop, each operation keeping its own path for its precision.
        Expr::Bin(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), l, _) if chains(op, l) => {
            let mut rev = Vec::new();
            let mut cur: &Expr = e;
            let mut p = path.clone();
            while let Expr::Bin(o @ (BinOp::Add | BinOp::Sub | BinOp::Mul), l2, r2) = cur {
                rev.push((ar_op(o), &**r2, p.clone()));
                let more = chains(o, l2);
                cur = l2;
                p.push(0);
                if !more {
                    break;
                }
            }
            let first = num_expr(cx, cur, used, p)?;
            let mut steps = Vec::with_capacity(rev.len());
            for (o, r, np) in rev.into_iter().rev() {
                let mut rp = np.clone();
                rp.push(1);
                steps.push((o, num_expr(cx, r, used, rp)?, np));
            }
            Some(ir::NumExpr::Chain {
                first: Box::new(first),
                steps,
                path,
            })
        }
        Expr::Bin(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div), l, r) => {
            let mut lp = path.clone();
            lp.push(0);
            let mut rp = path.clone();
            rp.push(1);
            Some(ir::NumExpr::Bin {
                op: match op {
                    BinOp::Add => ir::ArOp::Add,
                    BinOp::Sub => ir::ArOp::Sub,
                    BinOp::Div => ir::ArOp::Div,
                    _ => ir::ArOp::Mul,
                },
                left: Box::new(num_expr(cx, l, used, lp)?),
                right: Box::new(num_expr(cx, r, used, rp)?),
                path,
            })
        }
        // %DIV and %REM of two numeric expressions (S2.18).
        Expr::Bif(b, args) if matches!(b.as_str(), "DIV" | "REM") => {
            let [x, y] = args.as_slice() else {
                return None;
            };
            let mut lp = path.clone();
            lp.push(0);
            let mut rp = path.clone();
            rp.push(1);
            Some(ir::NumExpr::DivRem {
                left: Box::new(num_expr(cx, x, used, lp)?),
                right: Box::new(num_expr(cx, y, used, rp)?),
                rem: b == "REM",
                path,
            })
        }
        // %STATUS with no file (S2.31): the code the interpreter's
        // bifs.rs "STATUS" arm gives, `Dec::new(code, 0)`.
        Expr::Bif(b, args) if b == "STATUS" && args.is_empty() => Some(ir::NumExpr::Status),
        // %LEN of a varying-length field (S2.20): its current length, as
        // the interpreter's `(Some(DType::Varchar(_)), Value::Char(c))`
        // arm gives it.
        Expr::Bif(b, args) if b == "LEN" && args.len() == 1 => {
            let v = variable(cx, args.first()?, used)?;
            match v {
                ir::Operand::Var {
                    ty: ir::Ty::Varchar(_),
                    ..
                } => Some(ir::NumExpr::VarLen { var: v }),
                _ => operand(cx, e, used).map(ir::NumExpr::Leaf),
            }
        }
        // %LOOKUP and %LOOKUPxx of a literal or field in a global array
        // (S2.27), with a start and a number of elements that are numeric
        // literals or fields.
        Expr::Bif(b, args)
            if matches!(
                b.as_str(),
                "LOOKUP" | "LOOKUPLT" | "LOOKUPLE" | "LOOKUPGE" | "LOOKUPGT"
            ) =>
        {
            let (x, arr, start, n) = match args.as_slice() {
                [x, a] => (x, a, None, None),
                [x, a, s] => (x, a, Some(s), None),
                [x, a, s, n] => (x, a, Some(s), Some(n)),
                _ => return None,
            };
            let Expr::Name { parts, args: None } = arr else {
                return None;
            };
            let name = parts.join(".");
            let Some(Sym::Var(v)) = cx.syms.get(&name) else {
                return None;
            };
            let prog = cx.prog;
            if v.dim == 0
                || v.local
                || name.starts_with("TAB")
                || prog.varying.contains_key(&name)
                || prog.based.contains_key(&v.buf)
                || prog.occurs.iter().any(|o| o.buf == v.buf)
                || prog.data_imports.iter().any(|(n, _)| *n == name)
                || prog.data_exports.iter().any(|(n, _)| *n == name)
            {
                return None;
            }
            let ty = match v.dtype {
                DType::Packed(d, s) => ir::Ty::Packed(d, s),
                DType::Zoned(d, s) => ir::Ty::Zoned(d, s),
                DType::Binary(d, s) => ir::Ty::Binary(d, s),
                DType::Int(n) => ir::Ty::Int(n),
                DType::Uns(n) => ir::Ty::Uns(n),
                DType::Char(n) => ir::Ty::Char(n),
                DType::Varchar(n) => ir::Ty::Varchar(n),
                _ => return None,
            };
            let x = any_operand(cx, x, used)?;
            if matches!(x, ir::Operand::Fig(_)) {
                return None;
            }
            used.push(format!(
                "lookup {name} {} {} {} {} {:?}",
                v.buf, v.off, v.stride, v.dim, v.dtype
            ));
            Some(ir::NumExpr::Lookup {
                x,
                buf: v.buf,
                off: v.off,
                stride: v.stride,
                dim: v.dim,
                ty,
                start: match start {
                    Some(s) => Some(operand(cx, s, used)?),
                    None => None,
                },
                n: match n {
                    Some(s) => Some(operand(cx, s, used)?),
                    None => None,
                },
                how: b.get(6..).unwrap_or_default().to_string(),
            })
        }
        // %MAX, %MIN of numeric expressions (S2.33): the interpreter's
        // bifs.rs "MAX" | "MIN" arm, every argument evaluated in order and
        // the first of the greatest or smallest kept, with its own scale.
        Expr::Bif(b, args) if matches!(b.as_str(), "MAX" | "MIN") && args.len() >= 2 => {
            let mut out = Vec::with_capacity(args.len());
            for (k, a) in args.iter().enumerate() {
                let mut ap = path.clone();
                ap.push(u8::try_from(k).ok()?);
                out.push(num_expr(cx, a, used, ap)?);
            }
            Some(ir::NumExpr::MinMax {
                args: out,
                max: b == "MAX",
                path,
            })
        }
        // %ABS of a numeric expression (S2.32): the interpreter's
        // `v.num()?.abs()` (interp.rs "ABS"; a float argument is not an
        // eligible numeric expression).
        Expr::Bif(b, args) if b == "ABS" => {
            let [inner] = args.as_slice() else {
                return None;
            };
            let mut ip = path;
            ip.push(0);
            Some(ir::NumExpr::Abs(Box::new(num_expr(cx, inner, used, ip)?)))
        }
        // %INT, %INTH of a numeric expression (S2.14).
        Expr::Bif(b, args) if matches!(b.as_str(), "INT" | "INTH") => {
            let [inner] = args.as_slice() else {
                return None;
            };
            let mut ip = path.clone();
            ip.push(0);
            Some(ir::NumExpr::Int {
                inner: Box::new(num_expr(cx, inner, used, ip)?),
                half: b == "INTH",
                path,
            })
        }
        // %DEC, %DECH of a numeric expression, with digits and decimal
        // places that are literals, or none (S2.14).
        Expr::Bif(b, args) if matches!(b.as_str(), "DEC" | "DECH") => {
            let (inner, to) = match args.as_slice() {
                [i] => (i, None),
                [i, Expr::Num(p), Expr::Num(s)] => {
                    let p = u32::try_from(p.to_i128().ok()?).ok()?;
                    let s = u32::try_from(s.to_i128().ok()?).ok()?;
                    u8::try_from(s).ok()?;
                    (i, Some((p, s)))
                }
                _ => return None,
            };
            let mut ip = path.clone();
            ip.push(0);
            Some(ir::NumExpr::Dec {
                inner: Box::new(num_expr(cx, inner, used, ip)?),
                to,
                half: b == "DECH",
                path,
            })
        }
        // %CHECK, %CHECKR of character expressions with no start (S2.34):
        // the interpreter's interp.rs "CHECK" | "CHECKR" arm. With a start
        // they stay with the interpreter (upstream #16 changes its check).
        Expr::Bif(b, args) if matches!(b.as_str(), "CHECK" | "CHECKR") => {
            let [set, s] = args.as_slice() else {
                return None;
            };
            let mut ap = path.clone();
            ap.push(0);
            let mut sp = path.clone();
            sp.push(1);
            Some(ir::NumExpr::Check {
                set: Box::new(char_expr(cx, set, used, ap)?),
                s: Box::new(char_expr(cx, s, used, sp)?),
                rev: b == "CHECKR",
                path,
            })
        }
        // %SCAN of character expressions, with a start that is a numeric
        // literal or field (S2.12).
        Expr::Bif(b, args) if b == "SCAN" => {
            let (pat, s, start) = match args.as_slice() {
                [p, s] => (p, s, None),
                [p, s, st] => (p, s, Some(st)),
                _ => return None,
            };
            let mut pp = path.clone();
            pp.push(0);
            let mut sp = path.clone();
            sp.push(1);
            Some(ir::NumExpr::Scan {
                pat: Box::new(char_expr(cx, pat, used, pp)?),
                s: Box::new(char_expr(cx, s, used, sp)?),
                start: match start {
                    Some(x) => Some(operand(cx, x, used)?),
                    None => None,
                },
                path,
            })
        }
        _ => operand(cx, e, used).map(ir::NumExpr::Leaf),
    }
}

fn cond(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Cond> {
    cond_at(cx, e, used, Vec::new())
}

/// A condition at its place in the instruction's expression (0 left, 1
/// right from the root), which names the helpers of its sides.
fn cond_at(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>, path: Vec<u8>) -> Option<ir::Cond> {
    // An indicator, and NOT of a condition (S2.39): eval's
    // `Value::Ind(self.ind(n))` and `Value::Ind(!v.truth()?)`.
    match e {
        Expr::Ind(n) => return Some(ir::Cond::Ind(n.clone())),
        Expr::Unary(super::super::ast::UnOp::Not, x) => {
            let mut p = path.clone();
            p.push(0);
            return Some(ir::Cond::Not(Box::new(cond_at(cx, x, used, p)?)));
        }
        _ => {}
    }
    let Expr::Bin(op, l, r) = e else {
        return None;
    };
    let child = |k: u8| {
        let mut c = path.clone();
        c.push(k);
        c
    };
    // A chain of the same AND or OR down the left operands (S2.41).
    if matches!(op, BinOp::And | BinOp::Or) && matches!(&**l, Expr::Bin(o, _, _) if o == op) {
        let mut rev = Vec::new();
        let mut cur: &Expr = e;
        let mut p = path.clone();
        while let Expr::Bin(o, l2, r2) = cur {
            if o != op {
                break;
            }
            let mut rp = p.clone();
            rp.push(1);
            rev.push((&**r2, rp));
            cur = l2;
            p.push(0);
        }
        let mut conds = Vec::with_capacity(rev.len().saturating_add(1));
        conds.push(cond_at(cx, cur, used, p)?);
        for (r, rp) in rev.into_iter().rev() {
            conds.push(cond_at(cx, r, used, rp)?);
        }
        return Some(ir::Cond::Chain(*op == BinOp::And, conds));
    }
    let cmp = match op {
        BinOp::And | BinOp::Or => {
            let (a, b) = (
                cond_at(cx, l, used, child(0))?,
                cond_at(cx, r, used, child(1))?,
            );
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
    if operand_only(l) && operand_only(r) {
        let (a, b) = (any_operand(cx, l, used)?, any_operand(cx, r, used)?);
        // A figurative constant compares with either class (data::compare).
        let fig = |o: &ir::Operand| matches!(o, ir::Operand::Fig(_));
        let both_figs = fig(&a) && fig(&b);
        let ok = !both_figs && (fig(&a) || fig(&b) || is_char(&a) == is_char(&b));
        return ok.then_some(ir::Cond::Cmp(cmp, a, b));
    }
    // An expression on either side (S2.13).
    let (a, b) = (side(cx, l, used, child(0))?, side(cx, r, used, child(1))?);
    let same = matches!(
        (&a, &b),
        (ir::CmpSide::Num(_), ir::CmpSide::Num(_)) | (ir::CmpSide::Char(_), ir::CmpSide::Char(_))
    );
    same.then(|| ir::Cond::CmpExpr(cmp, Box::new(a), Box::new(b)))
}

/// Whether an expression is a literal or a name (an operand, as S2.1
/// compares them), not a built-in function or an operation.
fn operand_only(e: &Expr) -> bool {
    matches!(
        e,
        Expr::Num(_) | Expr::Str(_) | Expr::Name { .. } | Expr::Fig(_)
    )
}

/// The figurative constants lowered code takes (S2.22).
fn fig_op(f: &Fig) -> Option<ir::FigOp> {
    Some(match f {
        Fig::Blanks => ir::FigOp::Blanks,
        Fig::Zeros => ir::FigOp::Zeros,
        Fig::HiVal => ir::FigOp::HiVal,
        Fig::LoVal => ir::FigOp::LoVal,
        Fig::All(b) => ir::FigOp::All(b.clone()),
        // *NULL (S2.47): for pointers, in EVAL and comparisons.
        Fig::Null => ir::FigOp::Null,
        Fig::On | Fig::Off => return None,
    })
}

/// A side of a comparison: a character expression, or a numeric one with
/// no arithmetic (`+`, `-`, `*` take their precision from an EVAL's
/// expression, which a condition has not).
fn side(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>, path: Vec<u8>) -> Option<ir::CmpSide> {
    if let Some(c) = char_expr(cx, e, used, path.clone()) {
        return Some(ir::CmpSide::Char(c));
    }
    match num_expr(cx, e, used, path)? {
        ir::NumExpr::Bin { .. } => None,
        n => Some(ir::CmpSide::Num(n)),
    }
}

/// A literal or an eligible field, of either class.
fn any_operand(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    match e {
        Expr::Num(d) => {
            let m: i128 = d.mantissa().to_string().parse().ok()?;
            Some(ir::Operand::Num { m, s: d.scale() })
        }
        Expr::Str(b) => Some(ir::Operand::Str(b.clone())),
        // A named constant (S2.42): eval's `Sym::Const(c) => c`, its value
        // as defined, so the same operand as the literal it stands for.
        // The value goes into the routine's layout lines, so the
        // fingerprint changes with it: two programs whose code is the same
        // but whose constant differs (one from an include of another
        // source) must not share generated code (rla_test
        // include_search_order_follows_source_kind under native-diff).
        Expr::Name { parts, args: None }
            if matches!(cx.syms.get(&parts.join(".")), Some(Sym::Const(_))) =>
        {
            let name = parts.join(".");
            let op = match cx.syms.get(&name) {
                Some(Sym::Const(Value::Char(b))) => ir::Operand::Str(b.clone()),
                Some(Sym::Const(Value::Num(d))) => {
                    let m: i128 = d.mantissa().to_string().parse().ok()?;
                    ir::Operand::Num { m, s: d.scale() }
                }
                _ => return None,
            };
            used.push(format!("const {name} {op:?}"));
            Some(op)
        }
        // A figurative constant (S2.22); `operand` keeps it out of
        // numeric places, and only comparisons take it.
        Expr::Fig(f) => fig_op(f).map(ir::Operand::Fig),
        // %SIZE and %ELEM of a name (S2.25): constants, as the
        // interpreter's "SIZE" and "ELEM" arms work them out from the
        // definitions (bif_override changes %ELEM only for varying-dimension
        // arrays, which stay with the interpreter).
        Expr::Bif(b, args) if b == "SIZE" || b == "ELEM" => {
            let Some(Expr::Name { parts, args: None }) = args.first() else {
                return None;
            };
            let name = parts.join(".");
            let n: i128 = match (b.as_str(), cx.syms.get(&name)?) {
                ("SIZE", Sym::Var(v)) => {
                    let k = if args.len() > 1 { v.dim.max(1) } else { 1 };
                    i128::try_from(v.dtype.size())
                        .ok()?
                        .checked_mul(i128::from(k))?
                }
                ("SIZE", Sym::Ds(d)) => {
                    let k = if args.len() > 1 { d.dim.max(1) } else { 1 };
                    i128::try_from(d.size).ok()?.checked_mul(i128::from(k))?
                }
                ("SIZE", Sym::Const(Value::Num(d))) => {
                    let digits = d.int_digits().checked_add(u32::from(d.scale()))?.max(1);
                    i128::from(digits.checked_div(2)?.checked_add(1)?)
                }
                ("SIZE", Sym::Const(c)) => i128::try_from(c.chars().ok()?.len()).ok()?,
                ("ELEM", Sym::Var(v))
                    if args.len() == 1 && !cx.prog.varying.contains_key(&name) =>
                {
                    i128::from(v.dim)
                }
                ("ELEM", Sym::Ds(d)) if args.len() == 1 => i128::from(d.dim),
                _ => return None,
            };
            used.push(format!("{b} {name} {n}"));
            Some(ir::Operand::Num { m: n, s: 0 })
        }
        // %LEN of a fixed-length character field (S2.9): its length, a
        // constant (the interpreter's `type_of(arg).len()`; varying
        // fields are not eligible).
        Expr::Bif(b, args) if b == "LEN" => {
            let [arg] = args.as_slice() else {
                return None;
            };
            match variable(cx, arg, used)? {
                ir::Operand::Var {
                    ty: ir::Ty::Char(n),
                    ..
                } => Some(ir::Operand::Num {
                    m: i128::from(n),
                    s: 0,
                }),
                _ => None,
            }
        }
        _ => variable(cx, e, used),
    }
}

/// Whether an operand is character (else numeric).
fn is_char(o: &ir::Operand) -> bool {
    matches!(
        o,
        ir::Operand::Str(_)
            | ir::Operand::Var {
                ty: ir::Ty::Char(_) | ir::Ty::Varchar(_),
                ..
            }
    )
}

/// A numeric literal or an eligible numeric field.
fn operand(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    let o = any_operand(cx, e, used)?;
    (!is_char(&o) && !is_pointer(&o) && !matches!(o, ir::Operand::Fig(_))).then_some(o)
}

/// A pointer field (S2.47).
fn is_pointer(o: &ir::Operand) -> bool {
    matches!(
        o,
        ir::Operand::Var {
            ty: ir::Ty::Pointer,
            ..
        }
    )
}

/// An eligible numeric field.
fn num_variable(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    let o = variable(cx, e, used)?;
    (!is_char(&o) && !is_pointer(&o)).then_some(o)
}

/// An eligible field (ADR 0006): a global scalar of the main program,
/// numeric or fixed-length character, that is not an array, not based,
/// not in a multiple-occurrence data structure, not varying, not imported
/// or exported. File fields are eligible since program.rs gives them
/// buffer numbers in name order (S2.6).
fn variable(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    if let Expr::Path { .. } = e {
        return ds_array_field(cx, e, used);
    }
    let Expr::Name { parts, args } = e else {
        return None;
    };
    // A qualified subfield (`ds.field`, S2.16) is found under its parts
    // joined with periods, as the interpreter's `sym(&joined(parts))`.
    let joined = parts.join(".");
    let name = &joined;
    if parts.is_empty() {
        return None;
    }
    let v = match cx.syms.get(name) {
        Some(Sym::Var(v)) => v,
        // A whole data structure (S2.24), not an array, global, not based
        // or of multiple occurrences: the interpreter reads its bytes and
        // assigns it as a character field of its size (`eval`, `assign`).
        Some(Sym::Ds(d)) if args.is_none() => {
            let prog = cx.prog;
            if d.local
                || d.dim != 0
                || prog.based.contains_key(&d.buf)
                || prog.occurs.iter().any(|o| o.buf == d.buf)
                || prog.data_imports.iter().any(|(n, _)| n == name)
                || prog.data_exports.iter().any(|(n, _)| n == name)
            {
                return None;
            }
            let size = u32::try_from(d.size).ok()?;
            used.push(format!(
                "layout {name} {} {} ds {size} global",
                d.buf, d.off
            ));
            return Some(ir::Operand::Var {
                name: name.clone(),
                buf: d.buf,
                off: d.off,
                ty: ir::Ty::Char(size),
                local: false,
                index: None,
                based: None,
            });
        }
        _ => return None,
    };
    // An element of a global array with a literal index in range
    // (S2.15): a field at a fixed place, as the interpreter's `elem_off`
    // finds it (off + (index - 1) * stride). Not a table (TAB...), whose
    // name without an index is its current element.
    // An element with an index that is a field (S2.21): its place is
    // worked out when it runs (`rpgrt::native::elem_at`), as `elem_off`.
    let mut index = None;
    let elem = match args.as_deref() {
        None if v.dim == 0 => None,
        // A varying-dimension array's element is checked at run time
        // against its elements in use (S2.45), so a literal index takes
        // the run-time path below.
        Some([Expr::Num(k)])
            if v.dim > 0
                && !v.local
                && !name.starts_with("TAB")
                && !cx.prog.varying.contains_key(name) =>
        {
            let k = k.to_i128().ok()?;
            if k < 1 || k > i128::from(v.dim) {
                return None;
            }
            Some(usize::try_from(k).ok()?)
        }
        Some([ix]) if v.dim > 0 && !v.local && !name.starts_with("TAB") => {
            let of = operand(cx, ix, used)?;
            let vary = cx.prog.varying.get(name).map(|k| (name.clone(), *k == 'A'));
            if let Some((n, auto)) = &vary {
                used.push(format!("vary {n} auto={auto}"));
            }
            index = Some(Box::new(ir::Index {
                of,
                stride: v.stride,
                dim: v.dim,
                vary,
            }));
            Some(1)
        }
        _ => return None,
    };
    let off = match elem {
        Some(k) => v
            .off
            .checked_add(k.checked_sub(1)?.checked_mul(v.stride)?)?,
        None => v.off,
    };
    let prog = cx.prog;
    let ty = match v.dtype {
        DType::Packed(d, s) => ir::Ty::Packed(d, s),
        DType::Zoned(d, s) => ir::Ty::Zoned(d, s),
        DType::Binary(d, s) => ir::Ty::Binary(d, s),
        DType::Int(n) => ir::Ty::Int(n),
        DType::Uns(n) => ir::Ty::Uns(n),
        DType::Char(n) => ir::Ty::Char(n),
        DType::Varchar(n) => ir::Ty::Varchar(n),
        // A pointer field (S2.47): only where a pointer is taken (EVAL of
        // a pointer, comparisons, %ADDR), never as a number (`operand`).
        DType::Pointer => ir::Ty::Pointer,
        _ => return None,
    };
    // A based field (S2.48), not an array: where its pointer points now.
    // The pointer is found as the interpreter's based_place finds it
    // (`sym`, the procedure's names first), and must not be based itself.
    let based_on = if v.local {
        cx.based.and_then(|b| b.get(&v.buf))
    } else {
        prog.based.get(&v.buf)
    };
    let based = match based_on {
        Some(p) => {
            if v.dim != 0 || prog.occurs.iter().any(|o| o.buf == v.buf) {
                return None;
            }
            let ptr = variable(
                cx,
                &Expr::Name {
                    parts: vec![p.clone()],
                    args: None,
                },
                used,
            )?;
            if !matches!(
                &ptr,
                ir::Operand::Var {
                    ty: ir::Ty::Pointer,
                    index: None,
                    based: None,
                    ..
                }
            ) {
                return None;
            }
            used.push(format!("based {name} on {p}"));
            Some(Box::new(ir::Based {
                ptr,
                size: v.dtype.size(),
            }))
        }
        None => None,
    };
    let excluded = if v.local {
        // A procedure local (S2.8): only in a procedure (based ones above).
        v.dim != 0 || cx.based.is_none()
    } else {
        (v.dim != 0) != elem.is_some()
            || prog.occurs.iter().any(|o| o.buf == v.buf)
            // A varying-dimension array's elements only with a run-time
            // index (S2.45).
            || (prog.varying.contains_key(name) && index.is_none())
            || prog.data_imports.iter().any(|(n, _)| n == name)
            || prog.data_exports.iter().any(|(n, _)| n == name)
    };
    if excluded {
        return None;
    }
    let shown = match (&index, elem) {
        (Some(ix), _) => format!(
            "{name}({}) stride {} dim {}",
            match &ix.of {
                ir::Operand::Var { name, .. } => name.clone(),
                other => format!("{other:?}"),
            },
            ix.stride,
            ix.dim
        ),
        (None, Some(k)) => format!("{name}({k})"),
        (None, None) => name.clone(),
    };
    used.push(format!(
        "layout {shown} {} {off} {:?} {}",
        v.buf,
        v.dtype,
        if v.local { "local" } else { "global" }
    ));
    Some(ir::Operand::Var {
        name: shown,
        buf: v.buf,
        off,
        ty,
        local: v.local,
        index,
        based,
    })
}

/// A scalar subfield of an element of a global data structure array,
/// `ds(i).field` (S2.30): the interpreter's `path_sym` adds (i - 1) times
/// the data structure's size to the subfield's place, after the same
/// index check as an array element's (RNQ0121), so it is an element with
/// the data structure's size as its stride.
fn ds_array_field(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    let Expr::Path {
        parts,
        idx,
        args: None,
    } = e
    else {
        return None;
    };
    // The index is on the data structure: the parser gives `idx` one
    // entry per part, or only up to the last indexed part.
    let (ds, field, ix) = match (parts.as_slice(), idx.as_slice()) {
        ([ds, field], [Some(ix), None]) | ([ds, field], [Some(ix)]) => (ds, field, ix),
        _ => return None,
    };
    let prog = cx.prog;
    let Some(Sym::Ds(d)) = cx.syms.get(ds) else {
        return None;
    };
    if d.local
        || d.dim == 0
        || prog.based.contains_key(&d.buf)
        || prog.occurs.iter().any(|o| o.buf == d.buf)
    {
        return None;
    }
    let name = format!("{ds}.{field}");
    let Some(Sym::Var(v)) = cx.syms.get(&name) else {
        return None;
    };
    if v.local || v.dim != 0 || v.buf != d.buf || prog.varying.contains_key(&name) {
        return None;
    }
    let ty = match v.dtype {
        DType::Packed(d, s) => ir::Ty::Packed(d, s),
        DType::Zoned(d, s) => ir::Ty::Zoned(d, s),
        DType::Binary(d, s) => ir::Ty::Binary(d, s),
        DType::Int(n) => ir::Ty::Int(n),
        DType::Uns(n) => ir::Ty::Uns(n),
        DType::Char(n) => ir::Ty::Char(n),
        DType::Varchar(n) => ir::Ty::Varchar(n),
        _ => return None,
    };
    let of = operand(cx, ix, used)?;
    let shown = format!(
        "{ds}({}).{field} stride {} dim {}",
        match &of {
            ir::Operand::Var { name, .. } => name.clone(),
            other => format!("{other:?}"),
        },
        d.size,
        d.dim
    );
    used.push(format!(
        "layout {shown} {} {} {:?} global",
        v.buf, v.off, v.dtype
    ));
    Some(ir::Operand::Var {
        name: shown,
        buf: v.buf,
        off: v.off,
        ty,
        local: false,
        index: Some(Box::new(ir::Index {
            of,
            stride: d.size,
            dim: d.dim,
            vary: None,
        })),
        based: None,
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

/// A name with arguments that is a procedure of this module, not a field
/// or array (S2.35): the interpreter's `call_named` finds it in
/// `prog.procs` when no symbol has the name.
fn is_call(cx: &Cx<'_>, parts: &[String]) -> bool {
    let name = parts.join(".");
    parts.len() == 1
        && cx.syms.get(&name).is_none()
        && (cx.prog.procs.contains_key(&name) || proto(cx, &name).is_some())
}

/// A prototype in scope (S2.46): the procedure's own, then the program's
/// (the interpreter's `proto`).
fn proto<'p>(cx: &Cx<'p>, name: &str) -> Option<&'p super::super::program::Proto> {
    cx.protos
        .and_then(|p| p.get(name))
        .or_else(|| cx.prog.protos.get(name))
}

/// What a call of `name` returns: the procedure's, or the prototype's.
fn call_returns(cx: &Cx<'_>, name: &str) -> Option<DType> {
    match cx.prog.procs.get(name) {
        Some(pi) => pi.returns,
        None => proto(cx, name)?.returns,
    }
}

/// The generator's type for a parameter's type, when it is eligible.
fn param_ty(t: DType) -> Option<ir::Ty> {
    Some(match t {
        DType::Packed(d, s) => ir::Ty::Packed(d, s),
        DType::Zoned(d, s) => ir::Ty::Zoned(d, s),
        DType::Binary(d, s) => ir::Ty::Binary(d, s),
        DType::Int(n) => ir::Ty::Int(n),
        DType::Uns(n) => ir::Ty::Uns(n),
        DType::Char(n) => ir::Ty::Char(n),
        DType::Varchar(n) => ir::Ty::Varchar(n),
        _ => return None,
    })
}

/// A call of a procedure of this module that runs natively (S2.35, ADR
/// 0007): what `call_named`, `args_for` and `call_proc` do, for a
/// procedure with no *PSSR, ON-EXIT, pointer initializations or local
/// files, and parameters with no OPTIONS other than *NOPASS for ones
/// not passed. Arguments, in parameter order:
/// - by reference: a field or array element of the parameter's own type;
/// - CONST: a field (no index) of the parameter's own type by reference
///   (`same_type_variable`), anything else by value;
/// - VALUE: by value, a numeric expression for a numeric parameter or a
///   character one for a character parameter.
fn call_of(
    cx: &Cx<'_>,
    parts: &[String],
    args: &[Expr],
    used: &mut Vec<String>,
    path: Vec<u8>,
) -> Option<ir::Call> {
    if parts.len() != 1 || cx.syms.get(&parts.join(".")).is_some() {
        return None;
    }
    let name = parts.join(".");
    let prog = cx.prog;
    // A procedure of this module (S2.35), or a prototype (S2.46): a
    // program (EXTPGM named by a literal or a constant), or a procedure
    // bound into the program; not a procedure pointer, an overload, a CEE
    // API or a C runtime function, which keep their own paths.
    let mut layout = format!("call {name}:");
    let (params, local) = match prog.procs.get(&name) {
        Some(pi) => (&pi.params, Some(pi)),
        None => {
            let pr = proto(cx, &name)?;
            if !pr.overload.is_empty() || pr.extproc_ptr.is_some() {
                return None;
            }
            let local = match &pr.extpgm {
                Some(pgm) => {
                    if matches!(cx.syms.get(pgm), Some(Sym::Var(_))) {
                        return None;
                    }
                    None
                }
                None => {
                    let target = pr.extproc.clone().unwrap_or_else(|| name.clone());
                    let up = target.to_ascii_uppercase();
                    if super::super::interp::crt::C_FUNCTIONS.contains(&up.as_str())
                        || super::super::interp::cee::CEE_APIS.contains(&up.as_str())
                    {
                        return None;
                    }
                    prog.procs.get(&target)
                }
            };
            let _ = std::fmt::Write::write_fmt(
                &mut layout,
                format_args!(" proto extpgm={:?} extproc={:?}", pr.extpgm, pr.extproc),
            );
            (&pr.params, local)
        }
    };
    if let Some(pi) = local {
        if pi.routines.subroutines.contains_key("*PSSR")
            || pi.on_exit.is_some()
            || !pi.ptr_inz.is_empty()
            || prog
                .files
                .iter()
                .any(|f| f.decl.proc.as_deref() == Some(pi.name.as_str()))
        {
            return None;
        }
    }
    if args.len() > params.len() || params.iter().skip(args.len()).any(|p| !p.nopass) {
        return None;
    }
    let mut out = Vec::with_capacity(args.len());
    for (k, (p, a)) in params.iter().zip(args).enumerate() {
        if p.file || p.omit || p.string || p.varsize || p.trim || p.convert {
            return None;
        }
        // A parameter that is a data structure (LIKEDS), or a whole data
        // structure as the argument: the interpreter may pass it by
        // reference (same_type_variable) where a value would be built
        // here; not lowered (OpenRPG test325 under native-corpus.sh).
        if local.is_some_and(|pi| !matches!(pi.syms.get(&p.name), Some(Sym::Var(_))))
            || matches!(a, Expr::Name { parts, .. } if matches!(cx.syms.get(&parts.join(".")), Some(Sym::Ds(_))))
        {
            return None;
        }
        let ty = param_ty(p.dtype)?;
        let _ = std::fmt::Write::write_fmt(
            &mut layout,
            format_args!(" {}={:?}/{}/{}", p.name, ty, p.by_value, p.konst),
        );
        let mut ap = path.clone();
        ap.push(u8::try_from(k).ok()?);
        let field = |a: &Expr| match a {
            Expr::Name { parts, .. } => matches!(cx.syms.get(&parts.join(".")), Some(Sym::Var(_))),
            _ => false,
        };
        let by_ref = !p.by_value && !p.konst;
        let const_ref =
            p.konst && !p.by_value && matches!(a, Expr::Name { args: None, .. }) && field(a);
        if by_ref || const_ref {
            if !field(a) {
                return None;
            }
            let v = variable(cx, a, used)?;
            match &v {
                ir::Operand::Var { ty: vt, .. } if *vt == ty => {}
                // A CONST field of another type goes by value.
                _ if const_ref => {
                    out.push(value_arg(cx, &ty, a, used, ap)?);
                    continue;
                }
                _ => return None,
            }
            out.push(ir::CallArg::Ref(v));
        } else {
            out.push(value_arg(cx, &ty, a, used, ap)?);
        }
    }
    used.push(layout);
    Some(ir::Call {
        proc: name,
        args: out,
        path,
    })
}

/// A VALUE or CONST argument passed as a value: the interpreter's
/// `eval` of it, numeric for a numeric parameter and character for a
/// character one (no conversion between them, no hexadecimal literal).
fn value_arg(
    cx: &Cx<'_>,
    ty: &ir::Ty,
    a: &Expr,
    used: &mut Vec<String>,
    path: Vec<u8>,
) -> Option<ir::CallArg> {
    if matches!(a, Expr::Hex(_) | Expr::Omit) {
        return None;
    }
    match ty {
        ir::Ty::Char(_) | ir::Ty::Varchar(_) => {
            Some(ir::CallArg::Char(char_expr(cx, a, used, path)?))
        }
        _ => Some(ir::CallArg::Num(num_expr(cx, a, used, path)?)),
    }
}

/// AS4TAINR_NATIVE_WHY=<file>: the shape of each EVAL that is not lowered,
/// one line each (the target's kind and type, then the expression's
/// nodes), to find the forms that keep code in the interpreter (ADR 0007,
/// phase A). A diagnostic: it does not change what is generated. At most
/// 100 000 lines per process.
fn note_why(cx: &Cx<'_>, i: &Ins) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static LINES: AtomicUsize = AtomicUsize::new(0);
    let Ok(path) = std::env::var("AS4TAINR_NATIVE_WHY") else {
        return;
    };
    let line =
        match i {
            Ins::Stmt(Stmt::Eval { target, value, .. }) => {
                let mut nodes = Vec::new();
                shape(cx, value, &mut nodes, 0);
                nodes.sort();
                nodes.dedup();
                format!(
                    "target={} value={}\n",
                    target_shape(cx, target),
                    nodes.join(",")
                )
            }
            // A call not lowered: a procedure of this module (and what keeps
            // it out), or a prototype of something else (a program, a
            // service program's procedure).
            Ins::Stmt(Stmt::CallP(Expr::Name { parts, .. })) => {
                let name = parts.join(".");
                match cx.prog.procs.get(&name) {
                    Some(pi) => {
                        let mut why = Vec::new();
                        if pi.routines.subroutines.contains_key("*PSSR") {
                            why.push("pssr");
                        }
                        if pi.on_exit.is_some() {
                            why.push("on-exit");
                        }
                        if !pi.ptr_inz.is_empty() {
                            why.push("ptr-inz");
                        }
                        if pi.params.iter().any(|p| {
                            p.file || p.omit || p.string || p.varsize || p.trim || p.convert
                        }) {
                            why.push("options");
                        }
                        if pi.params.iter().any(|p| param_ty(p.dtype).is_none()) {
                            why.push("param-type");
                        }
                        if pi
                            .params
                            .iter()
                            .any(|p| !matches!(pi.syms.get(&p.name), Some(Sym::Var(_))))
                        {
                            why.push("ds-param");
                        }
                        if why.is_empty() {
                            why.push("arguments");
                        }
                        format!("callp=local why={}\n", why.join(","))
                    }
                    None => "callp=proto\n".to_string(),
                }
            }
            Ins::Stmt(Stmt::Dsply { message, response }) => {
                let mut nodes = Vec::new();
                if let Some(m) = message {
                    shape(cx, m, &mut nodes, 0);
                }
                nodes.sort();
                nodes.dedup();
                format!(
                    "dsply response={} value={}\n",
                    response.is_some(),
                    nodes.join(",")
                )
            }
            _ => return,
        };
    if LINES.fetch_add(1, Ordering::Relaxed) >= 100_000 {
        return;
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = std::io::Write::write_all(&mut f, line.as_bytes());
    }
}

/// What kind of target an EVAL has, and its type.
fn target_shape(cx: &Cx<'_>, e: &Expr) -> String {
    match e {
        Expr::Name { parts, args } => {
            let k = if args.is_some() { "elem" } else { "name" };
            match cx.syms.get(&parts.join(".")) {
                Some(Sym::Var(v)) => format!("{k}:{}", type_name(v.dtype)),
                Some(Sym::Ds(_)) => format!("{k}:ds"),
                Some(_) => format!("{k}:other"),
                None => format!("{k}:unknown"),
            }
        }
        Expr::Path { parts, idx, args } => {
            let first = parts.first().map(String::as_str).unwrap_or("");
            let ds = match cx.syms.get(first) {
                Some(Sym::Ds(d)) => format!(
                    "ds{}{}",
                    if d.local { "-local" } else { "" },
                    if d.dim != 0 { "-arr" } else { "" }
                ),
                Some(Sym::Var(_)) => "var".to_string(),
                Some(_) => "other".to_string(),
                None => "unknown".to_string(),
            };
            let ix: Vec<&str> = idx
                .iter()
                .map(|i| match i.as_deref() {
                    None => "-",
                    Some(Expr::Num(_)) => "num",
                    Some(Expr::Name { args: None, .. }) => "name",
                    Some(_) => "expr",
                })
                .collect();
            format!(
                "path:{}:{ds}:[{}]{}",
                parts.len(),
                ix.join(" "),
                if args.is_some() { ":args" } else { "" }
            )
        }
        Expr::Ind(_) => "ind".to_string(),
        Expr::IndArray(_) => "indarray".to_string(),
        Expr::Bif(b, _) => format!("bif:%{b}"),
        _ => "other".to_string(),
    }
}

fn type_name(t: DType) -> String {
    let s = format!("{t:?}");
    s.split('(').next().unwrap_or("").to_string()
}

/// The kinds of node in an expression (bounded depth).
fn shape(cx: &Cx<'_>, e: &Expr, out: &mut Vec<String>, depth: u32) {
    if depth > 32 {
        out.push("deep".to_string());
        return;
    }
    let d = depth.saturating_add(1);
    match e {
        Expr::Num(_) => out.push("num".into()),
        Expr::Str(_) => out.push("str".into()),
        Expr::Hex(_) => out.push("hex".into()),
        Expr::Fig(_) => out.push("fig".into()),
        Expr::Ind(_) | Expr::IndArray(_) => out.push("ind".into()),
        Expr::Omit => out.push("omit".into()),
        Expr::Unary(op, x) => {
            out.push(format!("unary:{op:?}"));
            shape(cx, x, out, d);
        }
        Expr::Bin(op, l, r) => {
            out.push(format!("bin:{op:?}"));
            shape(cx, l, out, d);
            shape(cx, r, out, d);
        }
        Expr::Bif(b, args) => {
            out.push(format!("%{b}"));
            for a in args {
                shape(cx, a, out, d);
            }
        }
        Expr::Path { .. } => out.push("path".into()),
        Expr::Name { parts, args } => {
            let n = parts.join(".");
            match (cx.syms.get(&n), args) {
                (Some(Sym::Var(v)), None) => out.push(format!("var:{}", type_name(v.dtype))),
                (Some(Sym::Var(v)), Some(a)) => {
                    out.push(format!("elem:{}", type_name(v.dtype)));
                    for x in a {
                        shape(cx, x, out, d);
                    }
                }
                (Some(Sym::Ds(_)), _) => out.push("ds".into()),
                (Some(Sym::Const(_)), _) => out.push("const".into()),
                (Some(_), _) => out.push("sym:other".into()),
                (None, Some(a)) if cx.prog.procs.contains_key(&n) => {
                    out.push("call:local".into());
                    for x in a {
                        shape(cx, x, out, d);
                    }
                }
                (None, Some(a)) => {
                    out.push("call:proto".into());
                    for x in a {
                        shape(cx, x, out, d);
                    }
                }
                (None, None) => out.push("name:unknown".into()),
            }
        }
    }
}

/// The generator's operation for +, - or * (S2.41's chains).
fn ar_op(op: &BinOp) -> ir::ArOp {
    match op {
        BinOp::Add => ir::ArOp::Add,
        BinOp::Sub => ir::ArOp::Sub,
        _ => ir::ArOp::Mul,
    }
}

/// A name that is a pointer field (S2.47).
fn is_pointer_name(cx: &Cx<'_>, e: &Expr) -> bool {
    match e {
        Expr::Name { parts, .. } => {
            matches!(cx.syms.get(&parts.join(".")), Some(Sym::Var(v)) if v.dtype == DType::Pointer)
        }
        _ => false,
    }
}

/// A pointer value (S2.47, S2.49): a pointer field, %ADDR of a global
/// field (ptr_to of its place; *DATA skips a varying field's length), or
/// either moved by numbers (`p + n`, `p - n`, left to right: ptrs.rs
/// ptr_arith). A number plus a pointer, and a pointer less a pointer, are
/// not lowered.
fn ptr_val(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::PtrVal> {
    // The left spine of + and - with a number on the right, walked with a
    // loop (no recursion).
    let mut steps = Vec::new();
    let mut at = e;
    while let Expr::Bin(op @ (BinOp::Add | BinOp::Sub), l, r) = at {
        if is_pointer_name(cx, r) {
            return None;
        }
        steps.push((*op == BinOp::Sub, r.as_ref()));
        at = l;
        if steps.len() > MAX_NESTING {
            return None;
        }
    }
    let base = match at {
        Expr::Bif(b, args) if b == "ADDR" => {
            let (of, data) = match args.as_slice() {
                [x] => (x, false),
                [x, Expr::Name { parts, args: None }] if parts.as_slice() == ["*DATA"] => (x, true),
                _ => return None,
            };
            let of = variable(cx, of, used)?;
            let ir::Operand::Var {
                local: false,
                based: None,
                ty,
                ..
            } = &of
            else {
                return None;
            };
            let skip = match (data, ty) {
                (true, ir::Ty::Varchar(n)) => rpgrt::rpg::data::var_prefix(*n),
                _ => 0,
            };
            ir::PtrVal::Addr { of, skip }
        }
        v if is_pointer_name(cx, v) => ir::PtrVal::Var(variable(cx, v, used)?),
        _ => return None,
    };
    if steps.is_empty() {
        return Some(base);
    }
    steps.reverse();
    let mut by = Vec::with_capacity(steps.len());
    for (k, (neg, r)) in steps.into_iter().enumerate() {
        let path = vec![u8::try_from(k).ok()?];
        by.push((neg, num_expr(cx, r, used, path)?));
    }
    Some(ir::PtrVal::Move {
        base: Box::new(base),
        by,
    })
}
