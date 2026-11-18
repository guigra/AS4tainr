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
            params: None,
        }),
        Place::Proc(name, _) => prog.procs.get(name).map(|pi| Cx {
            prog,
            syms: &pi.syms,
            based: Some(&pi.based),
            protos: Some(&pi.protos),
            params: Some(&pi.params),
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
                    note_why(cx, &i.ins, i.line);
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
    /// A procedure's parameters (S2.63: an array passed by reference is
    /// the caller's storage, which is not lowered as a local array).
    params: Option<&'p [super::super::program::ParamInfo]>,
}

/// A name as the interpreter's `Machine::sym` finds it (S2.61): in a
/// procedure, its own names first (locals, parameters, its prototypes'
/// names), then the module's globals; in the main program, the
/// program's. Based globals and subfields of multiple-occurrence data
/// structures are found here as declared; `variable` places them (or
/// refuses them) as it does in the main program.
fn sym<'c>(cx: &Cx<'c>, name: &str) -> Option<&'c Sym> {
    cx.syms.get(name).or_else(|| match cx.based {
        Some(_) => cx.prog.syms.get(name),
        None => None,
    })
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
        // FOR-EACH over a whole global array of fixed dimension (S2.89):
        // list_of's array, the EachNext arm's assignment to a field.
        Ins::EachInit {
            list: Expr::Name { parts, args: None },
        } => {
            let name = parts.join(".");
            let Some(Sym::Var(v)) = sym(cx, &name) else {
                return None;
            };
            if v.dim == 0 || v.local || cx.prog.varying.contains_key(&name) {
                return None;
            }
            let ty = global_elem_ty(v.dtype, true)?;
            used.push(format!("each {name} stride {} dim {}", v.stride, v.dim));
            ir::Lowered::EachArray {
                arr: (v.buf, v.off, v.stride, v.dim, ty),
            }
        }
        Ins::EachNext { var, .. } => {
            let o = variable(cx, var, &mut used)?;
            if !matches!(o, ir::Operand::Var { based: None, .. }) {
                return None;
            }
            ir::Lowered::EachNext { var: o }
        }
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
                    let Some(Sym::Var(v)) = sym(cx, name) else {
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
        // (S2.11), and no date, time or timestamp field (S2.52), so
        // `move_temporal` does nothing for them.
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
            let from = any_operand(cx, from, &mut used)?;
            let to = variable(cx, to, &mut used)?;
            if is_temporal(&from) || is_temporal(&to) {
                return None;
            }
            ir::Lowered::Move {
                left: *left,
                pad: *pad,
                from,
                to,
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
            let Some(Sym::Var(v)) = sym(cx, &name) else {
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
            if !is_char(&base) || is_temporal(&base) {
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
                v => ir::IndValue::Cond(Box::new(cond_at(cx, v, &mut used, Vec::new())?)),
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
        // EVAL %OCCUR(ds) = n (S2.64): interp.rs assign's "OCCUR" arm,
        // set_occurrence with the value's whole number.
        Ins::Stmt(Stmt::Eval {
            target: Expr::Bif(b, args),
            value,
            ext,
            ..
        }) if b == "OCCUR" && args.len() == 1 && !ext.error && ext.other.is_empty() => {
            let ir::Operand::Occur { buf, .. } = occurs_ds(cx, args.first()?, &mut used)? else {
                return None;
            };
            ir::Lowered::SetOccur {
                buf,
                to: Some(num_expr(cx, value, &mut used, Vec::new())?),
                result: None,
            }
        }
        // OCCUR (S2.64): ops.rs occur_op. Factor 1 (a number, or another
        // multiple-occurrence data structure's current occurrence) sets
        // the occurrence; the result field gets it. No error indicator.
        Ins::Stmt(Stmt::Op {
            code,
            f1,
            f2: Some(ds),
            result,
            ext,
            ind,
        }) if code == "OCCUR" && !ext.error && ext.other.is_empty() && ind.is_empty() => {
            let ir::Operand::Occur { buf, .. } = occurs_ds(cx, ds, &mut used)? else {
                return None;
            };
            let to = match f1 {
                None => None,
                Some(e @ Expr::Name { parts, args: None })
                    if matches!(sym(cx, &parts.join(".")), Some(Sym::Ds(_))) =>
                {
                    Some(ir::NumExpr::Leaf(occurs_ds(cx, e, &mut used)?))
                }
                Some(e) => Some(num_expr(cx, e, &mut used, Vec::new())?),
            };
            let result = match result {
                None => None,
                Some(r) => Some(num_variable(cx, r, &mut used)?),
            };
            ir::Lowered::SetOccur { buf, to, result }
        }
        // SORTA of a whole global array (S2.86): ops.rs sorta.
        Ins::Stmt(Stmt::Op {
            code,
            f1: None,
            f2: Some(Expr::Name { parts, args: None }),
            result: None,
            ext,
            ind,
        }) if code == "SORTA" && !ext.error && ext.other.is_empty() && ind.is_empty() => {
            let name = parts.join(".");
            let Some(Sym::Var(v)) = sym(cx, &name) else {
                return None;
            };
            if v.dim == 0 || v.local || cx.prog.varying.contains_key(&name) {
                return None;
            }
            let ty = global_elem_ty(v.dtype, true)?;
            let descend = cx.prog.sequence.get(&name) == Some(&'D');
            used.push(format!(
                "sorta {name} stride {} dim {} lead {} descend {descend}",
                v.stride, v.dim, v.lead
            ));
            ir::Lowered::Sorta {
                arr: (v.buf, v.off, v.stride, v.dim, ty, v.lead),
                descend,
            }
        }
        // MOVEA of character data (S2.86): ops.rs movea, factor 2 and the
        // result fixed-length character arrays or fields (factor 2 also a
        // character literal); numbers (zoned images) are not lowered.
        Ins::Stmt(Stmt::Op {
            code,
            f1: None,
            f2: Some(f2),
            result: Some(r),
            ext,
            ind,
        }) if code == "MOVEA"
            && !ext.error
            && ext.other.chars().all(|c| c == 'P')
            && ind.is_empty() =>
        {
            let from = movea_side(cx, f2, true, &mut used)?;
            let to = movea_side(cx, r, false, &mut used)?;
            used.push("movea chars".into());
            ir::Lowered::Movea {
                from,
                to,
                pad: ext.other.contains('P'),
            }
        }
        // Fixed-form LOOKUP of an array and XFOOT (S2.85): ops.rs lookup
        // and array_op's XFOOT.
        Ins::Stmt(Stmt::Op {
            code,
            f1,
            f2: Some(f2),
            result,
            ext,
            ind,
        }) if (code == "LOOKUP" || code == "XFOOT")
            && !ext.error
            // Ext::other holds the whole extender: H for XFOOT only.
            && (ext.other.is_empty() || (code == "XFOOT" && ext.other == "H")) =>
        {
            let Expr::Name { parts, args } = f2 else {
                return None;
            };
            let aname = parts.join(".");
            let Some(Sym::Var(v)) = sym(cx, &aname) else {
                return None;
            };
            if v.dim == 0 || v.local || cx.prog.varying.contains_key(&aname) {
                return None;
            }
            let ety = match v.dtype {
                DType::Packed(d, s) => ir::Ty::Packed(d, s),
                DType::Zoned(d, s) => ir::Ty::Zoned(d, s),
                DType::Binary(d, s) => ir::Ty::Binary(d, s),
                DType::Int(n) => ir::Ty::Int(n),
                DType::Uns(n) => ir::Ty::Uns(n),
                DType::Char(n) if code == "LOOKUP" => ir::Ty::Char(n),
                _ => return None,
            };
            let arr = ir::Operand::Var {
                name: aname.clone(),
                buf: v.buf,
                off: v.off,
                ty: ety,
                local: false,
                index: None,
                based: None,
            };
            let (lookup, xfoot) = if code == "LOOKUP" {
                // An array, not a table (TAB...: its current element is the
                // interpreter's state); no alternating array.
                if aname.starts_with("TAB") || result.is_some() {
                    return None;
                }
                let arg = match f1.as_ref()? {
                    Expr::Str(b) | Expr::Hex(b) => ir::Operand::Str(b.clone()),
                    e @ Expr::Num(_) => operand(cx, e, &mut used)?,
                    e @ Expr::Name { .. } => {
                        let o = sql_field(cx, e, &mut used)?;
                        if !matches!(
                            o,
                            ir::Operand::Var {
                                index: None,
                                based: None,
                                ..
                            }
                        ) {
                            return None;
                        }
                        o
                    }
                    _ => return None,
                };
                let ix = match args.as_deref() {
                    None => None,
                    Some([i]) => Some(
                        str_num(cx, i, &mut used)
                            .filter(|o| matches!(o, ir::Operand::Var { .. }))?,
                    ),
                    Some(_) => return None,
                };
                (Some((arg, ix)), None)
            } else {
                if args.is_some() || f1.is_some() {
                    return None;
                }
                let r = str_num(cx, result.as_ref()?, &mut used)?;
                if !matches!(r, ir::Operand::Var { .. }) {
                    return None;
                }
                (None, Some((r, ext.half_adjust)))
            };
            used.push(format!(
                "array op {code} {aname} stride {} dim {}",
                v.stride, v.dim
            ));
            ir::Lowered::ArrOp(Box::new(ir::ArrOpData {
                lookup,
                xfoot,
                arr,
                stride: v.stride,
                dim: v.dim,
                hi: ind.hi.clone(),
                lo: ind.lo.clone(),
                eq: ind.eq.clone(),
            }))
        }
        // Fixed-form TESTN, TESTZ, TESTB, BITON and BITOFF (S2.84): ops.rs
        // bit_op on a character field.
        Ins::Stmt(Stmt::Op {
            code,
            f1: _,
            f2,
            result: Some(r),
            ext,
            ind,
        }) if matches!(
            code.as_str(),
            "TESTN" | "TESTZ" | "TESTB" | "BITON" | "BITOFF"
        ) && !ext.error
            && ext.other.is_empty() =>
        {
            let c = match code.as_str() {
                "TESTN" => ir::BitCode::TestN,
                "TESTZ" => ir::BitCode::TestZ,
                "TESTB" => ir::BitCode::TestB,
                "BITON" => ir::BitCode::BitOn,
                _ => ir::BitCode::BitOff,
            };
            let target = str_chars(cx, r, &mut used)?;
            if !matches!(target, ir::Operand::Var { .. }) {
                return None;
            }
            // ops.rs bit_mask: a literal of bit numbers 0-7 (another digit
            // is the interpreter's error), or a field's first byte.
            let (mask, mask_field) = match c {
                ir::BitCode::TestN | ir::BitCode::TestZ => (None, None),
                _ => match f2.as_ref()? {
                    Expr::Str(b) => {
                        let mut m = 0u8;
                        for ch in super::super::data::a(b).chars() {
                            let n = ch.to_digit(10).filter(|n| *n <= 7)?;
                            m |= 0x80u8.checked_shr(n)?;
                        }
                        (Some(m), None)
                    }
                    e @ Expr::Name { .. } => (None, Some(str_chars(cx, e, &mut used)?)),
                    _ => return None,
                },
            };
            used.push(format!("bit op {code}"));
            ir::Lowered::BitOp(Box::new(ir::BitOpData {
                code: c,
                target,
                mask,
                mask_field,
                hi: ind.hi.clone(),
                lo: ind.lo.clone(),
                eq: ind.eq.clone(),
            }))
        }
        // Fixed-form SCAN, CHECK, CHECKR, SUBST, XLATE and CAT (S2.83):
        // ops.rs string_op on the factors as split splits them.
        Ins::Stmt(Stmt::Op {
            code,
            f1,
            f2,
            result,
            ext,
            ind,
        }) if matches!(
            code.as_str(),
            "SCAN" | "CHECK" | "CHECKR" | "SUBST" | "XLATE" | "CAT"
        ) && !ext.error
            && ext.other.chars().all(|c| c == 'P') =>
        {
            str_op(cx, code, f1, f2, result, ext, ind, &mut used)?
        }
        // IN and OUT of data areas (S2.80): interp.rs data_area for the
        // field or, for *DTAARA, every data area of the program in name
        // order; the fields (and *VAR: name fields) placed.
        Ins::Stmt(Stmt::Op {
            code,
            f1,
            f2,
            result: None,
            ext,
            ind,
        }) if (code == "IN" || code == "OUT")
            && !ext.error
            && ext.other.is_empty()
            && ind.is_empty() =>
        {
            let lock = except_name(f1).as_deref() == Some("*LOCK");
            let target = except_name(f2)?;
            let names: Vec<String> = if target == "*DTAARA" {
                let mut v: Vec<String> = cx.prog.dtaaras.keys().cloned().collect();
                v.sort();
                v
            } else {
                vec![target]
            };
            let mut fields = Vec::new();
            for n in names {
                let def = cx.prog.dtaaras.get(&n)?;
                let field = data_area_field(cx, &n, &mut used)?;
                let var = match def.strip_prefix("*VAR:") {
                    Some(v) => {
                        let o = sql_field(cx, &Expr::name(v), &mut used)?;
                        if !is_char(&o) || !matches!(o, ir::Operand::Var { index: None, .. }) {
                            return None;
                        }
                        Some(o)
                    }
                    None => None,
                };
                fields.push((n, field, var));
            }
            used.push(format!("{code} {} data areas lock={lock}", fields.len()));
            ir::Lowered::DataArea {
                input: code == "IN",
                lock,
                fields,
            }
        }
        // EXCEPT (S2.78): the exception records of that name printed by
        // the runtime (interp.rs except); the program fields their output
        // specifications name, placed.
        Ins::Stmt(Stmt::Op {
            code,
            f1: None,
            f2,
            result: None,
            ext,
            ind,
        }) if code == "EXCEPT" && !ext.error && ext.other.is_empty() && ind.is_empty() => {
            let name = except_name(f2);
            let fields = except_fields(cx, name.as_deref(), &mut used)?;
            used.push(format!("except {name:?} {} fields", fields.len()));
            ir::Lowered::Except { name, fields }
        }
        // EXEC SQL, step 1 (S2.69): a statement with no host variable that
        // exec_sql.rs's sql_statement runs without cursors, INTO, CALL,
        // dynamic SQL or GET DIAGNOSTICS; the SQLCA's fields as operands.
        Ins::Stmt(Stmt::Sql(text)) if sql_step1(text) => {
            let field = |n: &str, used: &mut Vec<String>| variable(cx, &Expr::name(n), used);
            let errd = |k: i64, used: &mut Vec<String>| {
                variable(
                    cx,
                    &Expr::Name {
                        parts: vec!["SQLERRD".into()],
                        args: Some(vec![Expr::Num(super::super::dec::Dec::from_i64(k))]),
                    },
                    used,
                )
            };
            let sqlca = [
                field("SQLCODE", &mut used)?,
                field("SQLSTATE", &mut used)?,
                field("SQLERRMC", &mut used)?,
                errd(3, &mut used)?,
                errd(5, &mut used)?,
            ];
            used.push("exec sql step 1".into());
            ir::Lowered::ExecSql {
                sqlca: Box::new(sqlca),
            }
        }
        // CALL of an SQL procedure (S2.75).
        Ins::Stmt(Stmt::Sql(text)) if sql_call_kind(text) => {
            let sqlca = sqlca_ops(cx, &mut used)?;
            sql_call(cx, text, sqlca, &mut used)?
        }
        // INSERT ... n ROWS VALUES (:dsarray) (S2.74).
        Ins::Stmt(Stmt::Sql(text)) if sql_insert_rows_kind(text) => {
            let sqlca = sqlca_ops(cx, &mut used)?;
            sql_insert_rows(cx, text, sqlca, &mut used)?
        }
        // UPDATE and DELETE WHERE CURRENT OF (S2.73).
        Ins::Stmt(Stmt::Sql(text)) if sql_positioned_kind(text) => {
            let sqlca = sqlca_ops(cx, &mut used)?;
            sql_positioned(cx, text, sqlca, &mut used)?
        }
        // Cursors (S2.72): OPEN, FETCH of one row, CLOSE.
        Ins::Stmt(Stmt::Sql(text)) if sql_cursor_kind(text) => {
            let sqlca = sqlca_ops(cx, &mut used)?;
            sql_cursor(cx, text, sqlca, &mut used)?
        }
        // Dynamic SQL and GET DIAGNOSTICS (S2.71).
        Ins::Stmt(Stmt::Sql(text)) if sql_dyn_kind(text).is_some() => {
            let sqlca = sqlca_ops(cx, &mut used)?;
            match sql_dyn_kind(text)? {
                SqlKind::Diagnostics => {
                    let (condition, items) = sql_diag(cx, text, &mut used)?;
                    used.push(format!("get diagnostics {} items", items.len()));
                    ir::Lowered::SqlDiag {
                        sqlca: Box::new(sqlca),
                        condition,
                        items,
                    }
                }
                SqlKind::Dyn(kind) => {
                    let (name, stmt_text, inputs) = sql_dyn(cx, text, kind, &mut used)?;
                    used.push(format!("dynamic sql {kind:?} {name}"));
                    ir::Lowered::ExecSqlDyn {
                        sqlca: Box::new(sqlca),
                        kind,
                        name,
                        text: stmt_text,
                        inputs,
                    }
                }
            }
        }
        // EXEC SQL with host variables (S2.70): SELECT or VALUES INTO, SET
        // :host, and the statements sql_change runs, as exec_sql.rs binds
        // and assigns them.
        Ins::Stmt(Stmt::Sql(text)) if sql_hosts_kind(text) => {
            let (pieces, inputs, outputs, into) = sql_hosts(cx, text, &mut used)?;
            let field = |n: &str, used: &mut Vec<String>| variable(cx, &Expr::name(n), used);
            let errd = |k: i64, used: &mut Vec<String>| {
                variable(
                    cx,
                    &Expr::Name {
                        parts: vec!["SQLERRD".into()],
                        args: Some(vec![Expr::Num(super::super::dec::Dec::from_i64(k))]),
                    },
                    used,
                )
            };
            let sqlca = [
                field("SQLCODE", &mut used)?,
                field("SQLSTATE", &mut used)?,
                field("SQLERRMC", &mut used)?,
                errd(3, &mut used)?,
                errd(5, &mut used)?,
            ];
            used.push(format!(
                "exec sql hosts {} in {} out",
                inputs.len(),
                outputs.len()
            ));
            ir::Lowered::ExecSqlHosts {
                sqlca: Box::new(sqlca),
                pieces,
                inputs,
                outputs,
                into,
            }
        }
        // A read of a database file (S2.58).
        // OPEN, CLOSE and FEOD of a printer file (S2.79): errors.rs
        // file_op and printer.rs printer_op.
        Ins::Stmt(Stmt::File {
            op: op @ (FileOp::Open | FileOp::Close | FileOp::Feod),
            file,
            key: None,
            ds: None,
            fields: None,
            ext,
            ind,
        }) if !ext.error && ext.other.is_empty() && ind.is_empty() && printer_file(cx, file) => {
            used.push(format!("printer {op:?} {file}"));
            ir::Lowered::PrinterOp
        }
        // WRITE of an externally described printer file's record format
        // (S2.79): errors.rs file_op and printer.rs printer_write, with
        // the fields the format prints and the INDDS structure placed.
        Ins::Stmt(Stmt::File {
            op: FileOp::Write,
            file,
            key: None,
            ds: None,
            fields: None,
            ext,
            ind,
        }) if !ext.error
            && ext.other.is_empty()
            && ind.lo.is_none()
            && ind.hi.is_none()
            && ind.eq.is_none()
            && printer_format(cx, file).is_some() =>
        {
            let (fields, indds) = printer_fields(cx, file, &mut used)?;
            used.push(format!("printer write {file} {} fields", fields.len()));
            ir::Lowered::PrinterWrite { fields, indds }
        }
        Ins::Stmt(Stmt::File {
            op,
            file,
            key,
            ds: None,
            fields: None,
            ext,
            ind,
        }) if !ext.error && ext.other.is_empty() && ind.lo.is_none() => {
            file_read(cx, *op, file, key.as_deref(), &mut used)?
        }
        // DSPLY with no response field (S2.36): Stmt::Dsply's batch path.
        // A float, date or time message is not an eligible expression.
        Ins::Stmt(Stmt::Dsply {
            message,
            response: None,
        }) => ir::Lowered::Dsply(match message {
            None => None,
            // A date or time field shows in its format, %CHAR of it; a
            // timestamp field as its value (Stmt::Dsply, S2.53).
            Some(m @ Expr::Name { .. }) if is_temporal_name(cx, m) => {
                let o = variable(cx, m, &mut used)?;
                Some(ir::ValExpr::Char(match &o {
                    ir::Operand::Var {
                        ty: ir::Ty::Timestamp(_),
                        ..
                    } => ir::CharExpr::Leaf(o),
                    _ => temporal_char(cx, std::slice::from_ref(m), &mut used)?,
                }))
            }
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
            let value = ptr_val(cx, value, &mut used, Vec::new())?;
            ir::Lowered::EvalPtr { target: t, value }
        }
        // DEALLOC (S2.50), as ptrs.rs storage_op: a pointer field as the
        // result (free form: factor 2), (N) or no extender, no indicators.
        // ALLOC and REALLOC are not lowered: the parser does not take
        // them (fidelity.md).
        Ins::Stmt(Stmt::Op {
            code,
            f1: None,
            f2,
            result,
            ext,
            ind,
        }) if code == "DEALLOC" && !ext.error && ind.is_empty() => {
            let target = result.as_ref().or(f2.as_ref())?;
            if !is_pointer_name(cx, target) || !ext.other.chars().all(|c| c == 'N') {
                return None;
            }
            ir::Lowered::Dealloc {
                target: variable(cx, target, &mut used)?,
                null: ext.other.contains('N'),
            }
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
                    value: char_value(cx, value, &mut used)?,
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

/// A part of a concatenation (S2.62): a character expression, or an
/// indicator (*INxx, *INLR...), which the interpreter's + takes as
/// character data of length 1 ('1' or '0') next to characters or another
/// indicator (ILE RPG Reference, "Indicator Data Type"; interp.rs eval's
/// BinOp::Add). Every other part of a lowered concatenation is character
/// data, so the + is always a concatenation; an indicator plus a number,
/// a numeric sum, is not lowered here.
fn cat_part(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>, path: Vec<u8>) -> Option<ir::CharExpr> {
    match e {
        Expr::Ind(n) => {
            used.push(format!("ind {n}"));
            Some(ir::CharExpr::Ind(n.clone()))
        }
        _ => char_expr(cx, e, used, path),
    }
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
            parts.push(cat_part(cx, cur, used, p)?);
            for (r, rp) in rev.into_iter().rev() {
                parts.push(cat_part(cx, r, used, rp)?);
            }
            Some(ir::CharExpr::Chain { parts, path })
        }
        Expr::Bin(BinOp::Add, l, r) => {
            let mut lp = path.clone();
            lp.push(0);
            let mut rp = path.clone();
            rp.push(1);
            Some(ir::CharExpr::Cat {
                left: Box::new(cat_part(cx, l, used, lp)?),
                right: Box::new(cat_part(cx, r, used, rp)?),
                path,
            })
        }
        // %LEFT and %RIGHT of a character expression, with a length that is
        // a numeric literal or field (S2.67).
        Expr::Bif(b, args) if (b == "LEFT" || b == "RIGHT") && args.len() == 2 => {
            let mut sp = path.clone();
            sp.push(0);
            Some(ir::CharExpr::LeftRight {
                left: b == "LEFT",
                s: Box::new(char_expr(cx, args.first()?, used, sp)?),
                n: operand(cx, args.get(1)?, used)?,
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
        // %CHAR of a date, time or timestamp field (S2.53): bif_override's
        // form, with the format it picks known now.
        Expr::Bif(b, args) if b == "CHAR" && temporal_arg(cx, args) => {
            temporal_char(cx, args, used)
        }
        // %CHAR of a numeric expression (S2.14). The program's DECEDIT is
        // part of the code, so it goes into the fingerprint's layout.
        // %STR(p[: max]) (S2.88): a pointer value (ptr_val) and a number;
        // ptrs.rs str_at is the runtime's.
        Expr::Bif(b, args) if b == "STR" && matches!(args.len(), 1 | 2) => {
            let mut pp = path.clone();
            pp.push(0);
            let ptr = ptr_val(cx, args.first()?, used, pp)?;
            let max = match args.get(1) {
                Some(m) => {
                    let mut mp = path.clone();
                    mp.push(1);
                    Some(num_expr(cx, m, used, mp)?)
                }
                None => None,
            };
            Some(ir::CharExpr::StrAt {
                ptr: Box::new(ptr),
                max,
                path,
            })
        }
        // %CHAR of an indicator (S2.85): its character, '1' or '0', as
        // interp.rs bif's CHAR arm gives a Value::Ind (an indicator in a
        // concatenation, S2.62, is the same value).
        Expr::Bif(b, args) if b == "CHAR" && matches!(args.as_slice(), [Expr::Ind(_)]) => {
            let [Expr::Ind(n)] = args.as_slice() else {
                return None;
            };
            used.push(format!("ind {n}"));
            Some(ir::CharExpr::Ind(n.clone()))
        }
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
            // A third argument (S2.81): *ASTFILL or *CURSYM, or a literal
            // (another expression's value is not lowered).
            let (x, code, third) = match args.as_slice() {
                [x, Expr::Str(code)] => (x, code, None),
                [x, Expr::Str(code), t] => {
                    let third = match t {
                        Expr::Name { parts, args: None } if matches!(parts.as_slice(), [p] if p == "*ASTFILL" || p == "*CURSYM") => {
                            parts.first()?.clone()
                        }
                        Expr::Str(s) => super::super::data::a(s),
                        _ => return None,
                    };
                    (x, code, Some(third))
                }
                _ => return None,
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
                third,
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
            (is_char(&o) && !is_temporal(&o)).then_some(ir::CharExpr::Leaf(o))
        }
    }
}

/// The value of a character EVAL (S2.52): a date, time or timestamp field
/// as the whole value, or a character expression.
fn char_value(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::CharExpr> {
    // %DATE, %TIME and %TIMESTAMP (S2.54), and date arithmetic with
    // durations (S2.55), as the whole value.
    if matches!(e, Expr::Bif(..) | Expr::Bin(..)) {
        if let Some(t) = temporal_value(cx, e, used, Vec::new()) {
            return Some(t);
        }
    }
    if let Expr::Name { .. } = e {
        if let Some(o) = variable(cx, e, used) {
            if is_temporal(&o) {
                return Some(ir::CharExpr::Leaf(o));
            }
        }
    }
    char_expr(cx, e, used, Vec::new())
}

/// A numeric expression of +, - and * over literals and eligible fields
/// (S2.2), each operation with its place in the statement's expression.
fn num_expr(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>, path: Vec<u8>) -> Option<ir::NumExpr> {
    match e {
        // %DIFF and %SUBDT of temporal values (S2.55), as bif_more: the
        // arguments in order, the unit a special value.
        Expr::Bif(b, args) if b == "DIFF" || b == "SUBDT" => {
            let at = |k: u8| {
                let mut p = path.clone();
                p.push(k);
                p
            };
            match (b.as_str(), args.as_slice()) {
                ("DIFF", [x, y, u]) => {
                    let unit = dur_unit(u)?;
                    Some(ir::NumExpr::TemporalDiff {
                        x: Box::new(temporal_value(cx, x, used, at(0))?),
                        y: Box::new(temporal_value(cx, y, used, at(1))?),
                        unit,
                    })
                }
                ("SUBDT", [x, u]) => {
                    let unit = dur_unit(u)?;
                    Some(ir::NumExpr::TemporalSubdt {
                        x: Box::new(temporal_value(cx, x, used, at(0))?),
                        unit,
                    })
                }
                _ => None,
            }
        }
        // %ELEM of a global varying-dimension array (S2.45): its elements
        // in use, bifs.rs bif_override "ELEM".
        Expr::Bif(b, args) if b == "ELEM" && args.len() == 1 => {
            let Some(Expr::Name { parts, args: None }) = args.first() else {
                return None;
            };
            let name = parts.join(".");
            match sym(cx, &name) {
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
        // %OCCUR(ds) (S2.64): the current occurrence, as bifs.rs's
        // `num(self.occurrence(&args[0])? as i128)`.
        Expr::Bif(b, args) if b == "OCCUR" && args.len() == 1 => {
            occurs_ds(cx, args.first()?, used).map(ir::NumExpr::Leaf)
        }
        // %LEN of a varying-length field (S2.20): its current length, as
        // the interpreter's `(Some(DType::Varchar(_)), Value::Char(c))`
        // arm gives it.
        // %LEN of a character expression that is not a name, an indicator
        // or a path (S2.88): interp.rs type_of gives none, so the value's
        // length.
        Expr::Bif(b, args)
            if b == "LEN"
                && matches!(
                    args.as_slice(),
                    [Expr::Bif(..) | Expr::Bin(..) | Expr::Str(_)]
                ) =>
        {
            let mut p = path.clone();
            p.push(0);
            let c = char_expr(cx, args.first()?, used, p)?;
            Some(ir::NumExpr::LenOf(Box::new(c)))
        }
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
            let Some(Sym::Var(v)) = sym(cx, &name) else {
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
                x: Box::new(x),
                buf: v.buf,
                off: v.off,
                stride: v.stride,
                dim: v.dim,
                ty,
                start: match start {
                    Some(s) => Some(Box::new(operand(cx, s, used)?)),
                    None => None,
                },
                n: match n {
                    Some(s) => Some(Box::new(operand(cx, s, used)?)),
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
        // %XFOOT, %MAXARR, %MINARR of a whole global array (S2.68), the
        // array as %LOOKUP takes it; no start or number of elements.
        Expr::Bif(b, args)
            if matches!(b.as_str(), "XFOOT" | "MAXARR" | "MINARR") && args.len() == 1 =>
        {
            let Some(Expr::Name { parts, args: None }) = args.first() else {
                return None;
            };
            let name = parts.join(".");
            let Some(Sym::Var(v)) = sym(cx, &name) else {
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
                // %MAXARR and %MINARR also compare characters.
                DType::Char(n) if b != "XFOOT" => ir::Ty::Char(n),
                _ => return None,
            };
            used.push(format!(
                "{b} {name} {} {} {} {} {:?}",
                v.buf, v.off, v.stride, v.dim, v.dtype
            ));
            Some(ir::NumExpr::ArrFold {
                kind: b.clone(),
                buf: v.buf,
                off: v.off,
                stride: v.stride,
                dim: v.dim,
                ty,
            })
        }
        // %SCANR of character expressions, with a start and a length that
        // are numeric literals or fields (S2.67).
        Expr::Bif(b, args) if b == "SCANR" => {
            let (pat, s, start, len) = match args.as_slice() {
                [p, s] => (p, s, None, None),
                [p, s, st] => (p, s, Some(st), None),
                [p, s, st, ln] => (p, s, Some(st), Some(ln)),
                _ => return None,
            };
            let mut pp = path.clone();
            pp.push(0);
            let mut sp = path.clone();
            sp.push(1);
            Some(ir::NumExpr::ScanR {
                pat: Box::new(char_expr(cx, pat, used, pp)?),
                s: Box::new(char_expr(cx, s, used, sp)?),
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
        // %EOF, %FOUND, %EQUAL (S2.58): interp.rs bif's file_status, of
        // the file named or of the last file operation.
        Expr::Bif(b, args) if matches!(b.as_str(), "EOF" | "FOUND" | "EQUAL") => {
            let bit = match b.as_str() {
                "EOF" => ir::FileBit::Eof,
                "FOUND" => ir::FileBit::Found,
                _ => ir::FileBit::Equal,
            };
            let file = match args.as_slice() {
                [] => None,
                [Expr::Name { parts, args: None }] => Some(parts.join(".")),
                _ => return None,
            };
            used.push(format!("file status {b} {file:?}"));
            return Some(ir::Cond::FileStatus(bit, file));
        }
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
    // Temporal values on both sides, not both fields (S2.55): a date,
    // time or timestamp bif or date arithmetic, compared as the character
    // data they are (data::compare).
    if !(operand_only(l) && operand_only(r)) {
        if let (Some(a), Some(b)) = (
            temporal_value(cx, l, used, child(0)),
            temporal_value(cx, r, used, child(1)),
        ) {
            return Some(ir::Cond::CmpExpr(
                cmp,
                Box::new(ir::CmpSide::Char(a)),
                Box::new(ir::CmpSide::Char(b)),
            ));
        }
    }
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
        // A hexadecimal literal (S2.57): eval's `Expr::Hex(s) =>
        // Value::Char(s)`, character data of those bytes. Its two other
        // forms stay out: an EVAL of one to an integer field (Stmt::Eval
        // takes its bytes as an unsigned number), which needs a numeric
        // operand and gets none here, and an integer argument (value_arg
        // refuses Hex).
        Expr::Hex(b) => Some(ir::Operand::Str(b.clone())),
        // A named constant (S2.42): eval's `Sym::Const(c) => c`, its value
        // as defined, so the same operand as the literal it stands for.
        // The value goes into the routine's layout lines, so the
        // fingerprint changes with it: two programs whose code is the same
        // but whose constant differs (one from an include of another
        // source) must not share generated code (rla_test
        // include_search_order_follows_source_kind under native-diff).
        Expr::Name { parts, args: None }
            if matches!(sym(cx, &parts.join(".")), Some(Sym::Const(_))) =>
        {
            let name = parts.join(".");
            let op = match sym(cx, &name) {
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
            let n: i128 = match (b.as_str(), sym(cx, &name)?) {
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
                ty: ir::Ty::Char(_)
                    | ir::Ty::Varchar(_)
                    | ir::Ty::Date(..)
                    | ir::Ty::Time(..)
                    | ir::Ty::Timestamp(_),
                ..
            }
    )
}

/// A date, time or timestamp field (S2.52). Its value is character data
/// (its *ISO text, data::load), but the interpreter treats it apart in
/// places: DSPLY shows it in its format (%CHAR), a date plus a time is a
/// timestamp, MOVE converts. So it is lowered only as an EVAL's target
/// or whole value and as a comparison's side, where eval, assign and
/// data::compare treat it as the character data it loads as.
fn is_temporal(o: &ir::Operand) -> bool {
    matches!(
        o,
        ir::Operand::Var {
            ty: ir::Ty::Date(..) | ir::Ty::Time(..) | ir::Ty::Timestamp(_),
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
    let v = match sym(cx, name) {
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
                && local_array_ok(cx, v)
                && !name.starts_with("TAB")
                && !cx.prog.varying.contains_key(name) =>
        {
            let k = k.to_i128().ok()?;
            if k < 1 || k > i128::from(v.dim) {
                return None;
            }
            Some(usize::try_from(k).ok()?)
        }
        Some([ix]) if v.dim > 0 && local_array_ok(cx, v) && !name.starts_with("TAB") => {
            let of = operand(cx, ix, used)?;
            let vary = cx.prog.varying.get(name).map(|k| (name.clone(), *k == 'A'));
            // A local varying-dimension array is not lowered (S2.63).
            if v.local && vary.is_some() {
                return None;
            }
            if let Some((n, auto)) = &vary {
                used.push(format!("vary {n} auto={auto}"));
            }
            index = Some(Box::new(ir::Index {
                of,
                stride: v.stride,
                dim: v.dim,
                vary,
                outer: None,
            }));
            Some(1)
        }
        _ => return None,
    };
    // A scalar subfield of a global multiple-occurrence data structure
    // (S2.64): in the current occurrence, as Machine::sym adds
    // (occurrence - 1) times the occurrence's size.
    let occurs = (!v.local && v.dim == 0)
        .then(|| cx.prog.occurs.iter().find(|o| o.buf == v.buf))
        .flatten();
    if let Some(o) = occurs {
        used.push(format!(
            "occurs {} size {} count {}",
            o.name, o.size, o.count
        ));
        index = Some(Box::new(ir::Index {
            of: ir::Operand::Occur {
                buf: o.buf,
                name: o.name.clone(),
            },
            stride: o.size,
            dim: o.count,
            vary: None,
            outer: None,
        }));
    }
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
        // A date, time or timestamp field (S2.52): only where `is_temporal`
        // lets it (an EVAL target or a whole value, a comparison's side).
        DType::Date(f) => ir::Ty::Date(f.name, f.sep),
        DType::Time(f) => ir::Ty::Time(f.name, f.sep),
        DType::Timestamp(n) => ir::Ty::Timestamp(n),
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
        // A procedure local (S2.8): only in a procedure (based ones above);
        // an array only by an element (S2.63, `local_array_ok`).
        (v.dim != 0) != elem.is_some() || cx.based.is_none()
    } else {
        (v.dim != 0) != elem.is_some()
            || (occurs.is_none() && prog.occurs.iter().any(|o| o.buf == v.buf))
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

/// A global multiple-occurrence data structure by name (S2.64), as ops.rs
/// occurs_of finds it among the program's names: its current occurrence
/// as an operand.
fn occurs_ds(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    let Expr::Name { parts, args: None } = e else {
        return None;
    };
    let name = parts.join(".");
    let Some(Sym::Ds(d)) = cx.prog.syms.get(&name) else {
        return None;
    };
    let o = cx.prog.occurs.iter().find(|o| o.buf == d.buf)?;
    used.push(format!("occurrence of {} buf {}", o.name, o.buf));
    Some(ir::Operand::Occur {
        buf: o.buf,
        name: o.name.clone(),
    })
}

/// An array that lowering may index (S2.63): a global one, or a local one
/// of the procedure's own storage that is neither based nor a parameter
/// (one passed by reference is the caller's storage, where the
/// interpreter's `elem_off` also checks that the element is there,
/// MCH0601).
fn local_array_ok(cx: &Cx<'_>, v: &super::super::program::VarInfo) -> bool {
    if !v.local {
        return true;
    }
    cx.based.is_some_and(|b| !b.contains_key(&v.buf))
        && cx
            .params
            .is_some_and(|ps| ps.iter().all(|p| p.buf != Some(v.buf)))
}

/// A scalar subfield of an element of a data structure array, global
/// (S2.30) or a procedure's own (S2.63), `ds(i).field`: the
/// interpreter's `path_sym` adds (i - 1) times
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
    // The indexes are on data structures: the parser gives `idx` one entry
    // per part, or only up to the last indexed part. Each on a part but
    // the last (S2.76: `list.users(i).name`, a LIKEDS subfield with DIM,
    // whose subfields program.rs names LIST.USERS.NAME at the first
    // element's place; S2.77: more than one, `gv(s).m(1).parmNo`).
    // interp.rs path_sym adds, part by part from the left, (i - 1) times
    // that structure's size after its range check: here a chain of
    // indexes, the outer one first (`ir::Index::outer`).
    let indexed: Vec<usize> = idx
        .iter()
        .enumerate()
        .filter_map(|(k, x)| x.is_some().then_some(k))
        .collect();
    let last = parts.len().checked_sub(1)?;
    let first = *indexed.first()?;
    if parts.len() < 2 || indexed.iter().any(|k| *k >= last) {
        return None;
    }
    let ds = parts.get(..=first)?.join(".");
    let ds = &ds;
    let prog = cx.prog;
    let Some(Sym::Ds(d)) = sym(cx, ds) else {
        return None;
    };
    // A local one (S2.63): of the procedure's own storage, not based and
    // not a parameter (`local_array_ok`).
    let local_ok = !d.local
        || (cx.based.is_some_and(|b| !b.contains_key(&d.buf))
            && cx
                .params
                .is_some_and(|ps| ps.iter().all(|p| p.buf != Some(d.buf))));
    if !local_ok
        || d.dim == 0
        || (!d.local && prog.based.contains_key(&d.buf))
        || (!d.local && prog.occurs.iter().any(|o| o.buf == d.buf))
    {
        return None;
    }
    let name = parts.join(".");
    let Some(Sym::Var(v)) = sym(cx, &name) else {
        return None;
    };
    if v.local != d.local || v.dim != 0 || v.buf != d.buf || prog.varying.contains_key(&name) {
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
    let mut index: Option<Box<ir::Index>> = None;
    let mut shown = String::new();
    for (k, part) in parts.iter().enumerate() {
        if k > 0 {
            shown.push('.');
        }
        shown.push_str(part);
        let Some(Some(ix)) = idx.get(k) else {
            continue;
        };
        let at = parts.get(..=k)?.join(".");
        let Some(Sym::Ds(dk)) = sym(cx, &at) else {
            return None;
        };
        if dk.dim == 0 || dk.buf != d.buf || dk.local != d.local {
            return None;
        }
        let of = operand(cx, ix, used)?;
        let _ = std::fmt::Write::write_fmt(
            &mut shown,
            format_args!(
                "({}) stride {} dim {}",
                match &of {
                    ir::Operand::Var { name, .. } => name.clone(),
                    other => format!("{other:?}"),
                },
                dk.size,
                dk.dim
            ),
        );
        index = Some(Box::new(ir::Index {
            of,
            stride: dk.size,
            dim: dk.dim,
            vary: None,
            outer: index,
        }));
    }
    used.push(format!(
        "layout {shown} {} {} {:?} {}",
        v.buf,
        v.off,
        v.dtype,
        if v.local { "local" } else { "global" }
    ));
    Some(ir::Operand::Var {
        name: shown,
        buf: v.buf,
        off: v.off,
        ty,
        local: v.local,
        index: Some(index?),
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
        && sym(cx, &name).is_none()
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
    if parts.len() != 1 || sym(cx, &parts.join(".")).is_some() {
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
            // EXTPROC(procedure pointer) (S2.87): a pointer field, which the
            // runtime reads when the call is made (call_proto).
            if !pr.overload.is_empty()
                || pr.extproc_ptr.as_ref().is_some_and(|v| {
                    !matches!(sym(cx, v), Some(Sym::Var(vi)) if vi.dtype == DType::Pointer && vi.dim == 0)
                })
            {
                return None;
            }
            let local = match &pr.extpgm {
                Some(pgm) => {
                    if matches!(sym(cx, pgm), Some(Sym::Var(_))) {
                        return None;
                    }
                    None
                }
                None => {
                    // call_named_inner's order (S2.56): this module's
                    // procedure, unless EXTPROC written in lower case names
                    // the system's C function; the binder's, a CEE API or a
                    // C function are the runtime's choice (call_proto).
                    let target = pr.extproc.clone().unwrap_or_else(|| name.clone());
                    let system_named = pr
                        .written
                        .as_deref()
                        .is_some_and(|w| w != w.to_ascii_uppercase())
                        && super::super::interp::crt::C_FUNCTIONS.contains(&target.as_str());
                    if system_named {
                        None
                    } else {
                        prog.procs.get(&target)
                    }
                }
            };
            let _ = std::fmt::Write::write_fmt(
                &mut layout,
                format_args!(
                    " proto extpgm={:?} extproc={:?} written={:?}",
                    pr.extpgm, pr.extproc, pr.written
                ),
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
    // A C function takes arguments past its prototype's, as values
    // (call_named_inner: printf's list); S2.56.
    let c_extra = local.is_none()
        && !prog.procs.contains_key(&name)
        && proto(cx, &name).is_some_and(|pr| {
            pr.extpgm.is_none()
                && super::super::interp::crt::C_FUNCTIONS.contains(
                    &pr.extproc
                        .clone()
                        .unwrap_or_else(|| name.clone())
                        .to_ascii_uppercase()
                        .as_str(),
                )
        });
    if (args.len() > params.len() && !c_extra) || params.iter().skip(args.len()).any(|p| !p.nopass)
    {
        return None;
    }
    let mut out = Vec::with_capacity(args.len());
    for (k, (p, a)) in params.iter().zip(args).enumerate() {
        // OPTIONS(*VARSIZE) (S2.51) only for a program or a bound
        // procedure, whose arguments go as the interpreter's loc_of gives
        // them (the argument's own size and type).
        // OPTIONS(*STRING) (S2.66): for a pointer parameter of a C
        // function passed by value or CONST (call_proto's C path makes the
        // copy args_for makes).
        let string_ok =
            c_extra && p.dtype == DType::Pointer && (p.by_value || p.konst) && !p.varsize;
        if p.file
            || p.omit
            || (p.string && !string_ok)
            || p.trim
            || p.convert
            || (p.varsize && local.is_some())
        {
            return None;
        }
        // A parameter that is a data structure (LIKEDS), or a whole data
        // structure as the argument: the interpreter may pass it by
        // reference (same_type_variable) where a value would be built
        // here; not lowered (OpenRPG test325 under native-corpus.sh).
        // S2.60: a whole data structure for a program or a procedure of
        // another module is passed as loc_of gives it (its bytes, as a
        // character field of its size): by reference, or for CONST by
        // reference when the sizes match (same_type_variable) and as its
        // value otherwise.
        let whole_ds = matches!(a, Expr::Name { parts, .. } if matches!(sym(cx, &parts.join(".")), Some(Sym::Ds(_))));
        if local.is_some_and(|pi| !matches!(pi.syms.get(&p.name), Some(Sym::Var(_))))
            || (whole_ds && local.is_some())
        {
            return None;
        }
        let ty = call_param_ty(p.dtype)?;
        let _ = std::fmt::Write::write_fmt(
            &mut layout,
            format_args!(
                " {}={:?}/{}/{}/{}",
                p.name, ty, p.by_value, p.konst, p.varsize
            ),
        );
        let mut ap = path.clone();
        ap.push(u8::try_from(k).ok()?);
        let field = |a: &Expr| match a {
            Expr::Name { parts, .. } => match sym(cx, &parts.join(".")) {
                Some(Sym::Var(_)) => true,
                // A whole data structure (S2.60), for a call that is not
                // local; `variable` takes only those it can place.
                Some(Sym::Ds(_)) => local.is_none(),
                _ => false,
            },
            _ => false,
        };
        // A character value for an OPTIONS(*STRING) pointer parameter
        // (S2.66): args_for's eval of it is character data, so it goes as
        // its characters, and the runtime makes the null-terminated copy.
        if p.string {
            if let Some(c) = char_expr(cx, a, used, ap.clone()) {
                out.push(ir::CallArg::Char(c));
                continue;
            }
        }
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
                // OPTIONS(*VARSIZE) (S2.51): a character field of any
                // length for a character parameter, by reference (args_for:
                // by reference as loc_of gives it; CONST: char_variable).
                ir::Operand::Var {
                    ty: ir::Ty::Char(_),
                    ..
                } if p.varsize && matches!(ty, ir::Ty::Char(_)) => {}
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
    for (k, a) in args.iter().enumerate().skip(params.len()) {
        let mut ap = path.clone();
        ap.push(u8::try_from(k).ok()?);
        if matches!(a, Expr::Hex(_) | Expr::Omit) {
            return None;
        }
        let v = if let Some(c) = char_expr(cx, a, used, ap.clone()) {
            ir::CallArg::Char(c)
        } else if let Some(n) = num_expr(cx, a, used, ap.clone()) {
            ir::CallArg::Num(n)
        } else {
            ir::CallArg::Ptr(ptr_val(cx, a, used, ap)?)
        };
        let _ = std::fmt::Write::write_fmt(&mut layout, format_args!(" extra{k}"));
        out.push(v);
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
/// character one (no conversion between them; a hexadecimal literal only
/// for a character parameter, S2.60).
fn value_arg(
    cx: &Cx<'_>,
    ty: &ir::Ty,
    a: &Expr,
    used: &mut Vec<String>,
    path: Vec<u8>,
) -> Option<ir::CallArg> {
    // A hexadecimal literal for a character parameter is its bytes (S2.60:
    // eval's Value::Char); for an integer one args_for makes it a number,
    // which is not lowered.
    if matches!(a, Expr::Omit)
        || (matches!(a, Expr::Hex(_)) && !matches!(ty, ir::Ty::Char(_) | ir::Ty::Varchar(_)))
    {
        return None;
    }
    match ty {
        ir::Ty::Char(_) | ir::Ty::Varchar(_) => {
            Some(ir::CallArg::Char(char_expr(cx, a, used, path)?))
        }
        // A pointer parameter (S2.51): *NULL, or a pointer value.
        ir::Ty::Pointer => Some(ir::CallArg::Ptr(match a {
            Expr::Fig(Fig::Null) => ir::PtrVal::Null,
            _ => ptr_val(cx, a, used, path)?,
        })),
        _ => Some(ir::CallArg::Num(num_expr(cx, a, used, path)?)),
    }
}

/// The generator's type for a parameter of a call (S2.51): `param_ty`, or
/// a pointer.
fn call_param_ty(t: DType) -> Option<ir::Ty> {
    match t {
        DType::Pointer => Some(ir::Ty::Pointer),
        other => param_ty(other),
    }
}

/// AS4TAINR_NATIVE_WHY=<file>: the shape of each EVAL that is not lowered,
/// one line each (the target's kind and type, then the expression's
/// nodes), to find the forms that keep code in the interpreter (ADR 0007,
/// phase A). A diagnostic: it does not change what is generated. At most
/// 100 000 lines per process.
fn note_why(cx: &Cx<'_>, i: &Ins, at: usize) {
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
            Ins::Stmt(Stmt::CallP(Expr::Name { parts, args })) => {
                let name = parts.join(".");
                // Each argument's shape against its parameter's type and
                // passing (S2.60's diagnostic).
                let params = match cx.prog.procs.get(&name) {
                    Some(pi) => Some(&pi.params),
                    None => proto(cx, &name).map(|pr| &pr.params),
                };
                let mut detail = Vec::new();
                for (k, a) in args.as_deref().unwrap_or(&[]).iter().enumerate() {
                    let mut nodes = Vec::new();
                    shape(cx, a, &mut nodes, 0);
                    nodes.sort();
                    nodes.dedup();
                    let pd = match params.and_then(|ps| ps.get(k)) {
                        Some(p) => format!(
                            "{}{}",
                            type_name(p.dtype),
                            if p.by_value {
                                "/value"
                            } else if p.konst {
                                "/const"
                            } else {
                                "/ref"
                            }
                        ),
                        None => "extra".to_string(),
                    };
                    detail.push(format!("{pd}<-{}", nodes.join("&")));
                }
                let detail = detail.join(" ");
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
                        format!("callp=local why={} args={detail}\n", why.join(","))
                    }
                    None => format!("callp=proto why={} args={detail}\n", proto_why(cx, &name)),
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
        // Where it is, after the shape (S2.61): the program and its line.
        let line = format!("{} @{}:{at}\n", line.trim_end(), cx.prog.name);
        let _ = std::io::Write::write_all(&mut f, line.as_bytes());
    }
}

/// What kind of target an EVAL has, and its type.
fn target_shape(cx: &Cx<'_>, e: &Expr) -> String {
    match e {
        Expr::Name { parts, args } => {
            let k = if args.is_some() { "elem" } else { "name" };
            match sym(cx, &parts.join(".")) {
                Some(Sym::Var(v)) => format!("{k}:{}!{}", type_name(v.dtype), var_why(cx, v)),
                Some(Sym::Ds(_)) => format!("{k}:ds"),
                Some(_) => format!("{k}:other"),
                None => format!("{k}:unknown:{}", parts.join(".")),
            }
        }
        Expr::Path { parts, idx, args } => {
            let first = parts.first().map(String::as_str).unwrap_or("");
            let ds = match sym(cx, first) {
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
            match (sym(cx, &n), args) {
                (Some(Sym::Var(v)), None) => {
                    let why = if variable(cx, e, &mut Vec::new()).is_some() {
                        String::new()
                    } else {
                        format!("!{}", var_why(cx, v))
                    };
                    out.push(format!("var:{}{why}", type_name(v.dtype)))
                }
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
                    out.push(format!("call:proto!{}", proto_why(cx, &n)));
                    for x in a {
                        shape(cx, x, out, d);
                    }
                }
                (None, None) => out.push(format!("name:unknown:{n}")),
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
            matches!(sym(cx, &parts.join(".")), Some(Sym::Var(v)) if v.dtype == DType::Pointer)
        }
        _ => false,
    }
}

/// The path of the size in %ALLOC and %REALLOC (S2.50):
/// apart from the steps of pointer arithmetic (0, 1, ...), so their
/// helpers' names differ.
const ALLOC_PATH: u8 = 200;

/// A pointer value (S2.47, S2.49): a pointer field, %ADDR of a global
/// field (ptr_to of its place; *DATA skips a varying field's length), or
/// either moved by numbers (`p + n`, `p - n`, left to right: ptrs.rs
/// ptr_arith). A number plus a pointer, and a pointer less a pointer, are
/// not lowered.
fn ptr_val(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>, path: Vec<u8>) -> Option<ir::PtrVal> {
    // Each number's helpers are named by `path` and its own place.
    let at_path = |k: u8| {
        let mut p = path.clone();
        p.push(k);
        p
    };
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
            // A global field, or (S2.65) a procedure's local one: ptr_to of
            // the place Machine::sym gives it, the frame's buffer (or the
            // caller's storage for a parameter passed by reference), which
            // generated code asks `rt.local` for. Not a based field.
            let of = variable(cx, of, used)?;
            let ir::Operand::Var {
                based: None, ty, ..
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
        // %ALLOC(n) and %REALLOC(p: n) (S2.50), the interpreter's bif
        // arguments in order. %REALLOC's pointer is a pointer field.
        // %PADDR (S2.87): the procedure's name as interp.rs bif's PADDR arm
        // takes it (a literal; a name: its prototype's EXTPROC, a named
        // constant's value, or itself); where it is, the runtime's.
        Expr::Bif(b, args) if b == "PADDR" => {
            let n = match args.as_slice() {
                [Expr::Str(b)] => super::super::data::a(b).trim().to_ascii_uppercase(),
                [Expr::Name { parts, args: None }] if parts.len() == 1 => {
                    let n = parts.first()?.to_ascii_uppercase();
                    match proto(cx, &n).and_then(|p| p.extproc.clone()) {
                        Some(e) => e.to_ascii_uppercase(),
                        None => match sym(cx, &n) {
                            Some(Sym::Const(v)) => super::super::data::a(&v.chars().ok()?)
                                .trim()
                                .to_ascii_uppercase(),
                            _ => n,
                        },
                    }
                }
                _ => return None,
            };
            used.push(format!("paddr {n}"));
            ir::PtrVal::Paddr(n)
        }
        Expr::Bif(b, args) if b == "ALLOC" => match args.as_slice() {
            [n] => ir::PtrVal::Alloc(num_expr(cx, n, used, at_path(ALLOC_PATH))?),
            _ => return None,
        },
        Expr::Bif(b, args) if b == "REALLOC" => match args.as_slice() {
            [p, n] if is_pointer_name(cx, p) => ir::PtrVal::Realloc {
                p: Box::new(ir::PtrVal::Var(variable(cx, p, used)?)),
                size: num_expr(cx, n, used, at_path(ALLOC_PATH))?,
            },
            _ => return None,
        },
        _ => return None,
    };
    if steps.is_empty() {
        return Some(base);
    }
    steps.reverse();
    let mut by = Vec::with_capacity(steps.len());
    for (k, (neg, r)) in steps.into_iter().enumerate() {
        by.push((neg, num_expr(cx, r, used, at_path(u8::try_from(k).ok()?))?));
    }
    Some(ir::PtrVal::Move {
        base: Box::new(base),
        by,
    })
}

/// AS4TAINR_NATIVE_WHY: what may keep a field out of lowered code (the
/// checks of `variable`, by name; a diagnostic only).
fn var_why(cx: &Cx<'_>, v: &super::super::program::VarInfo) -> String {
    let prog = cx.prog;
    let mut why = Vec::new();
    if v.local {
        why.push("local");
    }
    if v.dim != 0 {
        why.push("array");
    }
    let based = if v.local {
        cx.based.is_some_and(|b| b.contains_key(&v.buf))
    } else {
        prog.based.contains_key(&v.buf)
    };
    if based {
        why.push("based");
    }
    if prog.occurs.iter().any(|o| o.buf == v.buf) {
        why.push("occurs");
    }
    if !matches!(
        v.dtype,
        DType::Packed(..)
            | DType::Zoned(..)
            | DType::Binary(..)
            | DType::Int(_)
            | DType::Uns(_)
            | DType::Char(_)
            | DType::Varchar(_)
            | DType::Pointer
    ) {
        why.push("type");
    }
    if why.is_empty() {
        why.push("other");
    }
    why.join("+")
}

/// AS4TAINR_NATIVE_WHY: what keeps a call through a prototype out of
/// lowered code (the checks of `call_of`, by name; a diagnostic only).
fn proto_why(cx: &Cx<'_>, name: &str) -> String {
    let Some(pr) = proto(cx, name) else {
        return "no-proto".to_string();
    };
    let mut why = Vec::new();
    if !pr.overload.is_empty() {
        why.push("overload".to_string());
    }
    if pr.extproc_ptr.is_some() {
        why.push("procptr".to_string());
    }
    match &pr.extpgm {
        Some(pgm) if matches!(sym(cx, pgm), Some(Sym::Var(_))) => why.push("pgm-var".to_string()),
        Some(_) => {}
        None => {
            let up = pr
                .extproc
                .clone()
                .unwrap_or_else(|| name.to_string())
                .to_ascii_uppercase();
            if super::super::interp::crt::C_FUNCTIONS.contains(&up.as_str()) {
                why.push("c".to_string());
            }
            if super::super::interp::cee::CEE_APIS.contains(&up.as_str()) {
                why.push("cee".to_string());
            }
        }
    }
    for p in &pr.params {
        // Each option by name (S2.60's diagnostic).
        for (on, opt) in [
            (p.file, "likefile"),
            (p.omit, "omit"),
            (p.string, "string"),
            (p.varsize, "varsize"),
            (p.trim, "trim"),
            (p.convert, "convert"),
            (p.nopass, "nopass"),
        ] {
            if on {
                why.push(format!("opt:{opt}"));
            }
        }
        if param_ty(p.dtype).is_none() {
            why.push(format!("param:{}", type_name(p.dtype)));
        }
    }
    if pr.returns.is_some_and(|t| param_ty(t).is_none()) {
        why.push("returns".to_string());
    }
    if why.is_empty() {
        why.push("arguments".to_string());
    }
    why.sort();
    why.dedup();
    why.join("+")
}

/// A date, time or timestamp field by name (S2.52).
fn is_temporal_name(cx: &Cx<'_>, e: &Expr) -> bool {
    match e {
        Expr::Name { parts, .. } => matches!(
            sym(cx, &parts.join(".")),
            Some(Sym::Var(v)) if matches!(v.dtype, DType::Date(_) | DType::Time(_) | DType::Timestamp(_))
        ),
        _ => false,
    }
}

/// %CHAR's arguments as bif_override takes them (S2.53): a date, time or
/// timestamp field, and maybe a format.
fn temporal_arg(cx: &Cx<'_>, args: &[Expr]) -> bool {
    matches!(args.len(), 1 | 2) && args.first().is_some_and(|a| is_temporal_name(cx, a))
}

/// %CHAR of a date, time or timestamp field (S2.53), as bifs.rs
/// bif_override: the explicit format (a special value, *MDY...), or the
/// field's, or the control specification's DATFMT or TIMFMT. A format
/// that does not parse, or a second argument that is not a special value,
/// is not lowered.
fn temporal_char(cx: &Cx<'_>, args: &[Expr], used: &mut Vec<String>) -> Option<ir::CharExpr> {
    use rpgrt::rpg::dates::{DateFmt, TimeFmt};
    let (x, explicit) = match args {
        [x] => (x, None),
        [x, Expr::Name { parts, args: None }] if parts.len() == 1 => {
            let f = parts.first()?;
            if !f.starts_with('*') {
                return None;
            }
            (x, Some(f.clone()))
        }
        _ => return None,
    };
    let of = variable(cx, x, used)?;
    let ir::Operand::Var { ty, .. } = &of else {
        return None;
    };
    let fmt = match *ty {
        ir::Ty::Date(name, sep) => {
            let f = match &explicit {
                Some(e) => DateFmt::parse(e).ok()?,
                None => DateFmt { name, sep }.or(cx.prog.datfmt().ok()?),
            };
            ir::TemporalFmt::Date(f.name, f.sep)
        }
        ir::Ty::Time(name, sep) => {
            let f = match &explicit {
                Some(e) => TimeFmt::parse(e).ok()?,
                None => TimeFmt { name, sep }.or(cx.prog.timfmt().ok()?),
            };
            ir::TemporalFmt::Time(f.name, f.sep)
        }
        ir::Ty::Timestamp(_) => ir::TemporalFmt::Ts {
            iso0: explicit.as_deref() == Some("*ISO0"),
        },
        _ => return None,
    };
    used.push(format!("temporal char {fmt:?}"));
    Some(ir::CharExpr::TemporalChar { of, fmt })
}

/// %DATE, %TIME or %TIMESTAMP (S2.54), as bifs.rs bif_more takes them:
/// no argument, *SYS (and *JOB for %DATE) for the current date and time;
/// otherwise the first argument's value (a date, time or timestamp field,
/// a character or a numeric expression), and a special value as the
/// second argument is the format (any other second argument is not
/// looked at, as special_arg gives none).
fn temporal_bif(
    cx: &Cx<'_>,
    b: &str,
    args: &[Expr],
    used: &mut Vec<String>,
    path: Vec<u8>,
) -> Option<ir::CharExpr> {
    let mut ap = path;
    ap.push(0);
    let special = |e: Option<&Expr>| match e {
        Some(Expr::Name { parts, args: None }) if parts.len() == 1 => {
            parts.first().filter(|p| p.starts_with('*')).cloned()
        }
        _ => None,
    };
    let (kind, dft) = match b {
        "DATE" => (ir::TemporalBifKind::Date, cx.prog.datfmt().ok()?.code()),
        "TIME" => (ir::TemporalBifKind::Time, cx.prog.timfmt().ok()?.code()),
        _ => (ir::TemporalBifKind::Ts, String::new()),
    };
    let now = match special(args.first()).as_deref() {
        _ if args.is_empty() => true,
        Some("*SYS") => true,
        Some("*JOB") if kind == ir::TemporalBifKind::Date => true,
        // Another special value as the argument: eval of it.
        Some(_) => return None,
        None => false,
    };
    let arg = if now {
        None
    } else {
        let a = args.first()?;
        let v = if is_temporal_name(cx, a) {
            ir::ValExpr::Char(ir::CharExpr::Leaf(variable(cx, a, used)?))
        } else if let Some(c) = char_expr(cx, a, used, ap.clone()) {
            ir::ValExpr::Char(c)
        } else {
            ir::ValExpr::Num(num_expr(cx, a, used, ap)?)
        };
        Some(Box::new(v))
    };
    let fmt = if now { None } else { special(args.get(1)) };
    used.push(format!("temporal bif {b} dft {dft}"));
    Some(ir::CharExpr::TemporalBif {
        kind,
        arg,
        fmt,
        dft,
    })
}

/// The rpgrt::rpg::data::DurUnit variant a special value names (S2.55):
/// *YEARS or *Y ... *MSECONDS or *MS (DurUnit::parse).
fn dur_unit(e: &Expr) -> Option<&'static str> {
    let Expr::Name { parts, args: None } = e else {
        return None;
    };
    let [p] = parts.as_slice() else {
        return None;
    };
    unit_name(rpgrt::rpg::data::DurUnit::parse(p)?)
}

fn unit_name(u: rpgrt::rpg::data::DurUnit) -> Option<&'static str> {
    use rpgrt::rpg::data::DurUnit as U;
    Some(match u {
        U::Years => "Years",
        U::Months => "Months",
        U::Days => "Days",
        U::Hours => "Hours",
        U::Minutes => "Minutes",
        U::Seconds => "Seconds",
        U::MSeconds => "MSeconds",
    })
}

/// A duration bif, %YEARS ... %MSECONDS of one number (S2.55): its unit,
/// as bif_more's `DurUnit::parse(&format!("*{b}"))`.
fn duration(e: &Expr) -> Option<(&'static str, &Expr)> {
    let Expr::Bif(b, args) = e else {
        return None;
    };
    if !matches!(
        b.as_str(),
        "YEARS" | "MONTHS" | "DAYS" | "HOURS" | "MINUTES" | "SECONDS" | "MSECONDS"
    ) {
        return None;
    }
    let [n] = args.as_slice() else {
        return None;
    };
    Some((
        unit_name(rpgrt::rpg::data::DurUnit::parse(&format!("*{b}"))?)?,
        n,
    ))
}

/// A temporal value (S2.55): a date, time or timestamp field, %DATE,
/// %TIME or %TIMESTAMP, or one of these plus or minus a duration (eval's
/// date_arith: a duration on either side of +, on the right of -).
fn temporal_value(
    cx: &Cx<'_>,
    e: &Expr,
    used: &mut Vec<String>,
    path: Vec<u8>,
) -> Option<ir::CharExpr> {
    let at = |k: u8| {
        let mut p = path.clone();
        p.push(k);
        p
    };
    match e {
        Expr::Name { .. } if is_temporal_name(cx, e) => {
            Some(ir::CharExpr::Leaf(variable(cx, e, used)?))
        }
        Expr::Bif(b, args) if matches!(b.as_str(), "DATE" | "TIME" | "TIMESTAMP") => {
            temporal_bif(cx, b, args, used, path)
        }
        Expr::Bin(op @ (BinOp::Add | BinOp::Sub), l, r) => {
            let (base, dur, dur_first) = match (duration(l), duration(r)) {
                (None, Some(d)) => (&**l, d, false),
                (Some(d), None) if *op == BinOp::Add => (&**r, d, true),
                _ => return None,
            };
            let (unit, n) = dur;
            let (bp, np) = if dur_first {
                (at(1), at(0))
            } else {
                (at(0), at(1))
            };
            Some(ir::CharExpr::TemporalAdd {
                base: Box::new(temporal_value(cx, base, used, bp)?),
                unit,
                n: Box::new(num_expr(cx, n, used, np)?),
                minus: *op == BinOp::Sub,
                dur_first,
            })
        }
        _ => None,
    }
}

/// A read of a database file (S2.58, ADR 0007): CHAIN, READ, READE,
/// READP, READPE, SETLL or SETGT of an externally described DISK file of
/// the main program (no file declared in a procedure or passed with
/// LIKEFILE, so the name finds the same file wherever it runs), with no
/// INFDS, no input specifications for it, and a key of fields and
/// literals (or a KLIST of them, or *LOVAL, *HIVAL, *START, *END alone).
/// The fields of its record format go into the program fields of the
/// same names as fill_fields moves them: a number, characters or
/// hexadecimal, never a date, time, timestamp, float or graphic field.
fn file_read(
    cx: &Cx<'_>,
    op: FileOp,
    file: &str,
    key: Option<&[Expr]>,
    used: &mut Vec<String>,
) -> Option<ir::Lowered> {
    let change = matches!(op, FileOp::Write | FileOp::Update | FileOp::Delete);
    if !change
        && !matches!(
            op,
            FileOp::Chain
                | FileOp::Read
                | FileOp::ReadE
                | FileOp::ReadP
                | FileOp::ReadPE
                | FileOp::SetLL
                | FileOp::SetGT
        )
    {
        return None;
    }
    // WRITE and UPDATE take no key; DELETE's is a key or none (the record
    // last read). *LOVAL and the like are no DELETE key.
    if matches!(op, FileOp::Write | FileOp::Update) && key.is_some() {
        return None;
    }
    if op == FileOp::Delete && matches!(key, Some([Expr::Fig(_)])) {
        return None;
    }
    let prog = cx.prog;
    // No file of a procedure and no LIKEFILE parameter anywhere: file_idx
    // then finds a name among the program's files only.
    if prog.files.iter().any(|f| f.decl.proc.is_some())
        || prog.procs.values().any(|p| p.params.iter().any(|q| q.file))
        || prog
            .protos
            .values()
            .any(|p| p.params.iter().any(|q| q.file))
    {
        return None;
    }
    let i = prog
        .files
        .iter()
        .position(|f| f.decl.name == file || f.format == file)
        .or_else(|| prog.file_index(file))?;
    let fi = prog.files.get(i)?;
    let fref = fi.fref.as_ref()?;
    if fi.decl.device != "DISK"
        || fi.program_described
        || !fi.inputs.is_empty()
        || fi
            .decl
            .keywords
            .iter()
            .any(|k| k.name.eq_ignore_ascii_case("INFDS"))
    {
        return None;
    }
    // The key: *LOVAL/*HIVAL/*START/*END alone (no values), a KLIST's
    // fields, or fields and literals.
    let keys = match key {
        None => None,
        Some([Expr::Fig(Fig::LoVal | Fig::HiVal)]) => None,
        Some([Expr::Name { parts, .. }])
            if parts.first().is_some_and(|p| p == "*START" || p == "*END") =>
        {
            None
        }
        Some(k) => {
            let list: Vec<Expr> = match k {
                [Expr::Name { parts, args: None }] => match sym(cx, &parts.join(".")) {
                    Some(Sym::Klist(l)) => l.clone(),
                    _ => k.to_vec(),
                },
                _ => k.to_vec(),
            };
            let mut vals = Vec::with_capacity(list.len());
            for e in &list {
                // A field or a literal only: its value cannot fail, so it
                // makes no difference that it is taken before the
                // operation starts rather than inside it (key_values).
                if !matches!(
                    e,
                    Expr::Name { .. } | Expr::Num(_) | Expr::Str(_) | Expr::Hex(_)
                ) || is_temporal_name(cx, e)
                {
                    return None;
                }
                let o = any_operand(cx, e, used)?;
                if is_pointer(&o) || matches!(o, ir::Operand::Fig(_)) {
                    return None;
                }
                vals.push(if is_char(&o) {
                    ir::ValExpr::Char(ir::CharExpr::Leaf(o))
                } else {
                    ir::ValExpr::Num(ir::NumExpr::Leaf(o))
                });
            }
            Some(vals)
        }
    };
    let mut moves = Vec::new();
    if matches!(
        op,
        FileOp::Chain
            | FileOp::Read
            | FileOp::ReadE
            | FileOp::ReadP
            | FileOp::ReadPE
            | FileOp::Write
            | FileOp::Update
    ) {
        for (col, (fd, pname)) in fref.file.fields.iter().zip(&fi.fields).enumerate() {
            if pname.is_empty() {
                continue;
            }
            let target = variable(cx, &Expr::name(pname), used)?;
            let ir::Operand::Var { ty, index, .. } = &target else {
                return None;
            };
            // Date, time and timestamp fields (S2.82): their *ISO text both
            // ways, as fill_fields' Value::str and record_of's characters
            // (data::store and decode convert for the program field).
            if index.is_some()
                || !matches!(
                    ty,
                    ir::Ty::Packed(..)
                        | ir::Ty::Zoned(..)
                        | ir::Ty::Binary(..)
                        | ir::Ty::Int(_)
                        | ir::Ty::Uns(_)
                        | ir::Ty::Char(_)
                        | ir::Ty::Varchar(_)
                        | ir::Ty::Date(..)
                        | ir::Ty::Time(..)
                        | ir::Ty::Timestamp(_)
                )
            {
                return None;
            }
            let kind = if fd.data_type.is_numeric() {
                ir::MoveKind::Num
            } else if fd.data_type == crate::dds::DataType::Hex {
                ir::MoveKind::Hex
            } else if matches!(
                fd.data_type,
                crate::dds::DataType::Char
                    | crate::dds::DataType::Date
                    | crate::dds::DataType::Time
                    | crate::dds::DataType::Timestamp
            ) {
                ir::MoveKind::Char
            } else {
                return None;
            };
            // A number into a character field, or characters into a
            // numeric one, is not a move fill_fields makes for fields of
            // the file's own definition.
            if (kind == ir::MoveKind::Num) == is_char(&target) {
                return None;
            }
            let decimals = u8::try_from(fd.decimals).ok()?;
            moves.push(ir::FieldMove {
                col,
                target,
                kind,
                decimals,
            });
        }
    }
    if change {
        used.push(format!("file change {op:?} {file} {i}"));
        return Some(ir::Lowered::FileChange {
            keys,
            fields: moves,
            width: fref.file.fields.len(),
        });
    }
    used.push(format!("file read {op:?} {file} {i}"));
    Some(ir::Lowered::FileRead { keys, moves })
}

/// Whether an EXEC SQL is one step 1 lowers (S2.69): no host variable
/// (bind_host's `:name` outside literals), and not a statement exec_sql.rs
/// treats on its own (OPEN, FETCH, CLOSE, PREPARE, EXECUTE, CALL, GET,
/// SET RESULT SETS, INSERT ... ROWS, a positioned UPDATE or DELETE).
fn sql_step1(sql: &str) -> bool {
    // DECLARE ... CURSOR (S2.72): sql_statement answers SQLCODE 0 before
    // any host variable is bound (they are bound at OPEN).
    {
        let w: Vec<String> = sql
            .split_whitespace()
            .map(|x| x.to_ascii_uppercase())
            .collect();
        if w.first().map(String::as_str) == Some("DECLARE") && w.iter().any(|x| x == "CURSOR") {
            return true;
        }
    }
    let mut quote = None::<char>;
    let mut prev_colon = false;
    for c in sql.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            prev_colon = false;
            continue;
        }
        if prev_colon && (c.is_ascii_alphabetic() || "_#@$".contains(c)) {
            return false;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
        }
        prev_colon = c == ':';
    }
    let w: Vec<String> = sql
        .split_whitespace()
        .map(|x| x.to_ascii_uppercase())
        .collect();
    let first = w.first().map(String::as_str).unwrap_or("");
    let second = w.get(1).map(String::as_str).unwrap_or("");
    if matches!(
        first,
        "OPEN" | "FETCH" | "CLOSE" | "PREPARE" | "EXECUTE" | "CALL" | "GET" | ""
    ) || (first == "SET" && second == "RESULT")
        || (first == "INSERT" && w.iter().any(|x| x == "ROWS"))
        || w.windows(2)
            .any(|p| matches!(p, [a, b] if a == "CURRENT" && b == "OF"))
    {
        return false;
    }
    true
}

/// Whether an EXEC SQL is one S2.70 lowers: it names host variables, and
/// exec_sql.rs runs it through single_row (SELECT, VALUES or WITH with
/// INTO; SET :host) or bind_host and sql_change; not the statements step
/// 1 leaves out, not DDL or DECLARE (whose host variables are not bound),
/// not COMMIT, ROLLBACK or the precompiler's directives.
fn sql_hosts_kind(sql: &str) -> bool {
    if sql_step1(sql) {
        return false;
    }
    let w: Vec<String> = sql
        .split_whitespace()
        .map(|x| x.to_ascii_uppercase())
        .collect();
    let first = w.first().map(String::as_str).unwrap_or("");
    let second = w.get(1).map(String::as_str).unwrap_or("");
    if matches!(
        first,
        "OPEN"
            | "FETCH"
            | "CLOSE"
            | "PREPARE"
            | "EXECUTE"
            | "CALL"
            | "GET"
            | ""
            | "CREATE"
            | "DROP"
            | "LABEL"
            | "COMMENT"
            | "ALTER"
            | "RENAME"
            | "DECLARE"
            | "COMMIT"
            | "ROLLBACK"
            | "INCLUDE"
            | "WHENEVER"
            | "BEGIN"
            | "END"
            | "GRANT"
            | "REVOKE"
            | "LOCK"
            | "MERGE"
    ) || (first == "SET" && (second == "RESULT" || second == "OPTION"))
        || (first == "INSERT" && w.iter().any(|x| x == "ROWS"))
        || w.windows(2)
            .any(|p| matches!(p, [a, b] if a == "CURRENT" && b == "OF"))
    {
        return false;
    }
    true
}

/// A keyword's byte offset at depth 0 outside literals (exec_sql.rs
/// find_word).
fn sql_word_at(sql: &str, word: &str) -> Option<usize> {
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

/// exec_sql.rs host_list: `:a :ai, :b INDICATOR :bi, :c` as names and
/// indicators, upper case.
fn sql_host_list(text: &str) -> Option<Vec<(String, Option<String>)>> {
    let mut out = Vec::new();
    for part in text.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        let names: Vec<&str> = p
            .split(|c: char| c.is_whitespace() || c == ':')
            .filter(|w| !w.is_empty() && !w.eq_ignore_ascii_case("INDICATOR"))
            .collect();
        match names.as_slice() {
            [v] => out.push((v.to_ascii_uppercase(), None)),
            [v, i] => out.push((v.to_ascii_uppercase(), Some(i.to_ascii_uppercase()))),
            _ => return None,
        }
    }
    Some(out)
}

/// exec_sql.rs bind_host's scan: the text in pieces around its host
/// variables, and each one's name and indicator, in order.
fn sql_bind_scan(sql: &str) -> (Vec<String>, Vec<(String, Option<String>)>) {
    let cs: Vec<char> = sql.chars().collect();
    let name_char = |c: char| c.is_ascii_alphanumeric() || "_#@$.".contains(c);
    let read_name = |i: &mut usize| -> String {
        let start = *i;
        while cs.get(*i).is_some_and(|c| name_char(*c)) {
            *i = i.saturating_add(1);
        }
        cs.get(start..*i)
            .map(|s| s.iter().collect())
            .unwrap_or_default()
    };
    let mut pieces = vec![String::new()];
    let mut hosts = Vec::new();
    let mut quote = None;
    let mut i = 0usize;
    while let Some(&c) = cs.get(i) {
        let cur = pieces.last_mut();
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            if let Some(p) = cur {
                p.push(c);
            }
            i = i.saturating_add(1);
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            if let Some(p) = cur {
                p.push(c);
            }
            i = i.saturating_add(1);
            continue;
        }
        if c == ':'
            && cs
                .get(i.saturating_add(1))
                .is_some_and(|n| n.is_ascii_alphabetic() || "_#@$".contains(*n))
        {
            i = i.saturating_add(1);
            let name = read_name(&mut i);
            let mut j = i;
            while cs.get(j).is_some_and(|c| c.is_whitespace()) {
                j = j.saturating_add(1);
            }
            let rest: String = cs.get(j..).map(|s| s.iter().collect()).unwrap_or_default();
            let mut ind = None;
            if rest.starts_with(':') {
                let mut k = j.saturating_add(1);
                ind = Some(read_name(&mut k));
                i = k;
            } else if rest.len() > 10
                && rest
                    .get(..9)
                    .is_some_and(|w| w.eq_ignore_ascii_case("INDICATOR"))
            {
                let r2 = rest.get(9..).unwrap_or_default().trim_start();
                if let Some(after) = r2.strip_prefix(':') {
                    // bind_host adds the bytes skipped to a char index: the same here.
                    let skip = rest.len().saturating_sub(after.len());
                    let mut k = j.saturating_add(skip);
                    ind = Some(read_name(&mut k));
                    i = k;
                }
            }
            hosts.push((name, ind));
            pieces.push(String::new());
            continue;
        }
        if let Some(p) = cur {
            p.push(c);
        }
        i = i.saturating_add(1);
    }
    (pieces, hosts)
}

/// exec_sql.rs host_targets: a host variable as the fields it stands for
/// (a data structure's subfields), never a LOB's _LEN and _DATA.
fn sql_targets(cx: &Cx<'_>, name: &str) -> Option<Vec<Expr>> {
    let e = super::super::expr::parse(name).ok()?;
    if let Expr::Name { parts, args: None } = &e {
        let full = parts.join(".");
        if let Some(Sym::Ds(d)) = sym(cx, &full) {
            if *d.subfields == [format!("{full}_LEN"), format!("{full}_DATA")] {
                return None;
            }
            return Some(
                d.subfields
                    .iter()
                    .map(|sf| {
                        if d.qualified {
                            Expr::Name {
                                parts: vec![full.clone(), sf.clone()],
                                args: None,
                            }
                        } else {
                            Expr::name(sf)
                        }
                    })
                    .collect(),
            );
        }
        sym(cx, &full)?;
    }
    Some(vec![e])
}

/// A host variable's field (S2.70): numeric or character, as an operand.
fn sql_field(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    let o = variable(cx, e, used)?;
    match &o {
        ir::Operand::Var {
            ty:
                ir::Ty::Packed(..)
                | ir::Ty::Zoned(..)
                | ir::Ty::Binary(..)
                | ir::Ty::Int(_)
                | ir::Ty::Uns(_)
                | ir::Ty::Char(_)
                | ir::Ty::Varchar(_),
            based: None,
            ..
        } => Some(o),
        _ => None,
    }
}

/// The pieces, inputs and outputs of an EXEC SQL with host variables
/// (S2.70), as exec_sql.rs's sql_statement and bind_host take it apart.
#[allow(clippy::type_complexity)] // the four parts of ExecSqlHosts
fn sql_hosts(
    cx: &Cx<'_>,
    sql: &str,
    used: &mut Vec<String>,
) -> Option<(Vec<String>, Vec<ir::SqlIn>, Vec<ir::SqlOut>, bool)> {
    let sql = sql.trim();
    let w: Vec<String> = sql
        .split_whitespace()
        .map(|x| x.to_ascii_uppercase())
        .collect();
    let first = w.first().map(String::as_str).unwrap_or("");
    let second = w.get(1).map(String::as_str).unwrap_or("");
    // (prefix, text to bind, suffix, the INTO hosts)
    let (prefix, query, suffix, outs, into) = if first == "SET" && second.starts_with(':') {
        let eq = sql.find('=')?;
        let hosts = sql_host_list(sql.get(3..eq)?)?;
        let expr = sql.get(eq.checked_add(1)?..)?.trim().to_string();
        ("VALUES (".to_string(), expr, ")".to_string(), hosts, true)
    } else if matches!(first, "SELECT" | "VALUES" | "WITH") && sql_word_at(sql, "INTO").is_some() {
        let into = sql_word_at(sql, "INTO")?;
        let end = sql
            .get(into..)
            .and_then(|r| sql_word_at(r, "FROM"))
            .and_then(|e| into.checked_add(e))
            .unwrap_or(sql.len());
        let hosts = sql_host_list(sql.get(into.checked_add(4)?..end)?)?;
        let query = format!("{} {}", sql.get(..into)?, sql.get(end..)?);
        (String::new(), query, String::new(), hosts, true)
    } else {
        (
            String::new(),
            sql.to_string(),
            String::new(),
            Vec::new(),
            false,
        )
    };
    let (mut scanned, hosts) = sql_bind_scan(&query);
    if let Some(f) = scanned.first_mut() {
        f.insert_str(0, &prefix);
    }
    if let Some(l) = scanned.last_mut() {
        l.push_str(&suffix);
    }
    // Each host's fields; a data structure's are joined with ", "
    // (host_literal), with no indicator.
    let mut pieces = Vec::new();
    let mut inputs = Vec::new();
    let mut parts = scanned.into_iter();
    pieces.push(parts.next()?);
    for (name, ind) in hosts {
        let targets = sql_targets(cx, &name)?;
        if targets.len() > 1 && ind.is_some() {
            return None;
        }
        let ind = match ind {
            Some(i) => Some(num_variable(
                cx,
                &super::super::expr::parse(&i).ok()?,
                used,
            )?),
            None => None,
        };
        for (k, t) in targets.iter().enumerate() {
            if k > 0 {
                pieces.push(", ".to_string());
            }
            let value = sql_field(cx, t, used)?;
            let char_len = match &value {
                ir::Operand::Var {
                    ty: ir::Ty::Char(n),
                    ..
                } => Some(*n),
                _ => None,
            };
            inputs.push(ir::SqlIn {
                value,
                char_len,
                ind: ind.clone(),
            });
        }
        pieces.push(parts.next()?);
    }
    let mut outputs = Vec::new();
    for (name, ind) in outs {
        let ind = match ind {
            Some(i) => {
                let o = num_variable(cx, &super::super::expr::parse(&i).ok()?, used)?;
                if !matches!(
                    o,
                    ir::Operand::Var {
                        index: None,
                        based: None,
                        ..
                    }
                ) {
                    return None;
                }
                Some(o)
            }
            None => None,
        };
        for t in sql_targets(cx, &name)? {
            let target = sql_field(cx, &t, used)?;
            if !matches!(target, ir::Operand::Var { index: None, .. }) {
                return None;
            }
            let numeric = !is_char(&target);
            outputs.push(ir::SqlOut {
                target,
                numeric,
                ind: ind.clone(),
            });
        }
    }
    used.push(format!("sql pieces {pieces:?}"));
    Some((pieces, inputs, outputs, into))
}

/// The SQLCA's fields exec_sql.rs exec_sql sets (S2.71): SQLCODE,
/// SQLSTATE, SQLERRMC, SQLERRD(3) and SQLERRD(5).
fn sqlca_ops(cx: &Cx<'_>, used: &mut Vec<String>) -> Option<[ir::Operand; 5]> {
    let errd = |k: i64, used: &mut Vec<String>| {
        variable(
            cx,
            &Expr::Name {
                parts: vec!["SQLERRD".into()],
                args: Some(vec![Expr::Num(super::super::dec::Dec::from_i64(k))]),
            },
            used,
        )
    };
    Some([
        variable(cx, &Expr::name("SQLCODE"), used)?,
        variable(cx, &Expr::name("SQLSTATE"), used)?,
        variable(cx, &Expr::name("SQLERRMC"), used)?,
        errd(3, used)?,
        errd(5, used)?,
    ])
}

/// What S2.71 lowers.
#[derive(Clone, Copy, Debug)]
enum SqlKind {
    Dyn(ir::SqlDyn),
    Diagnostics,
}

/// GET DIAGNOSTICS as exec_sql.rs exec_sql tells it (GET with
/// DIAGNOSTICS in the first three words), or sql_statement's PREPARE,
/// EXECUTE IMMEDIATE and EXECUTE (not one that names a cursor's FOR, not
/// EXECUTE of a CALL: the statement's kind is only known at run time, so
/// the runtime runs either as sql_statement does).
fn sql_dyn_kind(sql: &str) -> Option<SqlKind> {
    let w: Vec<String> = sql
        .split_whitespace()
        .map(|x| x.to_ascii_uppercase())
        .collect();
    let first = w.first().map(String::as_str).unwrap_or("");
    let second = w.get(1).map(String::as_str).unwrap_or("");
    if first == "GET" && w.iter().take(3).any(|x| x == "DIAGNOSTICS") {
        return Some(SqlKind::Diagnostics);
    }
    match first {
        "PREPARE" => Some(SqlKind::Dyn(ir::SqlDyn::Prepare)),
        "EXECUTE" if second == "IMMEDIATE" => Some(SqlKind::Dyn(ir::SqlDyn::Immediate)),
        "EXECUTE" => Some(SqlKind::Dyn(ir::SqlDyn::Execute)),
        _ => None,
    }
}

/// A statement's text (exec_sql.rs statement_text): `:name` or an
/// expression, here a character or varying field.
fn sql_text_operand(cx: &Cx<'_>, from: &str, used: &mut Vec<String>) -> Option<ir::Operand> {
    let from = from.trim();
    let src = from.strip_prefix(':').map(str::trim).unwrap_or(from);
    let e = super::super::expr::parse(src).ok()?;
    // A field (`:name`): character or varying, as sql_field takes it.
    let o = sql_field(cx, &e, used)?;
    is_char(&o).then_some(o)
}

/// PREPARE's name and text, EXECUTE IMMEDIATE's text, or EXECUTE's name
/// and USING host variables (S2.71), as sql_statement and using_values
/// take them apart.
#[allow(clippy::type_complexity)] // the three parts of ExecSqlDyn
fn sql_dyn(
    cx: &Cx<'_>,
    sql: &str,
    kind: ir::SqlDyn,
    used: &mut Vec<String>,
) -> Option<(String, Option<ir::Operand>, Vec<Vec<ir::SqlIn>>)> {
    let sql = sql.trim();
    let w: Vec<String> = sql
        .split_whitespace()
        .map(|x| x.to_ascii_uppercase())
        .collect();
    let second = w.get(1).cloned().unwrap_or_default();
    match kind {
        ir::SqlDyn::Prepare => {
            let at = sql_word_at(sql, "FROM")?;
            let text = sql_text_operand(cx, sql.get(at.checked_add(4)?..)?, used)?;
            Some((second, Some(text), Vec::new()))
        }
        ir::SqlDyn::Immediate => {
            let at = sql_word_at(sql, "IMMEDIATE")?;
            let text = sql_text_operand(cx, sql.get(at.checked_add(9)?..)?, used)?;
            Some((String::new(), Some(text), Vec::new()))
        }
        ir::SqlDyn::Execute => {
            let mut groups = Vec::new();
            if let Some(at) = sql_word_at(sql, "USING") {
                for (name, ind) in sql_host_list(sql.get(at.checked_add(5)?..)?)? {
                    let targets = sql_targets(cx, &name)?;
                    if targets.len() > 1 && ind.is_some() {
                        return None;
                    }
                    let ind = match ind {
                        Some(i) => Some(num_variable(
                            cx,
                            &super::super::expr::parse(&i).ok()?,
                            used,
                        )?),
                        None => None,
                    };
                    let mut g = Vec::new();
                    for t in &targets {
                        let value = sql_field(cx, t, used)?;
                        let char_len = match &value {
                            ir::Operand::Var {
                                ty: ir::Ty::Char(n),
                                ..
                            } => Some(*n),
                            _ => None,
                        };
                        g.push(ir::SqlIn {
                            value,
                            char_len,
                            ind: ind.clone(),
                        });
                    }
                    groups.push(g);
                }
            }
            Some((second, None, groups))
        }
    }
}

/// GET DIAGNOSTICS's items (S2.71), as exec_sql.rs get_diagnostics reads
/// them: CONDITION (or EXCEPTION) n or not, then `:target = ITEM`, ...;
/// lowering takes only the items get_diagnostics supports, each with the
/// fields of its target (host_targets).
#[allow(clippy::type_complexity)] // the parts of SqlDiag
fn sql_diag(
    cx: &Cx<'_>,
    sql: &str,
    used: &mut Vec<String>,
) -> Option<(bool, Vec<(String, Vec<ir::Operand>)>)> {
    let sql = sql.trim();
    let up = sql.to_ascii_uppercase();
    let at = up.find("DIAGNOSTICS")?.checked_add("DIAGNOSTICS".len())?;
    let mut rest = sql.get(at..)?.trim().trim_end_matches(';').trim();
    let first = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    let condition = matches!(first.as_str(), "CONDITION" | "EXCEPTION");
    if condition {
        let mut it = rest.splitn(3, char::is_whitespace);
        it.next();
        it.next();
        rest = it.next().unwrap_or("").trim();
    }
    let mut items = Vec::new();
    for item in rest.split(',') {
        let (target, name) = item.split_once('=')?;
        let target = target.trim().trim_start_matches(':');
        let name = name.trim().to_ascii_uppercase();
        let known = match condition {
            false => name == "ROW_COUNT",
            true => matches!(
                name.as_str(),
                "MESSAGE_TEXT"
                    | "MESSAGE_LENGTH"
                    | "RETURNED_SQLSTATE"
                    | "DB2_RETURNED_SQLCODE"
                    | "MESSAGE_ID"
                    | "DB2_MESSAGE_ID"
            ),
        };
        if !known {
            return None;
        }
        let mut targets = Vec::new();
        for t in sql_targets(cx, target)? {
            targets.push(sql_field(cx, &t, used)?);
        }
        items.push((name, targets));
    }
    Some((condition, items))
}

/// OPEN, FETCH and CLOSE, as exec_sql.rs sql_statement tells them (S2.72).
fn sql_cursor_kind(sql: &str) -> bool {
    let first = sql
        .split_whitespace()
        .next()
        .map(|x| x.to_ascii_uppercase())
        .unwrap_or_default();
    matches!(first.as_str(), "OPEN" | "FETCH" | "CLOSE")
}

/// A host variable's inputs (S2.72), as host_literal makes them: a data
/// structure's fields (no indicator then), or the field with its
/// indicator.
fn sql_host_inputs(
    cx: &Cx<'_>,
    name: &str,
    ind: Option<String>,
    used: &mut Vec<String>,
) -> Option<Vec<ir::SqlIn>> {
    let targets = sql_targets(cx, name)?;
    if targets.len() > 1 && ind.is_some() {
        return None;
    }
    let ind = match ind {
        Some(i) => Some(num_variable(
            cx,
            &super::super::expr::parse(&i).ok()?,
            used,
        )?),
        None => None,
    };
    let mut out = Vec::new();
    for t in &targets {
        let value = sql_field(cx, t, used)?;
        let char_len = match &value {
            ir::Operand::Var {
                ty: ir::Ty::Char(n),
                ..
            } => Some(*n),
            _ => None,
        };
        out.push(ir::SqlIn {
            value,
            char_len,
            ind: ind.clone(),
        });
    }
    Some(out)
}

/// OPEN, FETCH or CLOSE taken apart as exec_sql.rs sql_open, sql_fetch
/// and sql_statement's CLOSE do (S2.72). Lowering keeps in the
/// interpreter a cursor not declared in the program, a multiple-row FETCH
/// (ROWS), and INTO targets with an index.
fn sql_cursor(
    cx: &Cx<'_>,
    sql: &str,
    sqlca: [ir::Operand; 5],
    used: &mut Vec<String>,
) -> Option<ir::Lowered> {
    let sql = sql.trim();
    let words = |t: &str| -> Vec<String> {
        t.split_whitespace()
            .map(|x| x.to_ascii_uppercase())
            .collect()
    };
    let w = words(sql);
    let first = w.first().cloned().unwrap_or_default();
    let second = w.get(1).cloned().unwrap_or_default();
    match first.as_str() {
        "CLOSE" => {
            let name = second.trim_end_matches(';').to_string();
            used.push(format!("sql close {name}"));
            Some(ir::Lowered::SqlClose {
                sqlca: Box::new(sqlca),
                name,
            })
        }
        "OPEN" => {
            let name = second;
            let decl = cx.prog.cursors.get(&name)?;
            let (pieces, inputs, prepared, using) = match decl {
                super::super::program::Cursor::Select(q) => {
                    let (scanned, hosts) = sql_bind_scan(q);
                    let mut pieces = Vec::new();
                    let mut inputs = Vec::new();
                    let mut parts = scanned.into_iter();
                    pieces.push(parts.next()?);
                    for (hn, ind) in hosts {
                        let ins = sql_host_inputs(cx, &hn, ind, used)?;
                        for k in 0..ins.len() {
                            if k > 0 {
                                pieces.push(", ".to_string());
                            }
                        }
                        inputs.extend(ins);
                        pieces.push(parts.next()?);
                    }
                    (pieces, inputs, None, Vec::new())
                }
                super::super::program::Cursor::Prepared(st) => {
                    let mut using = Vec::new();
                    if let Some(at) = sql_word_at(sql, "USING") {
                        for (hn, ind) in sql_host_list(sql.get(at.checked_add(5)?..)?)? {
                            using.push(sql_host_inputs(cx, &hn, ind, used)?);
                        }
                    }
                    (Vec::new(), Vec::new(), Some(st.clone()), using)
                }
            };
            used.push(format!("sql open {name} {} inputs", inputs.len()));
            Some(ir::Lowered::SqlOpen {
                sqlca: Box::new(sqlca),
                name,
                pieces,
                inputs,
                prepared,
                using,
            })
        }
        "FETCH" => {
            let into = sql_word_at(sql, "INTO");
            let head = sql.get(..into.unwrap_or(sql.len()))?;
            let w = words(head);
            let mut k = 1usize;
            let mut orient = ("NEXT".to_string(), 0i64);
            if let Some(o) = w.get(k) {
                match o.as_str() {
                    "NEXT" | "PRIOR" | "FIRST" | "LAST" | "BEFORE" | "AFTER" | "CURRENT" => {
                        orient = (o.clone(), 0);
                        k = k.saturating_add(1);
                    }
                    "RELATIVE" | "ABSOLUTE" => {
                        let n = w
                            .get(k.saturating_add(1))
                            .and_then(|x| x.parse().ok())
                            .unwrap_or(0);
                        orient = (o.clone(), n);
                        k = k.saturating_add(2);
                    }
                    _ => {}
                }
            }
            if w.get(k).map(String::as_str) == Some("FROM") {
                k = k.saturating_add(1);
            }
            let name = w.get(k).cloned().unwrap_or_default();
            if w.iter().any(|x| x == "ROWS") {
                // FOR n ROWS (S2.74), as fetch_rows reads it: the word
                // before ROWS, then INTO a host structure array.
                let count = w
                    .iter()
                    .position(|x| x == "ROWS")
                    .and_then(|i| i.checked_sub(1))
                    .and_then(|i| w.get(i))
                    .cloned()
                    .unwrap_or_default();
                let count = sql_count_of(cx, &count, used)?;
                let at = into?;
                let host = sql
                    .get(at.checked_add(4)?..)?
                    .trim()
                    .trim_end_matches(';')
                    .trim()
                    .trim_start_matches(':')
                    .to_ascii_uppercase();
                let (dim, fields) = sql_row_fields(cx, &host, used)?;
                used.push(format!("sql fetch {name} rows into {host}"));
                return Some(ir::Lowered::SqlFetchRows {
                    sqlca: Box::new(sqlca),
                    name,
                    orient: orient.0,
                    n: orient.1,
                    count,
                    dim,
                    host,
                    fields,
                });
            }
            let outputs = match into {
                None => None,
                Some(at) => {
                    let mut outs = Vec::new();
                    for (hn, ind) in sql_host_list(sql.get(at.checked_add(4)?..)?)? {
                        let ind = match ind {
                            Some(i) => {
                                let o =
                                    num_variable(cx, &super::super::expr::parse(&i).ok()?, used)?;
                                if !matches!(
                                    o,
                                    ir::Operand::Var {
                                        index: None,
                                        based: None,
                                        ..
                                    }
                                ) {
                                    return None;
                                }
                                Some(o)
                            }
                            None => None,
                        };
                        for t in sql_targets(cx, &hn)? {
                            let target = sql_field(cx, &t, used)?;
                            if !matches!(target, ir::Operand::Var { index: None, .. }) {
                                return None;
                            }
                            let numeric = !is_char(&target);
                            outs.push(ir::SqlOut {
                                target,
                                numeric,
                                ind: ind.clone(),
                            });
                        }
                    }
                    Some(outs)
                }
            };
            used.push(format!("sql fetch {name} {}", orient.0));
            Some(ir::Lowered::SqlFetch {
                sqlca: Box::new(sqlca),
                name,
                orient: orient.0,
                n: orient.1,
                outputs,
            })
        }
        _ => None,
    }
}

/// UPDATE or DELETE WHERE CURRENT OF, as exec_sql.rs sql_statement tells
/// it (CURRENT a keyword at depth 0, then the words CURRENT OF) (S2.73).
fn sql_positioned_kind(sql: &str) -> bool {
    let sql = sql.trim();
    let w: Vec<String> = sql
        .split_whitespace()
        .map(|x| x.to_ascii_uppercase())
        .collect();
    matches!(w.first().map(String::as_str), Some("UPDATE" | "DELETE"))
        && sql_word_at(sql, "CURRENT").is_some()
        && w.windows(2)
            .any(|p| matches!(p, [a, b] if a == "CURRENT" && b == "OF"))
}

/// A positioned UPDATE or DELETE taken apart as exec_sql.rs
/// positioned_change does (S2.73): the cursor after CURRENT OF, the table
/// (UPDATE table, DELETE FROM table), and the text before WHERE with its
/// host variables (bind_host).
fn sql_positioned(
    cx: &Cx<'_>,
    sql: &str,
    sqlca: [ir::Operand; 5],
    used: &mut Vec<String>,
) -> Option<ir::Lowered> {
    let sql = sql.trim();
    let w: Vec<String> = sql
        .split_whitespace()
        .map(|x| x.to_ascii_uppercase())
        .collect();
    let at = w
        .windows(2)
        .position(|p| matches!(p, [a, b] if a == "CURRENT" && b == "OF"))
        .unwrap_or(0);
    let cursor = w
        .get(at.saturating_add(2))
        .map(|n| n.trim_end_matches(';').to_string())
        .unwrap_or_default();
    let table = match w.first().map(String::as_str) {
        Some("UPDATE") => w.get(1),
        Some("DELETE") => w.get(2),
        _ => None,
    }
    .cloned()
    .unwrap_or_default();
    let where_at = sql_word_at(sql, "WHERE").unwrap_or(sql.len());
    let head = sql.get(..where_at)?.trim_end();
    let (scanned, hosts) = sql_bind_scan(head);
    let mut pieces = Vec::new();
    let mut inputs = Vec::new();
    let mut parts = scanned.into_iter();
    pieces.push(parts.next()?);
    for (hn, ind) in hosts {
        let ins = sql_host_inputs(cx, &hn, ind, used)?;
        for _ in 1..ins.len() {
            pieces.push(", ".to_string());
        }
        inputs.extend(ins);
        pieces.push(parts.next()?);
    }
    used.push(format!("sql positioned {cursor} {table}"));
    Some(ir::Lowered::SqlPositioned {
        sqlca: Box::new(sqlca),
        cursor,
        table,
        pieces,
        inputs,
    })
}

/// A multiple-row statement's row count (S2.74): `:host`, a numeric field,
/// or a number as written (exec_sql.rs: Dec::parse).
fn sql_count_of(cx: &Cx<'_>, count: &str, used: &mut Vec<String>) -> Option<ir::SqlCount> {
    match count.strip_prefix(':') {
        Some(h) => Some(ir::SqlCount::Var(num_variable(
            cx,
            &super::super::expr::parse(h).ok()?,
            used,
        )?)),
        None => {
            super::super::dec::Dec::parse(count).ok()?;
            Some(ir::SqlCount::Lit(count.to_string()))
        }
    }
}

/// A host structure array's elements and subfields (S2.74): a global data
/// structure with DIM (so qualified), not based, not of multiple
/// occurrences; each subfield character, varying or numeric, `HOST.SF`
/// at its first element's place with the structure's size between
/// elements.
fn sql_row_fields(
    cx: &Cx<'_>,
    host: &str,
    used: &mut Vec<String>,
) -> Option<(u32, Vec<ir::SqlRowField>)> {
    let prog = cx.prog;
    let Some(Sym::Ds(d)) = sym(cx, host) else {
        return None;
    };
    if d.local
        || d.dim == 0
        || !d.qualified
        || prog.based.contains_key(&d.buf)
        || prog.occurs.iter().any(|o| o.buf == d.buf)
        || prog.data_imports.iter().any(|(n, _)| n == host)
        || prog.data_exports.iter().any(|(n, _)| n == host)
    {
        return None;
    }
    let mut fields = Vec::new();
    for sf in d.subfields.iter() {
        let name = format!("{host}.{sf}");
        let Some(Sym::Var(v)) = sym(cx, &name) else {
            return None;
        };
        if v.local || v.dim != 0 || v.buf != d.buf || prog.varying.contains_key(&name) {
            return None;
        }
        let (ty, numeric) = match v.dtype {
            DType::Packed(p, q) => (ir::Ty::Packed(p, q), true),
            DType::Zoned(p, q) => (ir::Ty::Zoned(p, q), true),
            DType::Binary(p, q) => (ir::Ty::Binary(p, q), true),
            DType::Int(n) => (ir::Ty::Int(n), true),
            DType::Uns(n) => (ir::Ty::Uns(n), true),
            DType::Char(n) => (ir::Ty::Char(n), false),
            DType::Varchar(n) => (ir::Ty::Varchar(n), false),
            _ => return None,
        };
        used.push(format!(
            "layout {name} {} {} {:?} stride {} dim {}",
            v.buf, v.off, v.dtype, d.size, d.dim
        ));
        fields.push(ir::SqlRowField {
            buf: v.buf,
            off: v.off,
            stride: d.size,
            ty,
            numeric,
        });
    }
    Some((d.dim, fields))
}

/// INSERT with ROWS, as exec_sql.rs sql_statement tells it (S2.74).
fn sql_insert_rows_kind(sql: &str) -> bool {
    let w: Vec<String> = sql
        .split_whitespace()
        .map(|x| x.to_ascii_uppercase())
        .collect();
    w.first().map(String::as_str) == Some("INSERT") && w.iter().any(|x| x == "ROWS")
}

/// INSERT ... n ROWS VALUES (:dsarray) taken apart as exec_sql.rs
/// sql_insert_rows does (S2.74): the text before the count (with no host
/// variables here), the count, VALUES or values, the host structure
/// array.
fn sql_insert_rows(
    cx: &Cx<'_>,
    sql: &str,
    sqlca: [ir::Operand; 5],
    used: &mut Vec<String>,
) -> Option<ir::Lowered> {
    let sql = sql.trim();
    let at = sql_word_at(sql, "ROWS")?;
    let head = sql.get(..at)?.trim_end();
    let i = head.rfind(char::is_whitespace)?;
    let target = head.get(..i)?;
    let count = head.get(i.checked_add(1)?..)?.trim();
    if !sql_bind_scan(target).1.is_empty() {
        return None;
    }
    let count = sql_count_of(cx, count, used)?;
    let rest = sql.get(at.checked_add(4)?..)?.trim_start();
    let vals = rest
        .strip_prefix("VALUES")
        .or_else(|| rest.strip_prefix("values"))
        .map(str::trim)?;
    let host = vals
        .trim_start_matches('(')
        .trim_end_matches(';')
        .trim_end_matches(')')
        .trim()
        .trim_start_matches(':')
        .to_ascii_uppercase();
    let (dim, fields) = sql_row_fields(cx, &host, used)?;
    used.push(format!("sql insert rows {host}"));
    Some(ir::Lowered::SqlInsertRows {
        sqlca: Box::new(sqlca),
        target: target.to_string(),
        count,
        dim,
        host,
        fields,
    })
}

/// CALL, as exec_sql.rs sql_statement tells it (S2.75).
fn sql_call_kind(sql: &str) -> bool {
    sql.split_whitespace()
        .next()
        .is_some_and(|w| w.eq_ignore_ascii_case("CALL"))
}

/// A host variable of a CALL argument (S2.75): `hv`, `hv ind` or
/// `hv INDICATOR ind`, as sql_call splits it; a field, not a data
/// structure.
fn sql_call_host(
    cx: &Cx<'_>,
    rest: &str,
    used: &mut Vec<String>,
) -> Option<(ir::SqlIn, ir::SqlOut)> {
    let parts: Vec<&str> = rest
        .split_whitespace()
        .filter(|w| !w.eq_ignore_ascii_case("INDICATOR"))
        .collect();
    let hv = parts.first().copied().unwrap_or("");
    if hv.contains(':') || parts.len() > 2 {
        return None;
    }
    let ind = parts.get(1).map(|i| i.trim_start_matches(':').to_string());
    let targets = sql_targets(cx, hv)?;
    let [t] = targets.as_slice() else {
        return None;
    };
    let value = sql_field(cx, t, used)?;
    if !matches!(
        value,
        ir::Operand::Var {
            index: None,
            based: None,
            ..
        }
    ) {
        return None;
    }
    let ind = match ind {
        Some(i) => {
            let o = num_variable(cx, &super::super::expr::parse(&i).ok()?, used)?;
            if !matches!(
                o,
                ir::Operand::Var {
                    index: None,
                    based: None,
                    ..
                }
            ) {
                return None;
            }
            Some(o)
        }
        None => None,
    };
    let char_len = match &value {
        ir::Operand::Var {
            ty: ir::Ty::Char(n),
            ..
        } => Some(*n),
        _ => None,
    };
    let numeric = !is_char(&value);
    Some((
        ir::SqlIn {
            value: value.clone(),
            char_len,
            ind: ind.clone(),
        },
        ir::SqlOut {
            target: value,
            numeric,
            ind,
        },
    ))
}

/// A CALL taken apart as exec_sql.rs sql_call does (S2.75): the routine
/// and its arguments (sqlpl::parse_call); an argument by name keeps its
/// `NAME => ` and is only an input; a positional host variable is also
/// where an OUT or INOUT parameter's value goes.
fn sql_call(
    cx: &Cx<'_>,
    sql: &str,
    sqlca: [ir::Operand; 5],
    used: &mut Vec<String>,
) -> Option<ir::Lowered> {
    let (name, args) = crate::sqlpl::parse_call(sql.trim()).ok()??;
    let mut out = Vec::new();
    for a in &args {
        let (kw, t) = match a.split_once("=>") {
            Some((k, e)) if !k.trim().is_empty() && !k.contains('\'') => {
                (format!("{} => ", k.trim()), e.trim())
            }
            _ => (String::new(), a.trim()),
        };
        match t.strip_prefix(':') {
            Some(rest) => {
                let (input, o) = sql_call_host(cx, rest, used)?;
                let positional = kw.is_empty();
                out.push(ir::SqlArg::Host {
                    kw,
                    input: Box::new(input),
                    out: positional.then(|| Box::new(o)),
                });
            }
            None => out.push(ir::SqlArg::Text(format!("{kw}{t}"))),
        }
    }
    used.push(format!("sql call {name} {} args", out.len()));
    Some(ir::Lowered::SqlCall {
        sqlca: Box::new(sqlca),
        name,
        args: out,
    })
}

/// interp.rs expr_name: EXCEPT's factor 2 as a name.
fn except_name(e: &Option<Expr>) -> Option<String> {
    match e {
        Some(Expr::Name { parts, args: None }) => Some(parts.join(".")),
        _ => None,
    }
}

/// The program fields the output records an EXCEPT may print name
/// (S2.78): its exception records, and the overflow lines of their
/// printers when one fetches overflow (any heading, detail or total line
/// of the file). Every one of those records must be to a printer file;
/// each field a character, varying or numeric field with no index (a
/// whole data structure as its bytes, not cleared after); an edit word
/// that is a name, a named constant of the program.
fn except_fields(
    cx: &Cx<'_>,
    name: Option<&str>,
    used: &mut Vec<String>,
) -> Option<Vec<(String, ir::Operand)>> {
    let prog = cx.prog;
    let records: Vec<&super::super::ast::OutRec> = prog
        .outputs
        .iter()
        .filter(|r| r.kind == 'E' && r.name.as_deref() == name)
        .collect();
    let mut all: Vec<&super::super::ast::OutRec> = records.clone();
    for r in &records {
        if r.fetch_overflow {
            all.extend(
                prog.outputs
                    .iter()
                    .filter(|o| o.file == r.file && matches!(o.kind, 'H' | 'D' | 'T')),
            );
        }
    }
    let mut fields: Vec<(String, ir::Operand)> = Vec::new();
    for r in all {
        let f = prog.files.get(prog.file_index(&r.file)?)?;
        if f.decl.device != "PRINTER" {
            return None;
        }
        for of in &r.fields {
            if let Some(w) = &of.edit_word {
                if let Some(n) = w.strip_prefix('\u{1}') {
                    if !matches!(sym(cx, n), Some(Sym::Const(_)))
                        || !matches!(prog.syms.get(n), Some(Sym::Const(_)))
                    {
                        return None;
                    }
                }
            }
            if of.literal.is_some() {
                continue;
            }
            let n = of.name.clone().unwrap_or_default();
            if matches!(
                n.as_str(),
                "PAGE"
                    | "PAGE1"
                    | "PAGE2"
                    | "PAGE3"
                    | "PAGE4"
                    | "PAGE5"
                    | "PAGE6"
                    | "PAGE7"
                    | "UDATE"
                    | "*DATE"
            ) || fields.iter().any(|(k, _)| *k == n)
            {
                continue;
            }
            if of.blank_after && matches!(sym(cx, &n), Some(Sym::Ds(_))) {
                return None;
            }
            let o = sql_field(cx, &Expr::name(&n), used)?;
            if !matches!(
                o,
                ir::Operand::Var {
                    index: None,
                    based: None,
                    ..
                }
            ) {
                return None;
            }
            fields.push((n, o));
        }
    }
    Some(fields)
}

/// The externally described printer file a WRITE names by its record
/// format, and the format (S2.79): a PRINTER file of the program (not of
/// a procedure, as file_read asks) with its DDS.
fn printer_format<'p>(
    cx: &Cx<'p>,
    fmt: &str,
) -> Option<(
    &'p super::super::program::FileInfo,
    &'p crate::prtf::PRecord,
)> {
    let prog = cx.prog;
    if prog.files.iter().any(|f| f.decl.proc.is_some()) {
        return None;
    }
    let i = prog
        .files
        .iter()
        .position(|f| f.decl.name == fmt || f.format == fmt)
        .or_else(|| prog.file_index(fmt))?;
    let fi = prog.files.get(i)?;
    if fi.decl.device != "PRINTER" {
        return None;
    }
    let rec = fi.printer.as_ref()?.record(fmt)?;
    Some((fi, rec))
}

/// The program fields a printer record format prints (S2.79), character,
/// varying or numeric with no index (not a date or time field), and the
/// INDDS data structure when the file names one.
#[allow(clippy::type_complexity)] // the two parts of PrinterWrite
fn printer_fields(
    cx: &Cx<'_>,
    fmt: &str,
    used: &mut Vec<String>,
) -> Option<(Vec<(String, ir::Operand)>, Option<ir::Operand>)> {
    let (fi, rec) = printer_format(cx, fmt)?;
    let mut fields: Vec<(String, ir::Operand)> = Vec::new();
    for f in &rec.fields {
        if !matches!(f.content, crate::prtf::Content::Field) {
            continue;
        }
        if matches!(f.dtype, 'L' | 'T') {
            return None;
        }
        if fields.iter().any(|(k, _)| *k == f.name) {
            continue;
        }
        let o = sql_field(cx, &Expr::name(&f.name), used)?;
        if !matches!(
            o,
            ir::Operand::Var {
                index: None,
                based: None,
                ..
            }
        ) {
            return None;
        }
        fields.push((f.name.clone(), o));
    }
    let indds = match super::super::ast::keyword(&fi.decl.keywords, "INDDS")
        .and_then(|k| k.args.first())
        .map(|a| a.trim().to_ascii_uppercase())
    {
        Some(n) => match sym(cx, &n) {
            Some(Sym::Ds(_)) => {
                let o = variable(cx, &Expr::name(&n), used)?;
                if !matches!(
                    o,
                    ir::Operand::Var {
                        index: None,
                        based: None,
                        ..
                    }
                ) {
                    return None;
                }
                Some(o)
            }
            _ => None,
        },
        None => None,
    };
    Some((fields, indds))
}

/// A printer file of the program by its name (S2.79), as file_idx finds
/// it (not of a procedure).
fn printer_file(cx: &Cx<'_>, file: &str) -> bool {
    let prog = cx.prog;
    if prog.files.iter().any(|f| f.decl.proc.is_some()) {
        return false;
    }
    prog.files
        .iter()
        .position(|f| f.decl.name == file || f.format == file)
        .or_else(|| prog.file_index(file))
        .and_then(|i| prog.files.get(i))
        .is_some_and(|f| f.decl.device == "PRINTER")
}

/// A data area's field (S2.80): a character, varying or numeric field, or
/// a data structure (its bytes), with no index and not based.
fn data_area_field(cx: &Cx<'_>, name: &str, used: &mut Vec<String>) -> Option<ir::Operand> {
    let e = Expr::name(name);
    let o = match sym(cx, name) {
        Some(Sym::Ds(_)) => variable(cx, &e, used)?,
        _ => sql_field(cx, &e, used)?,
    };
    matches!(
        o,
        ir::Operand::Var {
            index: None,
            based: None,
            ..
        }
    )
    .then_some(o)
}

/// ops.rs split: `a:b` (a *PAIR) as its two parts, *BLANKS first as none.
fn str_split(e: &Option<Expr>) -> (Option<Expr>, Option<Expr>) {
    match e {
        Some(Expr::Bif(b, args)) if b == super::super::parse::PAIR && args.len() == 2 => {
            let first = match args.first() {
                Some(Expr::Fig(Fig::Blanks)) | None => None,
                Some(x) => Some(x.clone()),
            };
            (first, args.get(1).cloned())
        }
        other => (other.clone(), None),
    }
}

/// A string operation's character factor (S2.83): a character or
/// hexadecimal literal, or a character or varying field with no index
/// (chars_of: its value's bytes). A figurative constant or a number is
/// not lowered.
fn str_chars(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    match e {
        Expr::Str(b) | Expr::Hex(b) => Some(ir::Operand::Str(b.clone())),
        Expr::Name { .. } => {
            let o = sql_field(cx, e, used)?;
            (is_char(&o)
                && matches!(
                    o,
                    ir::Operand::Var {
                        index: None,
                        based: None,
                        ..
                    }
                ))
            .then_some(o)
        }
        _ => None,
    }
}

/// A string operation's numeric factor (S2.83): a number, or a numeric
/// field with no index.
fn str_num(cx: &Cx<'_>, e: &Expr, used: &mut Vec<String>) -> Option<ir::Operand> {
    match e {
        Expr::Num(_) => operand(cx, e, used),
        Expr::Name { .. } => {
            let o = sql_field(cx, e, used)?;
            (!is_char(&o)
                && matches!(
                    o,
                    ir::Operand::Var {
                        index: None,
                        based: None,
                        ..
                    }
                ))
            .then_some(o)
        }
        _ => None,
    }
}

/// SCAN, CHECK, CHECKR, SUBST, XLATE or CAT taken apart as ops.rs
/// string_op takes it (S2.83). The result of SCAN, CHECK and CHECKR is a
/// numeric field or a whole numeric array (a global one), or none; that
/// of the others a character or varying field.
#[allow(clippy::too_many_arguments)] // the parts of a fixed-form operation
fn str_op(
    cx: &Cx<'_>,
    code: &str,
    f1: &Option<Expr>,
    f2: &Option<Expr>,
    result: &Option<Expr>,
    ext: &super::super::ast::Ext,
    ind: &super::super::ast::ResultInd,
    used: &mut Vec<String>,
) -> Option<ir::Lowered> {
    let c = match code {
        "SCAN" => ir::StrCode::Scan,
        "CHECK" => ir::StrCode::Check,
        "CHECKR" => ir::StrCode::CheckR,
        "SUBST" => ir::StrCode::Subst,
        "XLATE" => ir::StrCode::Xlate,
        _ => ir::StrCode::Cat,
    };
    let (mut o1, mut o1c, mut o1n) = (None, None, None);
    match c {
        ir::StrCode::Scan => {
            let (a, b) = str_split(f1);
            o1 = Some(str_chars(cx, &a?, used)?);
            o1n = match b {
                Some(b) => Some(str_num(cx, &b, used)?),
                None => None,
            };
        }
        ir::StrCode::Check | ir::StrCode::CheckR => {
            o1 = Some(str_chars(cx, f1.as_ref()?, used)?);
        }
        ir::StrCode::Subst => {
            o1n = match f1 {
                Some(l) => Some(str_num(cx, l, used)?),
                None => None,
            };
        }
        ir::StrCode::Xlate => {
            let (a, b) = str_split(f1);
            o1 = Some(str_chars(cx, &a?, used)?);
            o1c = Some(str_chars(cx, &b?, used)?);
        }
        ir::StrCode::Cat => {
            o1 = match f1 {
                Some(x) => Some(str_chars(cx, x, used)?),
                None => None,
            };
        }
    }
    let (b, bn) = str_split(f2);
    let o2 = str_chars(cx, &b?, used)?;
    let o2n = match bn {
        Some(n) => Some(str_num(cx, &n, used)?),
        None => None,
    };
    let res = match c {
        ir::StrCode::Scan | ir::StrCode::Check | ir::StrCode::CheckR => match result {
            None => ir::StrRes::None,
            Some(r @ Expr::Name { parts, args: None }) => {
                let name = parts.join(".");
                match sym(cx, &name) {
                    Some(Sym::Var(v)) if v.dim > 0 => {
                        if v.local || cx.prog.varying.contains_key(&name) {
                            return None;
                        }
                        let ty = match v.dtype {
                            DType::Packed(d, s) => ir::Ty::Packed(d, s),
                            DType::Zoned(d, s) => ir::Ty::Zoned(d, s),
                            DType::Binary(d, s) => ir::Ty::Binary(d, s),
                            DType::Int(n) => ir::Ty::Int(n),
                            DType::Uns(n) => ir::Ty::Uns(n),
                            _ => return None,
                        };
                        ir::StrRes::Positions {
                            target: ir::Operand::Var {
                                name: name.clone(),
                                buf: v.buf,
                                off: v.off,
                                ty,
                                local: false,
                                index: None,
                                based: None,
                            },
                            array: Some((v.stride, v.dim)),
                        }
                    }
                    _ => {
                        let o = str_num(cx, r, used)?;
                        ir::StrRes::Positions {
                            target: o,
                            array: None,
                        }
                    }
                }
            }
            Some(_) => return None,
        },
        _ => {
            let r = result.as_ref()?;
            let o = sql_field(cx, r, used)?;
            if !is_char(&o)
                || !matches!(
                    o,
                    ir::Operand::Var {
                        index: None,
                        based: None,
                        ..
                    }
                )
            {
                return None;
            }
            ir::StrRes::Chars(o)
        }
    };
    used.push(format!("string op {code} {:?}", ext.other));
    Some(ir::Lowered::StrOp(Box::new(ir::StrOpData {
        code: c,
        f1: o1,
        f1_chars: o1c,
        f1_num: o1n,
        f2: o2,
        f2_num: o2n,
        pad: ext.other.contains('P'),
        lo: ind.lo.clone(),
        eq: ind.eq.clone(),
        result: res,
    })))
}

/// An array element's type for lowered array operations (S2.86):
/// character (and varying when `varying`) or numeric.
fn global_elem_ty(t: DType, varying: bool) -> Option<ir::Ty> {
    Some(match t {
        DType::Packed(d, s) => ir::Ty::Packed(d, s),
        DType::Zoned(d, s) => ir::Ty::Zoned(d, s),
        DType::Binary(d, s) => ir::Ty::Binary(d, s),
        DType::Int(n) => ir::Ty::Int(n),
        DType::Uns(n) => ir::Ty::Uns(n),
        DType::Char(n) => ir::Ty::Char(n),
        DType::Varchar(n) if varying => ir::Ty::Varchar(n),
        _ => return None,
    })
}

/// A side of MOVEA for character data (S2.86): a global fixed-length
/// character array (not of varying dimension), from its index field's
/// element or the first; a fixed-length character field; for factor 2,
/// a character literal.
fn movea_side(cx: &Cx<'_>, e: &Expr, source: bool, used: &mut Vec<String>) -> Option<ir::MoveaOp> {
    match e {
        Expr::Str(b) if source => Some(ir::MoveaOp::Bytes(b.clone())),
        Expr::Name { parts, args } => {
            let name = parts.join(".");
            match sym(cx, &name) {
                Some(Sym::Var(v)) if v.dim > 0 => {
                    if v.local || cx.prog.varying.contains_key(&name) {
                        return None;
                    }
                    let DType::Char(n) = v.dtype else {
                        return None;
                    };
                    let index = match args.as_deref() {
                        None => None,
                        Some([i]) => Some(
                            str_num(cx, i, used)
                                .filter(|o| matches!(o, ir::Operand::Var { .. }))?,
                        ),
                        Some(_) => return None,
                    };
                    used.push(format!(
                        "movea array {name} stride {} dim {}",
                        v.stride, v.dim
                    ));
                    Some(ir::MoveaOp::Array {
                        arr: (v.buf, v.off, v.stride, v.dim, ir::Ty::Char(n)),
                        index,
                    })
                }
                _ => {
                    let o = sql_field(cx, e, used)?;
                    match &o {
                        ir::Operand::Var {
                            ty: ir::Ty::Char(_),
                            index: None,
                            based: None,
                            ..
                        } => Some(ir::MoveaOp::Field(o)),
                        _ => None,
                    }
                }
            }
        }
        _ => None,
    }
}
