//! The compiler's checks that CRTBNDRPG reports in its listing before it
//! creates a program (ILE RPG Reference; observed on IBM i 7.5 with
//! tools/golden/listings.txt):
//!
//! - RNF7030 (30): a name used in the calculations is not defined;
//! - RNF7503 (30): an expression has such an operand;
//! - RNF7416 (30): EVAL assigns a numeric value to a character target, or
//!   a character value to a numeric one;
//! - RNF7086 (00): RPG blocks the records of a file (the conditions of
//!   the BLOCK keyword's description);
//! - RNF3546 (20): LIKEREC names the data structure itself (a data
//!   structure named as the record format), with what follows from it:
//!   its subfields are not defined (RNF7030, RNF7503), it cannot be the
//!   result of a file operation (RNF7260), and the record format is then
//!   not used (RNF7066, 00) (PUB400 2026-10-28, tools/golden/verify12.txt
//!   n12_rpgp, n12_rpgx, n12_rpgj: `dcl-ds outr likerec(OUTR: *output)`);
//! - RNF7016 (20): DSPLY of a character field longer than 52 bytes
//!   (PUB400 2026-10-30, tools/golden/verify26.txt s26_dsply60);
//! - RNF7547 (30): a float field as an operand of MOVE or MOVEL (PUB400
//!   2026-11-12);
//! - RNF7571 (30): %OCCUR set to a literal outside the occurrences (PUB400
//!   2026-11-22);
//! - RNF0203 (30): a numeric operand of %CONCAT (PUB400 2026-11-22);
//! - RNF7421 (30): a time plus a date, or a duration plus a date, with
//!   RNF7416 (30) on the EVAL (PUB400 2026-11-16 and 2026-11-22);
//! - RNF0605 (20): %DATE(*SYS) (PUB400 2026-11-16);
//! - RNF7510 (30): %DATE with no format of a value too short for *ISO
//!   (PUB400 2026-11-16);
//! - RNF7030 and RNF7018 (30): EXSR in a subprocedure of a subroutine it
//!   does not have (PUB400 2026-11-15);
//! - RNF7569 (20): a FOR limit with decimal places (PUB400 2026-11-22);
//! - RNF7503 and RNF7030 (30): a numeric indicator used in an expression
//!   and set nowhere (PUB400 2026-11-23);
//! - RNF0530 (20) and RNF7563 (30): a constant index past a DIM(*AUTO)
//!   array's dimension, %ELEM of a DIM(*VAR) array set past its
//!   dimension (PUB400 2026-11-23);
//! - RNF5056 (30): TEST(D), TEST(T) or TEST(Z) with neither E nor an
//!   error indicator (PUB400 2026-11-20 and 2026-11-22);
//! - RNF2121 (30) and RNF2109 (40): an externally described file whose
//!   record format has the file's name, without RENAME (PUB400
//!   2026-11-05, tools/golden/verify47.txt p47_crtr).
//!
//! The texts are as4tainr's own.

use super::ast::{keyword, BinOp, CalcLine, Expr, FileOp, Line, Source, Stmt};
use super::data::DType;
use super::program::{Program, Sym};

/// A message of the compiler about source line `line` (0: none).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Diag {
    pub line: usize,
    pub id: String,
    pub sev: u8,
    pub text: String,
    /// Where on its line the message points (characters from the start
    /// of the source line, and how many): the listing shows it under the
    /// line with a marker letter rather than in the additional
    /// diagnostic messages (RNF3546: PUB400 2026-10-28,
    /// tools/golden/verify13.txt t13_rpge).
    pub at: Option<(usize, usize)>,
}

/// The checks' messages, and the names that are not defined (with the
/// lines that use them), for the cross reference.
#[derive(Debug, Default)]
pub struct Checked {
    pub diags: Vec<Diag>,
    pub undefined: Vec<(String, usize)>,
}

/// Built-in functions whose arguments name files, procedures or fields
/// rather than being values.
const NAME_BIFS: &[&str] = &[
    "%OPEN", "%EOF", "%FOUND", "%EQUAL", "%STATUS", "%ERROR", "%FIELDS", "%PADDR", "%PARMNUM",
    "%TARGET", "%HANDLER", "%XML", "%DATA", "%GEN", "%NULLIND", "%KDS", "%PROC", "%SIZE", "%ELEM",
    "%ADDR", "%LEN", "%DECPOS", "%PASSED", "%OMITTED",
];

/// Special names the calculations may use without defining them.
fn special(name: &str) -> bool {
    name.starts_with('*')
        || matches!(
            name,
            "UDATE" | "UDAY" | "UMONTH" | "UYEAR" | "PAGE" | "PAGE1" | "PAGE2" | "PAGE3"
        )
        || name.starts_with("PAGE")
}

struct Walker<'a> {
    prog: &'a Program,
    proc: Option<&'a super::program::ProcInfo>,
    out: Checked,
    /// Data structures whose LIKEREC names themselves (RNF3546) in the
    /// scope being checked: they have no subfields.
    bad_ds: Vec<String>,
    /// Record formats a file operation with such a data structure named.
    invalid_io: Vec<String>,
    /// Numeric indicators (01-99) used in expressions, with their lines.
    ind_uses: Vec<(usize, String)>,
    /// Numeric indicators passed to a procedure or program, which may set
    /// them.
    ind_passed: Vec<String>,
}

