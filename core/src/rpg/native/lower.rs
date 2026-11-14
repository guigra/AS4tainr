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
        }),
        Place::Proc(name, _) => prog.procs.get(name).map(|pi| Cx {
            prog,
            syms: &pi.syms,
            based: Some(&pi.based),
        }),
    };
    let mut layout = Vec::new();
    let ins = code
        .iter()
        .map(|i| {
            let (kind, what) = kind(&i.ins);
            let lowered = cx.as_ref().and_then(|cx| lowered(cx, &i.ins, &mut layout));
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

/// What lowering looks names up in.
struct Cx<'p> {
    prog: &'p Program,
    syms: &'p super::super::program::SymMap,
    /// A procedure's based locals (frame buffer → pointer).
    based: Option<&'p std::collections::HashMap<usize, String>>,
}

/// What an instruction evaluates, lowered, when all of it is eligible.
fn lowered(cx: &Cx<'_>, i: &Ins, layout: &mut Vec<String>) -> Option<ir::Lowered> {
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
        }) if ind.is_empty() && !matches!(from, Expr::Fig(_)) => ir::Lowered::Move {
            left: *left,
            pad: *pad,
            from: any_operand(cx, from, &mut used)?,
            to: variable(cx, to, &mut used)?,
        },
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
    let Expr::Bin(op, l, r) = e else {
        return None;
    };
    let child = |k: u8| {
        let mut c = path.clone();
        c.push(k);
        c
    };
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
        Fig::On | Fig::Off | Fig::Null => return None,
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
    (!is_char(&o) && !matches!(o, ir::Operand::Fig(_))).then_some(o)
}

/// An eligible numeric field.
fn num_variable(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    let o = variable(cx, e, used)?;
    (!is_char(&o)).then_some(o)
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
        Some([Expr::Num(k)]) if v.dim > 0 && !v.local && !name.starts_with("TAB") => {
            let k = k.to_i128().ok()?;
            if k < 1 || k > i128::from(v.dim) {
                return None;
            }
            Some(usize::try_from(k).ok()?)
        }
        Some([ix]) if v.dim > 0 && !v.local && !name.starts_with("TAB") => {
            let of = operand(cx, ix, used)?;
            index = Some(Box::new(ir::Index {
                of,
                stride: v.stride,
                dim: v.dim,
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
        _ => return None,
    };
    let excluded = if v.local {
        // A procedure local (S2.8): not based, and only in a procedure.
        v.dim != 0 || cx.based.is_none_or(|b| b.contains_key(&v.buf))
    } else {
        (v.dim != 0) != elem.is_some()
            || prog.based.contains_key(&v.buf)
            || prog.occurs.iter().any(|o| o.buf == v.buf)
            || prog.varying.contains_key(name)
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
    let ([ds, field], [Some(ix), None]) = (parts.as_slice(), idx.as_slice()) else {
        return None;
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
        })),
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