impl Walker<'_> {
    fn sym(&self, name: &str) -> Option<&Sym> {
        self.proc
            .and_then(|p| p.syms.get(name))
            .or_else(|| self.prog.syms.get(name))
    }

    fn defined(&self, name: &str) -> bool {
        special(name)
            || self.sym(name).is_some()
            || self.prog.procs.contains_key(name)
            || self.prog.protos.contains_key(name)
            || self.prog.imports.iter().any(|i| i == name)
            || self
                .prog
                .files
                .iter()
                .any(|f| f.decl.name == name || f.format == name)
    }

    /// Note the names of an expression that are not defined; true when
    /// there is one.
    fn expr(&mut self, line: usize, e: &Expr) -> bool {
        let mut bad = false;
        match e {
            Expr::Name { parts, .. } | Expr::Path { parts, .. }
                if parts.len() > 1 && self.bad_ds.contains(&parts[0].to_ascii_uppercase()) =>
            {
                self.undefined(line, &parts[1].to_ascii_uppercase());
                bad = true;
            }
            Expr::Name { parts, args } => {
                let name = parts.join(".");
                if !self.defined(&name) && !self.defined(&parts[0]) {
                    self.undefined(line, &name);
                    bad = true;
                }
                if self.prog.procs.contains_key(&name) || self.prog.protos.contains_key(&name) {
                    for a in args.iter().flatten() {
                        if let Expr::Ind(n) = a {
                            self.ind_passed.push(n.clone());
                        }
                    }
                }
                for a in args.iter().flatten() {
                    bad |= self.expr(line, a);
                }
            }
            Expr::Path { parts, idx, args } => {
                for i in idx.iter().flatten() {
                    bad |= self.expr(line, i);
                }
                for a in args.iter().flatten() {
                    bad |= self.expr(line, a);
                }
                if !self.defined(&parts[0]) && !self.defined(&parts.join(".")) {
                    self.undefined(line, &parts.join("."));
                    bad = true;
                }
            }
            Expr::IndArray(Some(x)) => bad |= self.expr(line, x),
            Expr::Ind(n) if n.len() == 2 && n.bytes().all(|b| b.is_ascii_digit()) => {
                self.ind_uses.push((line, n.clone()));
            }
            Expr::Bif(b, args) if !NAME_BIFS.contains(&b.to_ascii_uppercase().as_str()) => {
                for a in args {
                    bad |= self.expr(line, a);
                }
            }
            Expr::Unary(_, x) => bad |= self.expr(line, x),
            Expr::Bin(_, a, b) => {
                bad |= self.expr(line, a);
                bad |= self.expr(line, b);
            }
            _ => {}
        }
        bad
    }

    fn undefined(&mut self, line: usize, name: &str) {
        if !self
            .out
            .undefined
            .iter()
            .any(|(n, l)| n == name && *l == line)
        {
            self.out.undefined.push((name.to_string(), line));
            self.out.diags.push(Diag {
                line,
                id: "RNF7030".into(),
                sev: 30,
                text: format!("Name or indicator {name} is not defined."),
                at: None,
            });
        }
    }

    /// A constant index past a DIM(*AUTO) array's dimension as the target
    /// of an assignment: RNF0530, severity 20; %ELEM of a DIM(*VAR) array
    /// set to a constant past its dimension: RNF7563, severity 30
    /// (tools/golden/native220 n220_crt_aut and n220_crt_var, PUB400,
    /// IBM i 7.5, 2026-11-23: `au(6) = 1` for DIM(*AUTO:5), `%elem(va) =
    /// 5` for DIM(*VAR:4)).
    fn varying_past_dim(&mut self, l: usize, target: &Expr, value: &Expr) {
        let dim_of = |w: &Self, name: &str, kind: char| -> Option<i128> {
            let (k, _) = w
                .prog
                .varying
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))?;
            if w.prog.varying.get(k) != Some(&kind) {
                return None;
            }
            match w.sym(&name.to_ascii_uppercase())? {
                Sym::Var(v) => Some(i128::from(v.dim)),
                _ => None,
            }
        };
        match (target, value) {
            (
                Expr::Name {
                    parts,
                    args: Some(a),
                },
                _,
            ) if a.len() == 1 => {
                let name = parts.join(".");
                if let (Some(d), Some(Expr::Num(n))) = (dim_of(self, &name, 'A'), a.first()) {
                    if let Ok(k) = n.to_i128() {
                        if k > d {
                            self.out.diags.push(Diag {
                                line: l,
                                id: "RNF0530".into(),
                                sev: 20,
                                text: format!(
                                    "The index {k} of {name} is past its dimension, {d}."
                                ),
                                at: None,
                            });
                        }
                    }
                }
            }
            (Expr::Bif(b, args), Expr::Num(n)) if b.eq_ignore_ascii_case("ELEM") => {
                if let Some(Expr::Name { parts, args: None }) = args.first() {
                    let name = parts.join(".");
                    if let (Some(d), Ok(k)) = (dim_of(self, &name, 'V'), n.to_i128()) {
                        if k > d {
                            self.out.diags.push(Diag {
                                line: l,
                                id: "RNF7563".into(),
                                sev: 30,
                                text: format!(
                                    "%ELEM of {name} cannot be set to {k}: its dimension is {d}."
                                ),
                                at: None,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// A value: RNF7503 when it uses a name that is not defined.
    fn value(&mut self, line: usize, e: &Expr) {
        if self.expr(line, e) {
            self.out.diags.push(Diag {
                line,
                id: "RNF7503".into(),
                sev: 30,
                text: "The expression has an operand that is not defined.".into(),
                at: None,
            });
        }
        self.concat_operands(line, e);
        if self.time_plus_date(e) {
            self.out.diags.push(Diag {
                line,
                id: "RNF7421".into(),
                sev: 30,
                text: "The operands of + do not fit the operator in this order: a date comes before a time, a duration after what it changes.".into(),
                at: None,
            });
        }
    }

    /// Whether a numeric expression has decimal places by the precision
    /// rules, when that is known: a literal's or field's own, any
    /// quotient, and +, - and * of operands with them.
    fn has_decimals(&self, e: &Expr) -> Option<bool> {
        match e {
            Expr::Num(d) => Some(d.scale() > 0),
            Expr::Name { args: None, .. } => match self.field_type(e)? {
                DType::Packed(_, s) | DType::Zoned(_, s) | DType::Binary(_, s) => Some(s > 0),
                DType::Int(_) | DType::Uns(_) => Some(false),
                _ => None,
            },
            Expr::Unary(_, x) => self.has_decimals(x),
            Expr::Bin(BinOp::Div, ..) => Some(true),
            Expr::Bin(BinOp::Add | BinOp::Sub | BinOp::Mul, l, r) => {
                Some(self.has_decimals(l)? || self.has_decimals(r)?)
            }
            _ => None,
        }
    }

    /// The type of a field named without arguments.
    fn field_type(&self, e: &Expr) -> Option<DType> {
        match e {
            Expr::Name { parts, args: None } => match self.sym(&parts.join("."))? {
                Sym::Var(v) => Some(v.dtype),
                _ => None,
            },
            _ => None,
        }
    }

    /// A time plus a date (the date must come first), or a duration plus a
    /// date, time or timestamp (the duration must come second): RNF7421,
    /// severity 30 (PUB400 IBM i 7.5: `z = t + d` does not compile and
    /// `z = d + t` does, 2026-11-20 and 2026-11-22; `d2 = %years(2) + d1`
    /// does not compile, tools/golden/native56 n56_crt_first, 2026-11-16).
    fn time_plus_date(&self, e: &Expr) -> bool {
        let duration = |x: &Expr| {
            matches!(x, Expr::Bif(b, _) if matches!(b.as_str(),
                "YEARS" | "MONTHS" | "DAYS" | "HOURS" | "MINUTES" | "SECONDS" | "MSECONDS"))
        };
        match e {
            Expr::Bin(BinOp::Add, l, r) => {
                matches!(
                    (self.field_type(l), self.field_type(r)),
                    (Some(DType::Time(_)), Some(DType::Date(_)))
                ) || (duration(l)
                    && matches!(
                        self.field_type(r),
                        Some(DType::Date(_) | DType::Time(_) | DType::Timestamp(_))
                    ))
                    || self.time_plus_date(l)
                    || self.time_plus_date(r)
            }
            Expr::Bin(_, l, r) => self.time_plus_date(l) || self.time_plus_date(r),
            Expr::Unary(_, x) => self.time_plus_date(x),
            Expr::Bif(_, args)
            | Expr::Name {
                args: Some(args), ..
            } => args.iter().any(|a| self.time_plus_date(a)),
            _ => false,
        }
    }

    /// A numeric operand of %CONCAT: RNF0203, severity 30 (PUB400
    /// 2026-11-21 and 2026-11-22, IBM i 7.5: `%concat(',' : 'a' : n)` with
    /// N packed(3:0)); its operands must be character, graphic or UCS-2.
    fn concat_operands(&mut self, line: usize, e: &Expr) {
        match e {
            Expr::Bif(b, args) => {
                // %DATE(*SYS): RNF0605, severity 20 (tools/golden/native55
                // n55_crt_dft, PUB400, IBM i 7.5, 2026-11-16).
                if b == "DATE"
                    && matches!(args.first(), Some(Expr::Name { parts, args: None })
                        if parts.len() == 1 && parts[0] == "*SYS")
                {
                    self.out.diags.push(Diag {
                        line,
                        id: "RNF0605".into(),
                        sev: 20,
                        text: "*SYS cannot be the first parameter of %DATE.".into(),
                        at: None,
                    });
                }
                // %DATE with no format takes *ISO: a literal or numeric
                // field too short for an *ISO date is RNF7510, severity 30
                // (tools/golden/native55 n55_crt_dft, PUB400, IBM i 7.5,
                // 2026-11-16: %DATE('11/17/26') and %DATE(m) of a packed(6:0)
                // under DATFMT(*MDY)).
                if b == "DATE" && args.len() == 1 {
                    let short = match &args[0] {
                        Expr::Str(t) => t.len() < 10,
                        a => match self.field_type(a) {
                            Some(DType::Packed(d, 0) | DType::Zoned(d, 0)) => d < 8,
                            _ => false,
                        },
                    };
                    if short {
                        self.out.diags.push(Diag {
                            line,
                            id: "RNF7510".into(),
                            sev: 30,
                            text: "The operand of %DATE is too short for a whole *ISO date.".into(),
                            at: None,
                        });
                    }
                }
                if b == "CONCAT" {
                    for a in args.iter().skip(1) {
                        if self.class_of(a) == Some(true) {
                            self.out.diags.push(Diag {
                                line,
                                id: "RNF0203".into(),
                                sev: 30,
                                text: "An operand of %CONCAT must be character, graphic or UCS-2."
                                    .into(),
                                at: None,
                            });
                        }
                    }
                }
                for a in args {
                    self.concat_operands(line, a);
                }
            }
            Expr::Name {
                args: Some(args), ..
            } => {
                for a in args {
                    self.concat_operands(line, a);
                }
            }
            Expr::Unary(_, x) => self.concat_operands(line, x),
            Expr::Bin(_, l, r) => {
                self.concat_operands(line, l);
                self.concat_operands(line, r);
            }
            _ => {}
        }
    }

    fn opt(&mut self, line: usize, e: &Option<Expr>) {
        if let Some(e) = e {
            self.value(line, e);
        }
    }

    fn calc(&mut self, c: &CalcLine) {
        let l = c.line;
        match &c.what {
            Line::Stmt(s) => self.stmt(l, s),
            Line::If(e) | Line::ElseIf(e) | Line::Dow(e) | Line::Dou(e) | Line::When(e) => {
                self.value(l, e)
            }
            Line::For {
                var, from, to, by, ..
            } => {
                self.expr(l, var);
                self.value(l, from);
                self.opt(l, to);
                self.opt(l, by);
                // A limit with decimal places: RNF7569, severity 20 (PUB400
                // 2026-11-20 and 2026-11-22, IBM i 7.5: `for i = 0 to
                // 10 / 4` does not compile).
                if to
                    .as_ref()
                    .is_some_and(|t| self.has_decimals(t) == Some(true))
                {
                    self.out.diags.push(Diag {
                        line: l,
                        id: "RNF7569".into(),
                        sev: 20,
                        text: "The limit of the FOR operation must be a whole number.".into(),
                        at: None,
                    });
                }
            }
            Line::ForEach { var, list } => {
                self.expr(l, var);
                self.value(l, list);
            }
            Line::Do {
                start,
                limit,
                index,
            } => {
                self.opt(l, start);
                self.opt(l, limit);
                if let Some(i) = index {
                    self.expr(l, i);
                }
            }
            Line::Return(e) => self.opt(l, e),
            Line::Cab { f1, f2, .. } => {
                self.value(l, f1);
                self.value(l, f2);
            }
            _ => {}
        }
    }

    fn stmt(&mut self, l: usize, s: &Stmt) {
        match s {
            Stmt::Eval { target, value, .. } => {
                // %OCCUR(ds) set to a literal outside the data structure's
                // occurrences: RNF7571, severity 30 (PUB400 2026-11-17 and
                // 2026-11-22, IBM i 7.5: `%occur(rec) = 4` for OCCURS(3)).
                if let (Expr::Bif(b, args), Expr::Num(n)) = (target, value) {
                    if let (true, Some(Expr::Name { parts, .. })) = (b == "OCCUR", args.first()) {
                        let name = parts.join(".");
                        let count = self
                            .prog
                            .occurs
                            .iter()
                            .find(|o| o.name == name)
                            .map(|o| o.count);
                        if let (Some(c), Ok(v)) = (count, n.to_i128()) {
                            if v < 1 || v > i128::from(c) {
                                self.out.diags.push(Diag {
                                    line: l,
                                    id: "RNF7571".into(),
                                    sev: 30,
                                    text: format!(
                                        "%OCCUR of {name} cannot be {v}: it has {c} occurrences."
                                    ),
                                    at: None,
                                });
                            }
                        }
                    }
                }
                self.varying_past_dim(l, target, value);
                // A subfield of a data structure without subfields
                // (RNF3546) as the target: RNF7503 too (verify12
                // n12_rpgp: `outr.ID = id;` RNF7030 and RNF7503).
                if self.expr(l, target) && self.bad_target(target) {
                    self.out.diags.push(Diag {
                        line: l,
                        id: "RNF7503".into(),
                        sev: 30,
                        text: "The expression has an operand that is not defined.".into(),
                        at: None,
                    });
                }
                self.value(l, value);
                // The EVAL's value is then not of the target's type either:
                // RNF7416 too, as IBM i lists it.
                if matches!(value, Expr::Bin(BinOp::Add, ..)) && self.time_plus_date(value) {
                    self.out.diags.push(Diag {
                        line: l,
                        id: "RNF7416".into(),
                        sev: 30,
                        text: "The types of the target and the value of the EVAL operation do not match.".into(),
                        at: None,
                    });
                }
                if let (Some(t), Some(v)) = (self.class_of(target), self.class_of(value)) {
                    if t != v {
                        self.out.diags.push(Diag {
                            line: l,
                            id: "RNF7416".into(),
                            sev: 30,
                            text: "The types of the target and the value of the EVAL operation do not match.".into(),
                            at: None,
                        });
                    }
                }
            }
            Stmt::CallP(e) => {
                self.value(l, e);
            }
            Stmt::Call { parms, .. } => {
                for p in parms {
                    self.expr(l, &p.target);
                    self.opt(l, &p.value);
                }
            }
            Stmt::File { key, ds, file, .. } => {
                for k in key.iter().flatten() {
                    self.value(l, k);
                }
                // The result data structure is one without subfields
                // (RNF3546): not valid (verify12 n12_rpgp: `write OUTR
                // outr;` RNF7260).
                if ds
                    .as_ref()
                    .is_some_and(|d| self.bad_ds.contains(&d.to_ascii_uppercase()))
                {
                    self.out.diags.push(Diag {
                        line: l,
                        id: "RNF7260".into(),
                        sev: 30,
                        text: "The data structure in the result field cannot be used with this operation.".into(),
                        at: None,
                    });
                    self.invalid_io.push(file.to_ascii_uppercase());
                }
            }
            Stmt::Dsply { message, response } => {
                self.opt(l, message);
                // At most 52 bytes are displayed (ILE RPG Reference,
                // DSPLY): a longer character field is RNF7016, severity
                // 20 (PUB400 2026-10-30, tools/golden/verify26.txt
                // s26_dsply60, s26_dsply53; 52 compiles, s26_dsply52).
                if let Some(Expr::Name { parts, args: None }) = message {
                    if let Some(Sym::Var(v)) = self.sym(&parts.join(".")) {
                        if let DType::Char(n) = v.dtype {
                            if n > 52 {
                                self.out.diags.push(Diag {
                                    line: l,
                                    id: "RNF7016".into(),
                                    sev: 20,
                                    text: format!(
                                        "DSPLY shows at most 52 bytes; this message is {n}."
                                    ),
                                    at: None,
                                });
                            }
                        }
                    }
                }
                if let Some(r) = response {
                    self.expr(l, r);
                }
            }
            Stmt::Comp { f1, f2, .. } => {
                self.value(l, f1);
                self.value(l, f2);
            }
            // A float field as factor 2 or result of MOVE or MOVEL is
            // RNF7547, severity 30 (PUB400 2026-11-12, IBM i 7.5: a fixed
            // form MOVE of an 8F field into a character field).
            Stmt::Move { from, to, .. } if self.is_float(from) || self.is_float(to) => {
                self.out.diags.push(Diag {
                    line: l,
                    id: "RNF7547".into(),
                    sev: 30,
                    text: "MOVE and MOVEL do not take a float field.".into(),
                    at: None,
                });
            }
            // TEST(D), TEST(T) or TEST(Z) needs the E extender or an error
            // indicator: RNF5056, severity 30 (PUB400 2026-11-20 and
            // 2026-11-22, IBM i 7.5: `test(d) *iso c;` does not compile).
            Stmt::Op { code, ext, ind, .. }
                if code == "TEST"
                    && !ext.error
                    && ind.lo.is_none()
                    && ext.other.chars().any(|c| matches!(c, 'D' | 'T' | 'Z')) =>
            {
                self.out.diags.push(Diag {
                    line: l,
                    id: "RNF5056".into(),
                    sev: 30,
                    text: "TEST needs the E extender or an error indicator.".into(),
                    at: None,
                });
            }
            _ => {}
        }
    }

    /// Whether a value is a float field (or an element of a float array).
    fn is_float(&self, e: &Expr) -> bool {
        match e {
            Expr::Name { parts, .. } => matches!(
                self.sym(&parts.join(".")),
                Some(Sym::Var(v)) if matches!(v.dtype, DType::Float(_))
            ),
            _ => false,
        }
    }

    /// Whether an EVAL target is a subfield of a data structure without
    /// subfields (RNF3546).
    fn bad_target(&self, e: &Expr) -> bool {
        matches!(e, Expr::Name { parts, .. } | Expr::Path { parts, .. }
            if parts.len() > 1 && self.bad_ds.contains(&parts[0].to_ascii_uppercase()))
    }

    /// Whether a value is numeric (Some(true)) or character (Some(false)),
    /// when that is certain.
    fn class_of(&self, e: &Expr) -> Option<bool> {
        match e {
            Expr::Num(_) => Some(true),
            Expr::Str(_) => Some(false),
            Expr::Name { parts, args } => {
                let name = parts.join(".");
                match self.sym(&name)? {
                    Sym::Var(v) if args.is_none() || v.dim > 0 => match v.dtype {
                        DType::Char(_) | DType::Varchar(_) => Some(false),
                        DType::Zoned(..)
                        | DType::Packed(..)
                        | DType::Binary(..)
                        | DType::Int(_)
                        | DType::Uns(_)
                        | DType::Float(_) => Some(true),
                        _ => None,
                    },
                    _ => None,
                }
            }
            Expr::Bin(op, a, b) => {
                let (x, y) = (self.class_of(a)?, self.class_of(b)?);
                match op {
                    BinOp::Add if x == y => Some(x),
                    BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow if x && y => Some(true),
                    _ => None,
                }
            }
            Expr::Bif(b, _) => match b.to_ascii_uppercase().as_str() {
                "%CHAR" | "%TRIM" | "%TRIML" | "%TRIMR" | "%SUBST" | "%EDITC" | "%EDITW"
                | "%XLATE" | "%REPLACE" | "%UPPER" | "%LOWER" | "%STR" => Some(false),
                "%DEC" | "%DECH" | "%INT" | "%INTH" | "%UNS" | "%UNSH" | "%SCAN" | "%CHECK"
                | "%CHECKR" | "%LEN" | "%ELEM" | "%SIZE" | "%ABS" | "%REM" | "%DIV" => Some(true),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Run the checks on a program's calculations.
/// `text`: the source as written (where a message points on its line).
pub fn check(src: &Source, prog: &Program, text: &str) -> Checked {
    let mut w = Walker {
        prog,
        proc: None,
        out: Checked::default(),
        bad_ds: Vec::new(),
        invalid_io: Vec::new(),
        ind_uses: Vec::new(),
        ind_passed: Vec::new(),
    };
    let global_bad = self_likerec(&src.defs, text, &mut w.out.diags);
    w.bad_ds = global_bad.clone();
    for c in &src.calcs {
        w.calc(c);
    }
    for p in &src.procs {
        w.proc = prog.procs.get(&p.name.to_ascii_uppercase());
        let local = self_likerec(&p.defs, text, &mut w.out.diags);
        // A local definition of the name hides the global one.
        w.bad_ds = global_bad
            .iter()
            .filter(|g| !p.defs.iter().any(|d| d.name.eq_ignore_ascii_case(g)))
            .cloned()
            .chain(local)
            .collect();
        for c in &p.calcs {
            w.calc(c);
        }
        // EXSR in a subprocedure runs one of its own subroutines: one of
        // the main procedure's is not defined there, RNF7030 and RNF7018,
        // severity 30 (tools/golden/native45 n45_crt_sr, PUB400, IBM i
        // 7.5, 2026-11-15).
        let own: Vec<&String> = p
            .calcs
            .iter()
            .filter_map(|c| match &c.what {
                Line::BegSr(n) => Some(n),
                _ => None,
            })
            .collect();
        for c in &p.calcs {
            if let Line::ExSr(n) = &c.what {
                if n.starts_with('*') || own.iter().any(|o| o.eq_ignore_ascii_case(n)) {
                    continue;
                }
                let name = n.to_ascii_uppercase();
                if !w.defined(&name) {
                    w.out.diags.push(Diag {
                        line: c.line,
                        id: "RNF7030".into(),
                        sev: 30,
                        text: format!("{name} is not defined as a name or indicator here."),
                        at: None,
                    });
                }
                w.out.diags.push(Diag {
                    line: c.line,
                    id: "RNF7018".into(),
                    sev: 30,
                    text: format!(
                        "{name}, the operand of EXSR, is not a subroutine of this procedure."
                    ),
                    at: None,
                });
            }
        }
    }
    // A numeric indicator used in an expression and set nowhere in the
    // program: RNF7503 and RNF7030, severity 30 (tools/golden/native220
    // n220_crt_ind, PUB400, IBM i 7.5, 2026-11-23: `if *in60;`).
    if let Some(set) = indicators_set(src) {
        let mut seen: Vec<(usize, String)> = Vec::new();
        for (line, n) in std::mem::take(&mut w.ind_uses) {
            if set.contains(&n) || w.ind_passed.contains(&n) || seen.contains(&(line, n.clone())) {
                continue;
            }
            seen.push((line, n.clone()));
            w.out.diags.push(Diag {
                line,
                id: "RNF7503".into(),
                sev: 30,
                text: "The expression has an operand that is not defined.".into(),
                at: None,
            });
            w.undefined(line, &format!("*IN{n}"));
        }
    }
    // A record format whose only file operations were not valid: not
    // used (RNF7066, verify12 n12_rpgp).
    let mut used: Vec<String> = Vec::new();
    let mut note = |calcs: &[CalcLine]| {
        for c in calcs {
            if let Line::Stmt(Stmt::File { file, ds, .. }) = &c.what {
                let bad = ds.as_ref().is_some_and(|d| {
                    d.eq_ignore_ascii_case(file)
                        && w.invalid_io.contains(&file.to_ascii_uppercase())
                });
                if !bad {
                    used.push(file.to_ascii_uppercase());
                }
            }
        }
    };
    note(&src.calcs);
    for p in &src.procs {
        note(&p.calcs);
    }
    for f in &prog.files {
        let fmt = f.format.to_ascii_uppercase();
        if w.invalid_io.contains(&fmt)
            && !used.contains(&fmt)
            && !used.contains(&f.decl.name.to_ascii_uppercase())
        {
            w.out.diags.push(Diag {
                line: f.decl.line,
                id: "RNF7066".into(),
                sev: 0,
                text: format!("Record format {fmt} is not read or written."),
                at: None,
            });
        }
    }
    // An externally described file whose record format has the file's
    // name (an SQL table) and no RENAME: RNF2121 (30) then RNF2109 (40),
    // both pointing at the file's name on its declaration (PUB400
    // 2026-11-05, tools/golden/verify47.txt p47_crtr: `dcl-f GLD47F` over
    // an SQL table; the same file with rename(GLD47O:GLD47OR) compiled).
    for f in &prog.files {
        if f.program_described || f.fref.is_none() || f.decl.device != "DISK" {
            continue;
        }
        let name = f.decl.name.to_ascii_uppercase();
        if f.format.to_ascii_uppercase() != name {
            continue;
        }
        let at = text.lines().nth(f.decl.line.wrapping_sub(1)).and_then(|l| {
            let up = l.to_ascii_uppercase();
            let k = up.find(&name)?;
            Some((up[..k].chars().count(), name.chars().count()))
        });
        w.out.diags.push(Diag {
            line: f.decl.line,
            id: "RNF2121".into(),
            sev: 30,
            text: format!(
                "File {name} has a record format of the same name, {name}: the format needs RENAME."
            ),
            at,
        });
        w.out.diags.push(Diag {
            line: f.decl.line,
            id: "RNF2109".into(),
            sev: 40,
            text: format!("File {name} cannot be used: its record formats are not all usable."),
            at,
        });
    }
    let mut out = w.out;
    out.diags.extend(blocking(src));
    out.diags.extend(key_lengths(src, prog, text));
    out.diags.sort_by_key(|d| d.line);
    out
}

/// RNF3546 for each data structure among `defs` whose LIKEREC names the
/// data structure itself; their names.
fn self_likerec(defs: &[super::ast::Def], text: &str, diags: &mut Vec<Diag>) -> Vec<String> {
    let mut out = Vec::new();
    for d in defs {
        if d.kind != super::ast::DefKind::Ds {
            continue;
        }
        let Some(k) = keyword(&d.keywords, "LIKEREC") else {
            continue;
        };
        let fmt = k
            .args
            .first()
            .map(|a| a.trim().to_ascii_uppercase())
            .unwrap_or_default();
        if fmt == d.name.to_ascii_uppercase() {
            // The marker is under LIKEREC's argument (t13_rpge: "aaaa"
            // under OUTR).
            let at = text.lines().nth(d.line.wrapping_sub(1)).and_then(|l| {
                let up = l.to_ascii_uppercase();
                let k = up.find("LIKEREC")?;
                let open = k + up[k..].find('(')? + 1;
                let start = open + up[open..].len() - up[open..].trim_start().len();
                Some((up[..start].chars().count(), fmt.chars().count()))
            });
            diags.push(Diag {
                line: d.line,
                id: "RNF3546".into(),
                sev: 20,
                text: format!("LIKEREC({fmt}) does not name a record format here."),
                at,
            });
            out.push(fmt);
        }
    }
    out
}

/// RNF7078, severity 30: the search argument of CHAIN or SETLL, one
/// value (not a key list or a KLIST), whose length is not that of the
/// file's first key field: a numeric literal, named constant or field with
/// another number of digits, a character one with another number of
/// characters (tools/golden/native221, PUB400, IBM i 7.5, 2026-11-23: with a
/// zoned 3,0 key `setll 001` compiles, `setll 1`, `setll 0001`, `chain 1`,
/// a 1-digit constant, a packed(5:0) field and fixed-form factor 1 `1` do
/// not; with a 5-character key `'ABCDE'` compiles, `'AB'` and `'ABCDEF'` do
/// not). A literal's length is as written, so it is read from the source
/// line. Only those cases are checked: a key list, an expression, other
/// operations, other types and different decimal places are left alone.
fn key_lengths(src: &Source, prog: &Program, text: &str) -> Vec<Diag> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut walk = |calcs: &[CalcLine], defs: &[super::ast::Def], proc: Option<&String>| {
        let syms = proc.and_then(|p| prog.procs.get(&p.to_ascii_uppercase()));
        for c in calcs {
            let Line::Stmt(Stmt::File {
                op,
                file,
                key: Some(key),
                ..
            }) = &c.what
            else {
                continue;
            };
            let opname = match op {
                FileOp::Chain => "CHAIN",
                FileOp::SetLL => "SETLL",
                _ => continue,
            };
            let [arg] = key.as_slice() else { continue };
            let Some(want) = first_key(prog, file) else {
                continue;
            };
            let Some(line) = c.line.checked_sub(1).and_then(|i| lines.get(i)) else {
                continue;
            };
            let Some(written) = search_argument(line, opname) else {
                continue;
            };
            let have = match arg {
                Expr::Num(_) | Expr::Str(_) => written_length(&written),
                Expr::Name { parts, args: None } if parts.len() == 1 => {
                    let name = parts[0].to_ascii_uppercase();
                    let sym = syms
                        .and_then(|p| p.syms.get(&name))
                        .or_else(|| prog.syms.get(&name));
                    match sym {
                        Some(Sym::Var(v)) if v.dim == 0 => type_length(&v.dtype),
                        Some(Sym::Const(_)) => defs
                            .iter()
                            .chain(src.defs.iter())
                            .find(|d| d.name.eq_ignore_ascii_case(&name))
                            .and_then(const_text)
                            .and_then(|t| written_length(&t)),
                        _ => None,
                    }
                }
                _ => None,
            };
            if let Some(have) = have {
                if have.0 == want.0 && have.2 == want.2 && have.1 != want.1 {
                    out.push(Diag {
                        line: c.line,
                        id: "RNF7078".into(),
                        sev: 30,
                        text: "The search argument is not as long as the file's first key field."
                            .into(),
                        at: None,
                    });
                }
            }
        }
    };
    walk(&src.calcs, &[], None);
    for p in &src.procs {
        walk(&p.calcs, &p.defs, Some(&p.name));
    }
    out
}

/// (numeric, length, decimal places) of a type the check compares: digits
/// of a zoned or packed number, characters of a fixed-length field.
fn type_length(t: &DType) -> Option<(bool, u32, u32)> {
    match t {
        DType::Zoned(d, s) | DType::Packed(d, s) => Some((true, *d, *s)),
        DType::Char(n) => Some((false, *n, 0)),
        _ => None,
    }
}

/// The first key field of an externally described DISK file (by file or
/// record format name), as `type_length` gives it.
fn first_key(prog: &Program, file: &str) -> Option<(bool, u32, u32)> {
    let f = prog
        .files
        .iter()
        .find(|f| f.decl.name.eq_ignore_ascii_case(file) || f.format.eq_ignore_ascii_case(file))?;
    let fr = f.fref.as_ref()?;
    if f.program_described {
        return None;
    }
    let k = fr.file.keys.first()?;
    let i = fr.file.fields.iter().position(|fd| &fd.name == k)?;
    let name = f.fields.get(i)?;
    match prog.syms.get(&name.to_ascii_uppercase())? {
        Sym::Var(v) => type_length(&v.dtype),
        _ => None,
    }
}

/// The search argument as written on the operation's source line: free
/// form `CHAIN(E) arg file`, or fixed form factor 1. None for a key list in
/// parentheses, or when the line does not show it.
fn search_argument(line: &str, op: &str) -> Option<String> {
    let chars: Vec<char> = line.chars().collect();
    let fixed = chars.get(5).is_some_and(|c| c.eq_ignore_ascii_case(&'C'))
        && chars.len() > 26
        && !line.trim_start().starts_with("//");
    if fixed {
        // Fixed form when the operation is in positions 26-35; otherwise
        // a free-form line that happens to have a C in position 6.
        let opcode: String = chars.get(25..35.min(chars.len()))?.iter().collect();
        let opcode = opcode.trim().to_ascii_uppercase();
        if opcode == op || opcode.starts_with(&format!("{op}(")) {
            let f1: String = chars.get(11..25)?.iter().collect();
            let f1 = f1.trim();
            return (!f1.is_empty()).then(|| f1.to_string());
        }
    }
    let upper = line.to_ascii_uppercase();
    let mut at = 0;
    while let Some(i) = upper.get(at..)?.find(op) {
        let start = at + i;
        let end = start + op.len();
        at = end;
        let before = upper.get(..start)?.chars().last();
        if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '%') {
            continue;
        }
        let mut rest = line.get(end..)?;
        if rest.starts_with('(') {
            rest = rest.get(rest.find(')')? + 1..)?;
        }
        if !rest.starts_with([' ', '\t']) {
            continue;
        }
        let rest = rest.trim_start();
        if rest.starts_with('(') {
            return None;
        }
        let tok: String = if rest.starts_with('\'') {
            // A character literal, with its doubled quotes.
            let b: Vec<char> = rest.chars().collect();
            let mut j = 1;
            loop {
                match b.get(j) {
                    Some('\'') if b.get(j + 1) == Some(&'\'') => j += 2,
                    Some('\'') => break,
                    Some(_) => j += 1,
                    None => return None,
                }
            }
            b.get(..=j)?.iter().collect()
        } else {
            rest.chars()
                .take_while(|c| !c.is_whitespace() && *c != ';')
                .collect()
        };
        return (!tok.is_empty()).then_some(tok);
    }
    None
}

/// A literal's (numeric, length, decimal places) as written: the digits of
/// a whole number (sign aside), or the characters of a quoted literal. None
/// for anything else (decimal places, names, hexadecimal).
fn written_length(t: &str) -> Option<(bool, u32, u32)> {
    let t = t.trim();
    if let Some(body) = t.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        let n = body.replace("''", "'").chars().count();
        return u32::try_from(n).ok().map(|n| (false, n, 0));
    }
    let digits = t.strip_prefix(['+', '-']).unwrap_or(t);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    u32::try_from(digits.len()).ok().map(|n| (true, n, 0))
}

/// A named constant's value as written (DCL-C or a C definition).
fn const_text(d: &super::ast::Def) -> Option<String> {
    keyword(&d.keywords, "CONST")
        .and_then(|k| k.args.first().cloned())
        .or_else(|| {
            let joined = d
                .keywords
                .iter()
                .map(|k| k.name.clone())
                .collect::<Vec<_>>()
                .join(" ");
            (!joined.is_empty()).then_some(joined)
        })
}

/// RNF7086 for each file whose records RPG blocks (ILE RPG Reference,
/// BLOCK keyword): a DISK or SEQ file that is output-only, or input-only
/// and read only with READ, without RECNO, COMMIT, SFILE or BLOCK(*NO).
fn blocking(src: &Source) -> Vec<Diag> {
    let mut ops: Vec<(String, FileOp)> = Vec::new();
    let mut note = |calcs: &[CalcLine]| {
        for c in calcs {
            if let Line::Stmt(Stmt::File { op, file, .. }) = &c.what {
                ops.push((file.to_ascii_uppercase(), *op));
            }
        }
    };
    note(&src.calcs);
    for p in &src.procs {
        note(&p.calcs);
    }
    let mut out = Vec::new();
    for f in &src.files {
        if !matches!(f.device.as_str(), "DISK" | "SEQ") {
            continue;
        }
        let kw = |k: &str| keyword(&f.keywords, k).is_some();
        if kw("RECNO") || kw("COMMIT") || kw("SFILE") {
            continue;
        }
        if keyword(&f.keywords, "BLOCK")
            .and_then(|k| k.args.first())
            .is_some_and(|a| a.trim().eq_ignore_ascii_case("*NO"))
        {
            continue;
        }
        let name = f.name.to_ascii_uppercase();
        let formats: Vec<String> = Vec::new();
        let mine = |n: &str| n == name || formats.iter().any(|x| x == n);
        let blocked = match f.usage {
            'O' => true,
            'I' if !f.add => !ops.iter().any(|(n, op)| {
                mine(n)
                    && !matches!(
                        op,
                        FileOp::Read | FileOp::Open | FileOp::Close | FileOp::Feod
                    )
            }),
            _ => false,
        };
        if blocked {
            out.push(Diag {
                line: f.line,
                id: "RNF7086".into(),
                sev: 0,
                text: format!(
                    "RPG blocks the records of file {name}; its INFDS changes only when a block is read or written."
                ),
                at: None,
            });
        }
    }
    out
}

/// The numeric indicators (01-99) the program sets: resulting indicators,
/// SETON and SETOFF, and those that are the target of an assignment, a
/// result field, a parameter or CLEAR/RESET. None when the program can set
/// indicators in ways not seen here (a display or printer file, input
/// specifications, INDDS or INDARA, the *IN array), so that no indicator
/// is taken as set nowhere.
fn indicators_set(src: &Source) -> Option<Vec<String>> {
    if !src.inputs.is_empty()
        || src.files.iter().any(|f| {
            f.device != "DISK"
                || keyword(&f.keywords, "INDDS").is_some()
                || keyword(&f.keywords, "INDARA").is_some()
        })
    {
        return None;
    }
    let text = format!(
        "{:?}",
        (&src.calcs, &src.procs, &src.plists, &src.entry, &src.defs)
    );
    if text.contains("IndArray(") {
        return None;
    }
    let mut set = Vec::new();
    let mut quoted_after = |marker: &str, list: bool| {
        let mut rest = text.as_str();
        while let Some(i) = rest.find(marker) {
            rest = &rest[i + marker.len()..];
            let end = if list { rest.find(']').unwrap_or(0) } else { 4 };
            let part = rest.get(..end.min(rest.len())).unwrap_or("");
            for q in part.split('"').skip(1).step_by(2) {
                if q.len() == 2 && q.bytes().all(|b| b.is_ascii_digit()) {
                    set.push(q.to_string());
                }
                if !list {
                    break;
                }
            }
        }
    };
    for m in [
        "hi: Some(",
        "lo: Some(",
        "eq: Some(",
        "result: Some(Ind(",
        "target: Ind(",
        "Clear(Ind(",
        "Reset(Ind(",
        "response: Some(Ind(",
    ] {
        quoted_after(m, false);
    }
    quoted_after("inds: [", true);
    Some(set)
}
