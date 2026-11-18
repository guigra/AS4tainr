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
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+stage2.86");

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
        /// Why the routine cannot be generated at all because of this
        /// instruction (S2.41: an expression past IBM i's limits); the
        /// generator then refuses the routine, naming the line.
        pub refused: Option<String>,
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
        /// Fixed-length character (S2.4).
        Char(u32),
        /// A varying-length character field of this many characters at
        /// most (S2.19); its bytes start with the current length.
        Varchar(u32),
        /// A pointer (S2.47): 16 bytes, rpgrt::rpg::mem::Ptr's encoding.
        Pointer,
        /// A date, time or timestamp field (S2.52): its format's name and
        /// separator (rpgrt::rpg::dates::DateFmt, TimeFmt), or a
        /// timestamp's fractional digits. Its value is its *ISO text.
        Date(&'static str, Option<char>),
        Time(&'static str, Option<char>),
        Timestamp(u8),
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
            /// A procedure local (S2.8): `buf` is the frame's, and the
            /// runtime gives where it is in this call (`Runtime::local`).
            local: bool,
            /// An array element with an index known only when it runs
            /// (S2.21): `off` is the array's start.
            index: Option<Box<Index>>,
            /// A based field (S2.48): its storage is where its pointer
            /// points now, `off` bytes on (`Runtime::based_at`).
            based: Option<Box<Based>>,
        },
        /// A numeric literal: mantissa and scale (`Dec::new`).
        Num { m: i128, s: u8 },
        /// A character literal, its EBCDIC bytes (S2.4).
        Str(Vec<u8>),
        /// A figurative constant (S2.22), in comparisons only.
        Fig(FigOp),
        /// The current occurrence of the multiple-occurrence data
        /// structure in buffer `buf` (S2.64): %OCCUR's value, and the
        /// "index" of its subfields.
        Occur { buf: usize, name: String },
    }

    /// Which of %DATE, %TIME and %TIMESTAMP (S2.54).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TemporalBifKind {
        Date,
        Time,
        Ts,
    }

    /// How %CHAR shows a date, time or timestamp (S2.53): a date or time
    /// format's name and separator, or *ISO0 (digits only) for a
    /// timestamp.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum TemporalFmt {
        Date(&'static str, Option<char>),
        Time(&'static str, Option<char>),
        Ts { iso0: bool },
    }

    /// The figurative constants lowered code takes (S2.22).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum FigOp {
        Blanks,
        Zeros,
        HiVal,
        LoVal,
        /// *ALL'x...': the pattern's EBCDIC bytes.
        All(Vec<u8>),
        /// *NULL (S2.47).
        Null,
    }

    /// Where a based field is (S2.48): the pointer it is based on (a
    /// field that is not based itself), and the field's size, which must
    /// fit in the storage the pointer addresses.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Based {
        pub ptr: Operand,
        pub size: usize,
    }

    /// The index of an array element (S2.21): the operand it is, and the
    /// array's element distance and number of elements.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Index {
        pub of: Operand,
        pub stride: usize,
        pub dim: u32,
        /// A varying-dimension array (S2.45): its name, and whether it is
        /// DIM(*AUTO) (grows when written past its elements in use).
        pub vary: Option<(String, bool)>,
        /// The index of a data structure further out on the path (S2.77:
        /// `gv(s).m(1).parmNo`): its element first, then this one's.
        pub outer: Option<Box<Index>>,
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
        /// A comparison with an expression on either side (S2.13):
        /// %SCAN, %SUBST, %TRIM, %XLATE, concatenation.
        CmpExpr(CmpOp, Box<CmpSide>, Box<CmpSide>),
        And(Box<Cond>, Box<Cond>),
        Or(Box<Cond>, Box<Cond>),
        /// An indicator (`*IN50`, `*INLR`; S2.39).
        Ind(String),
        /// A chain of ANDs (`true`) or ORs down the left operands (S2.41):
        /// the conditions in order, each evaluated only while needed.
        Chain(bool, Vec<Cond>),
        /// NOT of a condition (S2.39).
        Not(Box<Cond>),
        /// %EOF, %FOUND or %EQUAL (S2.58): of the file named, or of the
        /// last file operation.
        FileStatus(FileBit, Option<String>),
    }

    /// Which part of a file's status a built-in function gives (S2.58).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FileBit {
        Eof,
        Found,
        Equal,
    }

    /// An input host variable (S2.70): its value, the length of a fixed
    /// character field (a `CAST(... AS char(n))`, as host_literal makes
    /// it), and its indicator variable.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SqlIn {
        pub value: Operand,
        pub char_len: Option<u32>,
        pub ind: Option<Operand>,
    }

    /// An argument of an SQL CALL (S2.75): text as written, or a host
    /// variable (after `NAME => ` when given by name) as a literal, with
    /// the field an OUT or INOUT parameter's value goes back to.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum SqlArg {
        Text(String),
        /// Boxed: the variants' sizes stay close (clippy::large_enum_variant).
        Host {
            kw: String,
            input: Box<SqlIn>,
            out: Option<Box<SqlOut>>,
        },
    }

    /// A side of MOVEA (S2.86): an array (buf, off, stride, dim, type)
    /// with its index field, a literal's bytes, or a field.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum MoveaOp {
        Array {
            arr: (usize, usize, usize, u32, Ty),
            index: Option<Operand>,
        },
        Bytes(Vec<u8>),
        Field(Operand),
    }

    /// A fixed-form array operation's parts (S2.85): LOOKUP's search
    /// argument and index field, or XFOOT's result field and half adjust;
    /// the array (its first element as an operand, the distance, the
    /// elements); the indicators.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ArrOpData {
        pub lookup: Option<(Operand, Option<Operand>)>,
        pub xfoot: Option<(Operand, bool)>,
        pub arr: Operand,
        pub stride: usize,
        pub dim: u32,
        pub hi: Option<String>,
        pub lo: Option<String>,
        pub eq: Option<String>,
    }

    /// A fixed-form bit or test operation's parts (S2.84).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct BitOpData {
        pub code: BitCode,
        pub target: Operand,
        pub mask: Option<u8>,
        pub mask_field: Option<Operand>,
        pub hi: Option<String>,
        pub lo: Option<String>,
        pub eq: Option<String>,
    }

    /// A fixed-form bit or test operation (S2.84).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum BitCode {
        TestN,
        TestZ,
        TestB,
        BitOn,
        BitOff,
    }

    /// A fixed-form string operation's parts (S2.83).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct StrOpData {
        pub code: StrCode,
        pub f1: Option<Operand>,
        pub f1_chars: Option<Operand>,
        pub f1_num: Option<Operand>,
        pub f2: Operand,
        pub f2_num: Option<Operand>,
        pub pad: bool,
        pub lo: Option<String>,
        pub eq: Option<String>,
        pub result: StrRes,
    }

    /// A fixed-form string operation (S2.83).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum StrCode {
        Scan,
        Check,
        CheckR,
        Subst,
        Xlate,
        Cat,
    }

    /// A string operation's result (S2.83): none, a numeric field, the
    /// first element of a numeric array with its distance and elements,
    /// or a character field.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum StrRes {
        None,
        Positions {
            target: Operand,
            array: Option<(usize, u32)>,
        },
        Chars(Operand),
    }

    /// A row count of a multiple-row FETCH or INSERT (S2.74): a number
    /// as written, or a host variable.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum SqlCount {
        Lit(String),
        Var(Operand),
    }

    /// A subfield of a host structure array (S2.74): its first element's
    /// place, the distance between elements, its type, whether it is
    /// numeric.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SqlRowField {
        pub buf: usize,
        pub off: usize,
        pub stride: usize,
        pub ty: Ty,
        pub numeric: bool,
    }

    /// The dynamic SQL statements (S2.71).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum SqlDyn {
        Prepare,
        Immediate,
        Execute,
    }

    /// An INTO target (S2.70): the field, whether it is numeric, and its
    /// indicator variable.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct SqlOut {
        pub target: Operand,
        pub numeric: bool,
        pub ind: Option<Operand>,
    }

    /// A field of a record read moved into the program (S2.58): the
    /// field's place in the file's record format, the program field, and
    /// how its external text becomes a value.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct FieldMove {
        pub col: usize,
        pub target: Operand,
        pub kind: MoveKind,
        /// The record field's decimal places (S2.59: a number written
        /// takes them).
        pub decimals: u8,
    }

    /// The value of a field read (S2.58): a number, characters, or
    /// hexadecimal (the text gives two digits a byte).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum MoveKind {
        Num,
        Char,
        Hex,
    }

    /// One side of a comparison: a numeric expression with no
    /// arithmetic (whose precision would come from an EVAL), or a
    /// character expression.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum CmpSide {
        Num(NumExpr),
        Char(CharExpr),
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
        /// Fixed-form ADD, SUB, MULT, Z-ADD, Z-SUB (S2.3).
        Arith {
            op: FixOp,
            /// Factor 1 (None: the result field, for ADD/SUB/MULT).
            f1: Option<Operand>,
            f2: Operand,
            result: Operand,
            half_adjust: bool,
            /// It has resulting indicators.
            inds: bool,
        },
        /// EVAL/EVALR of a character field (S2.4).
        EvalChar {
            target: Operand,
            value: CharExpr,
            half_adjust: bool,
            right: bool,
        },
        /// EVAL (S2.2): a numeric field gets a numeric expression.
        Eval {
            target: Operand,
            value: NumExpr,
            half_adjust: bool,
            right: bool,
        },
        /// CALLP of a procedure of the module (S2.35, ADR 0007); its value,
        /// if any, is not used.
        Call(Call),
        /// EVAL with %SUBST as its target (S2.44): the character field,
        /// the start and the length (none: to the end) and the value.
        EvalSubst {
            base: Operand,
            start: Box<NumExpr>,
            len: Option<Box<NumExpr>>,
            value: Box<CharExpr>,
            half_adjust: bool,
            right: bool,
        },
        /// DSPLY with no response field (S2.36): the message, if any.
        Dsply(Option<ValExpr>),
        /// A read of a database file (S2.58): CHAIN, READ, READE, READP,
        /// READPE, SETLL, SETGT. The key's values (none for *LOVAL,
        /// *HIVAL, *START, *END or no key), then the record's fields into
        /// the program.
        FileRead {
            keys: Option<Vec<ValExpr>>,
            moves: Vec<FieldMove>,
        },
        /// WRITE, UPDATE or DELETE of a database file (S2.59): a DELETE's
        /// key, the program fields that make the record (none for
        /// DELETE), and how many fields the record format has.
        FileChange {
            keys: Option<Vec<ValExpr>>,
            fields: Vec<FieldMove>,
            width: usize,
        },
        /// RETURN (S2.38): the value, if any.
        Return(Option<ValExpr>),
        /// EVAL of a pointer field (S2.47): another pointer field, or the
        /// address of a field (%ADDR), `skip` bytes in (%ADDR(v:*DATA)).
        EvalPtr { target: Operand, value: PtrVal },
        /// DEALLOC (S2.50): the heap storage a pointer field addresses
        /// freed, and with (N) the field set to *NULL.
        Dealloc { target: Operand, null: bool },
        /// EXEC SQL with no host variable (S2.69): the runtime runs it,
        /// then its outcome goes into the SQLCA: SQLCODE, SQLSTATE,
        /// SQLERRMC, SQLERRD(3), SQLERRD(5).
        ExecSql { sqlca: Box<[Operand; 5]> },
        /// Dynamic SQL (S2.71): PREPARE (`name`, the text), EXECUTE
        /// IMMEDIATE (the text) or EXECUTE (`name`, the USING inputs, one
        /// group per host variable); then the SQLCA.
        ExecSqlDyn {
            sqlca: Box<[Operand; 5]>,
            kind: SqlDyn,
            name: String,
            text: Option<Operand>,
            inputs: Vec<Vec<SqlIn>>,
        },
        /// OPEN (S2.72): the cursor's name; a declared query's text in
        /// pieces around its host variables and their inputs, or the
        /// prepared statement's name and the USING inputs (one group per
        /// host variable); then the SQLCA.
        SqlOpen {
            sqlca: Box<[Operand; 5]>,
            name: String,
            pieces: Vec<String>,
            inputs: Vec<SqlIn>,
            prepared: Option<String>,
            using: Vec<Vec<SqlIn>>,
        },
        /// FETCH (S2.72): the cursor, the orientation and its number, the
        /// INTO targets (None with no INTO); then the SQLCA.
        SqlFetch {
            sqlca: Box<[Operand; 5]>,
            name: String,
            orient: String,
            n: i64,
            outputs: Option<Vec<SqlOut>>,
        },
        /// UPDATE or DELETE WHERE CURRENT OF (S2.73): the cursor, the
        /// table, the statement's text before WHERE in pieces around its
        /// host variables and their inputs; then the SQLCA.
        SqlPositioned {
            sqlca: Box<[Operand; 5]>,
            cursor: String,
            table: String,
            pieces: Vec<String>,
            inputs: Vec<SqlIn>,
        },
        /// FETCH ... FOR n ROWS INTO :dsarray (S2.74): the cursor, the
        /// orientation, the row count, the array's elements and the place
        /// of each subfield's first element; then the SQLCA.
        SqlFetchRows {
            sqlca: Box<[Operand; 5]>,
            name: String,
            orient: String,
            n: i64,
            count: SqlCount,
            dim: u32,
            host: String,
            fields: Vec<SqlRowField>,
        },
        /// INSERT INTO ... n ROWS VALUES (:dsarray) (S2.74): the text
        /// before the count, the row count, the array's elements and its
        /// subfields; then the SQLCA.
        SqlInsertRows {
            sqlca: Box<[Operand; 5]>,
            target: String,
            count: SqlCount,
            dim: u32,
            host: String,
            fields: Vec<SqlRowField>,
        },
        /// CALL of an SQL procedure (S2.75): the routine's name as
        /// sqlpl::parse_call gives it, and each argument; then the SQLCA.
        SqlCall {
            sqlca: Box<[Operand; 5]>,
            name: String,
            args: Vec<SqlArg>,
        },
        /// A fixed-form string operation (S2.83): its code, factor 1's
        /// characters, second characters (XLATE's to) or number, factor
        /// 2's characters and number, the P extender, the error and found
        /// indicators, the result.
        /// Boxed: the variants' sizes stay close (clippy::large_enum_variant).
        StrOp(Box<StrOpData>),
        /// SORTA (S2.86): the array (buf, off, stride, dim, type, lead),
        /// descending or not.
        Sorta {
            arr: (usize, usize, usize, u32, Ty, usize),
            descend: bool,
        },
        /// MOVEA of character data (S2.86): factor 2 and the result.
        Movea {
            from: MoveaOp,
            to: MoveaOp,
            pad: bool,
        },
        /// LOOKUP of an array or XFOOT (S2.85).
        ArrOp(Box<ArrOpData>),
        /// TESTN, TESTZ, TESTB, BITON or BITOFF (S2.84): the result field,
        /// the bits (a literal's mask, or a field), the indicators.
        BitOp(Box<BitOpData>),
        /// IN or OUT of data areas (S2.80): with *LOCK or not, and each
        /// data area's field name, its place and its *VAR: name field.
        DataArea {
            input: bool,
            lock: bool,
            fields: Vec<(String, Operand, Option<Operand>)>,
        },
        /// OPEN, CLOSE or FEOD of a printer file (S2.79).
        PrinterOp,
        /// WRITE of a printer file's record format (S2.79): the program
        /// fields it prints and the INDDS data structure, with places.
        PrinterWrite {
            fields: Vec<(String, Operand)>,
            indds: Option<Operand>,
        },
        /// EXCEPT (S2.78): the exception records' name, and the program
        /// fields their output specifications name, with their places.
        Except {
            name: Option<String>,
            fields: Vec<(String, Operand)>,
        },
        /// CLOSE (S2.72): the cursor; then the SQLCA.
        SqlClose {
            sqlca: Box<[Operand; 5]>,
            name: String,
        },
        /// GET DIAGNOSTICS (S2.71): CONDITION or not, each item's name and
        /// the fields it is assigned to; then the SQLCA.
        SqlDiag {
            sqlca: Box<[Operand; 5]>,
            condition: bool,
            items: Vec<(String, Vec<Operand>)>,
        },
        /// EXEC SQL with host variables (S2.70): the statement's text in
        /// pieces around its input host variables (bind_host's order),
        /// the inputs, and the INTO (or SET) targets; then the SQLCA.
        ExecSqlHosts {
            sqlca: Box<[Operand; 5]>,
            pieces: Vec<String>,
            inputs: Vec<SqlIn>,
            outputs: Vec<SqlOut>,
            into: bool,
        },
        /// EVAL %OCCUR(ds) = n, and OCCUR (S2.64): set the current
        /// occurrence of the data structure in `buf` (from a number, or
        /// from another one's current occurrence), then put it in a
        /// result field.
        SetOccur {
            buf: usize,
            to: Option<NumExpr>,
            result: Option<Operand>,
        },
        /// %ELEM of a varying-dimension array set (S2.45).
        SetElems {
            name: String,
            buf: usize,
            off: usize,
            stride: usize,
            dim: u32,
            ty: Ty,
            value: Box<NumExpr>,
        },
        /// EVAL of an indicator (S2.39): `*ON`, `*OFF` or a condition.
        EvalInd { name: String, value: IndValue },
        /// SETON / SETOFF of indicators (S2.39).
        SetInd { on: bool, inds: Vec<String> },
        /// EVAL of a figurative constant into a field (S2.22): stored as
        /// the interpreter stores `Value::Fig`.
        EvalFig {
            target: Operand,
            fig: FigOp,
            half_adjust: bool,
            right: bool,
        },
        /// MOVE and MOVEL (S2.11), with no result indicators: a field
        /// or a literal into a field.
        Move {
            left: bool,
            pad: bool,
            from: Operand,
            to: Operand,
        },
    }

    /// The fixed-form arithmetic operations lowered.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum FixOp {
        ZAdd,
        ZSub,
        Add,
        Sub,
        Mult,
        /// DIV (S2.7): the quotient gets the result field's decimal places
        /// (`decs`, one more with H); the remainder is kept for MVR.
        Div {
            decs: u8,
        },
        /// MVR (S2.7): the remainder of the last DIV.
        Mvr,
    }

    /// A character expression of lowered code: operands and their
    /// concatenation with `+`.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum CharExpr {
        /// %LEFT (`left`) or %RIGHT of a character expression, with a
        /// length that is a numeric literal or field (S2.67).
        LeftRight {
            left: bool,
            s: Box<CharExpr>,
            n: Operand,
            path: Vec<u8>,
        },
        /// An indicator as a part of a concatenation (S2.62): character
        /// data of length 1, '1' or '0' (Value::Ind's chars).
        Ind(String),
        Leaf(Operand),
        /// A chain of concatenations down the left operands (S2.41): the
        /// parts in order.
        Chain {
            parts: Vec<CharExpr>,
            path: Vec<u8>,
        },
        /// The character value a procedure of the module returns (S2.35).
        Call(Call),
        Cat {
            left: Box<CharExpr>,
            right: Box<CharExpr>,
            path: Vec<u8>,
        },
        /// %SUBST (S2.10): a character expression, its start and, when
        /// given, its length (literals or fields), and its place in the
        /// statement's expression.
        Subst {
            inner: Box<CharExpr>,
            start: Operand,
            len: Option<Operand>,
            path: Vec<u8>,
        },
        /// %CHAR of a date, time or timestamp field (S2.53), in the format
        /// bif_override picks (`rpgrt::native::temporal_char`).
        TemporalChar {
            of: Operand,
            fmt: TemporalFmt,
        },
        /// A date, time or timestamp plus or minus a duration (S2.55):
        /// `unit` is rpgrt::rpg::data::DurUnit's variant; `dur_first` when
        /// the duration is the left operand (evaluated first).
        TemporalAdd {
            base: Box<CharExpr>,
            unit: &'static str,
            n: Box<NumExpr>,
            minus: bool,
            dur_first: bool,
        },
        /// %DATE, %TIME or %TIMESTAMP (S2.54): the argument (none for
        /// none, *SYS or *JOB), a special value given as the format, and
        /// the control specification's DATFMT or TIMFMT code
        /// (`rpgrt::native::temporal_bif`).
        TemporalBif {
            kind: TemporalBifKind,
            arg: Option<Box<ValExpr>>,
            fmt: Option<String>,
            dft: String,
        },
        /// %CHAR of a numeric expression (S2.14), with the program's
        /// DECEDIT: the decimal point and whether a zero goes before it.
        CharOfNum {
            inner: NumExpr,
            point: char,
            zero: bool,
            path: Vec<u8>,
        },
        /// %EDITC of a numeric expression with a literal edit code
        /// (S2.26), with the program's DECEDIT; `own` when the value is a
        /// literal, whose own digits the interpreter uses.
        EditC {
            inner: NumExpr,
            code: char,
            point: char,
            zero: bool,
            own: bool,
            /// The third argument (S2.81): *ASTFILL, *CURSYM, or a literal's
            /// text.
            third: Option<String>,
            path: Vec<u8>,
        },
        /// %EDITW of a numeric expression with a literal edit word (S2.29);
        /// `prec` is a field's digits and decimal places, None for any
        /// other expression (its own digits, as the interpreter).
        EditW {
            inner: NumExpr,
            word: String,
            prec: Option<(u32, u32)>,
            path: Vec<u8>,
        },
        /// %SCANRPL (S2.28): `pat` replaced by `rep` in `s` from `start`
        /// for `len` characters; the places of `pat`, `rep` and `s` are 0,
        /// 1 and 2.
        ScanRpl {
            pat: Box<CharExpr>,
            rep: Box<CharExpr>,
            s: Box<CharExpr>,
            start: Option<Operand>,
            len: Option<Operand>,
            path: Vec<u8>,
        },
        /// %REPLACE (S2.23): `rep` in place of `len` characters of `s`
        /// from `start`; the places of `rep` and `s` are 0 and 1.
        Replace {
            rep: Box<CharExpr>,
            s: Box<CharExpr>,
            start: Option<Operand>,
            len: Option<Operand>,
            path: Vec<u8>,
        },
        /// %XLATE (S2.12): the characters `from` becomes `to` in `s`,
        /// from `start` on; the places of its operands are its path and
        /// 0, 1 and 2.
        Xlate {
            from: Box<CharExpr>,
            to: Box<CharExpr>,
            s: Box<CharExpr>,
            start: Option<Operand>,
            path: Vec<u8>,
        },
        /// %TRIM (both), %TRIML (`left`) or %TRIMR (`right`) of one
        /// argument (S2.9), and its place in the statement's expression.
        Trim {
            inner: Box<CharExpr>,
            left: bool,
            right: bool,
            path: Vec<u8>,
        },
    }

    /// +, - and * in lowered code.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ArOp {
        Add,
        Sub,
        Mul,
        /// `/` (S2.17): the quotient's decimal places come from the
        /// operands' precision.
        Div,
    }

    /// A numeric expression of lowered code.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum NumExpr {
        Leaf(Operand),
        /// %DIFF(x : y : unit) and %SUBDT(x : unit) (S2.55): temporal
        /// values, and rpgrt::rpg::data::DurUnit's variant.
        TemporalDiff {
            x: Box<CharExpr>,
            y: Box<CharExpr>,
            unit: &'static str,
        },
        TemporalSubdt {
            x: Box<CharExpr>,
            unit: &'static str,
        },
        /// An operation, and its place in the statement's expression (0
        /// left, 1 right from the root): where the runtime finds it to give
        /// its precision.
        Bin {
            op: ArOp,
            left: Box<NumExpr>,
            right: Box<NumExpr>,
            path: Vec<u8>,
        },
        /// A chain of +, - and * down an operation's left operands (S2.41,
        /// ADR 0007): `first`, then each step's operator, right operand and
        /// the path of its operation, in the interpreter's order. Generated
        /// as straight-line code, however long the chain.
        Chain {
            first: Box<NumExpr>,
            steps: Vec<(ArOp, NumExpr, Vec<u8>)>,
            path: Vec<u8>,
        },
        /// %DIV (`rem` false) or %REM of two numeric expressions
        /// (S2.18): the integer quotient, or the remainder.
        DivRem {
            left: Box<NumExpr>,
            right: Box<NumExpr>,
            rem: bool,
            path: Vec<u8>,
        },
        /// %LEN of a varying-length field (S2.20): its current length.
        VarLen {
            var: Operand,
        },
        /// %STATUS with no file (S2.31): the most recent status code.
        Status,
        /// %ELEM of a varying-dimension array (S2.45): its elements in use.
        Elems(String),
        /// %ABS of a numeric expression (S2.32).
        Abs(Box<NumExpr>),
        /// The numeric value a procedure of the module returns (S2.35).
        Call(Call),
        /// %CHECK (`rev` false) or %CHECKR of character expressions, with
        /// no start (S2.34).
        Check {
            set: Box<CharExpr>,
            s: Box<CharExpr>,
            rev: bool,
            path: Vec<u8>,
        },
        /// %MAX (`max`) or %MIN of two or more numeric expressions
        /// (S2.33): the first of the greatest, or of the smallest.
        MinMax {
            args: Vec<NumExpr>,
            max: bool,
            path: Vec<u8>,
        },
        /// %LOOKUP and %LOOKUPxx (S2.27): `x` searched in a global
        /// array (`buf`, `off`, `stride`, `dim`, element type), from
        /// `start` for `n` elements; `how` is "", LT, LE, GE or GT.
        // The operands boxed (S2.52): an operand is large, with a date
        // format in its type (clippy::large_enum_variant).
        Lookup {
            x: Box<Operand>,
            buf: usize,
            off: usize,
            stride: usize,
            dim: u32,
            ty: Ty,
            start: Option<Box<Operand>>,
            n: Option<Box<Operand>>,
            how: String,
        },
        /// %INT or %INTH (`half`) of a numeric expression (S2.14).
        Int {
            inner: Box<NumExpr>,
            half: bool,
            path: Vec<u8>,
        },
        /// %DEC or %DECH (`half`) of a numeric expression, with literal
        /// digits and decimal places, or none (S2.14).
        Dec {
            inner: Box<NumExpr>,
            to: Option<(u32, u32)>,
            half: bool,
            path: Vec<u8>,
        },
        /// %SCAN (S2.12): where `pat` first is in `s`, from `start` on.
        Scan {
            pat: Box<CharExpr>,
            s: Box<CharExpr>,
            start: Option<Operand>,
            path: Vec<u8>,
        },
        /// %XFOOT, %MAXARR or %MINARR of a whole global array (S2.68):
        /// `kind` is "XFOOT", "MAXARR" or "MINARR"; the array as for
        /// %LOOKUP.
        ArrFold {
            kind: String,
            buf: usize,
            off: usize,
            stride: usize,
            dim: u32,
            ty: Ty,
        },
        /// %SCANR of character expressions, with a start and a length
        /// that are numeric literals or fields (S2.67).
        ScanR {
            pat: Box<CharExpr>,
            s: Box<CharExpr>,
            start: Option<Operand>,
            len: Option<Operand>,
            path: Vec<u8>,
        },
    }

    /// What a pointer is set to (S2.47).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum PtrVal {
        Var(Operand),
        Addr {
            of: Operand,
            skip: usize,
        },
        /// Pointer arithmetic (S2.49): the base moved by each number in
        /// turn, back when `true` (ptr - n).
        Move {
            base: Box<PtrVal>,
            by: Vec<(bool, NumExpr)>,
        },
        /// %ALLOC(n) (S2.50): new heap storage.
        Alloc(NumExpr),
        /// *NULL as an argument (S2.51).
        Null,
        /// %REALLOC(p: n) (S2.50): the heap storage made n bytes, the
        /// pointer evaluated first (the bif's arguments in order).
        Realloc {
            p: Box<PtrVal>,
            size: NumExpr,
        },
    }

    /// The value an indicator is set to (S2.39).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum IndValue {
        On,
        Off,
        /// Boxed (S2.52): a condition is large (clippy::large_enum_variant).
        Cond(Box<Cond>),
    }

    /// A numeric or a character expression: a DSPLY message (S2.36), a
    /// RETURN value (S2.38).
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ValExpr {
        Num(NumExpr),
        Char(CharExpr),
    }

    /// A call of a procedure of the module (S2.35, ADR 0007): its name, and
    /// its arguments in parameter order.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Call {
        pub proc: String,
        pub args: Vec<CallArg>,
        pub path: Vec<u8>,
    }

    /// An argument: the caller's storage, for a parameter passed by
    /// reference; or a value, for VALUE and CONST parameters.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum CallArg {
        Ref(Operand),
        Num(NumExpr),
        Char(CharExpr),
        /// A pointer value for a pointer parameter (S2.51).
        Ptr(PtrVal),
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
    /// An instruction the routine cannot be generated with (S2.41).
    Refused {
        at: usize,
        line: usize,
        why: String,
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
            Error::Refused { at, line, why } => {
                write!(f, "instruction {at} (line {line}): {why}")
            }
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
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
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

/// How an instruction runs (ADR 0007, phase A), for the source map:
/// "native" when its arm never calls the interpreter (every lowered
/// instruction since S2.37: no fallback), and "interp" when it always
/// does (stage 1). "fallback" was a lowered instruction that the
/// interpreter ran when `native_ok` was false (S2.35-S2.36).
fn via(i: &ir::Ins) -> &'static str {
    use ir::Kind::*;
    // Lowered code has no interpreter fallback since S2.37.
    if i.lowered.is_some() {
        return "native";
    }
    match i.kind {
        // EXSR runs the subroutine's generated code; LEAVESR and a GOTO out
        // of a subroutine are signals (S2.40): no interpreter evaluation.
        Jump(_) | Label | Nop | Monitor { .. } | EndMonitor { .. } | ExSr | LeaveSr | GotoOut => {
            "native"
        }
        Stmt
        | JumpIfNot(_)
        | LoopTest { .. }
        | LoopStep
        | EachInit
        | EachNext { .. }
        | Return
        | Cab { .. }
        | EndSr => "interp",
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
        Char(n) => format!("{p}::Char({n})"),
        Varchar(n) => format!("{p}::Varchar({n})"),
        Pointer => format!("{p}::Pointer"),
        Date(n, sep) => {
            format!("{p}::Date(rpgrt::rpg::dates::DateFmt {{ name: {n:?}, sep: {sep:?} }})")
        }
        Time(n, sep) => {
            format!("{p}::Time(rpgrt::rpg::dates::TimeFmt {{ name: {n:?}, sep: {sep:?} }})")
        }
        Timestamp(n) => format!("{p}::Timestamp({n})"),
    }
}

/// Where an array element with a run-time index is (S2.21): the
/// interpreter's `elem_off`, the index evaluated first.
fn elem_at(off: usize, ix: &ir::Index) -> String {
    match &ix.vary {
        // A varying-dimension array (S2.45): within its elements in use.
        Some((name, _)) => format!(
            "{{ let i = {}; rpgrt::native::elem_in_use({off}, {}, {}, &i, rt.elems({}))? }}",
            value(&ix.of),
            ix.stride,
            ix.dim,
            json_str(name)
        ),
        None => {
            // The outer index's element first (S2.77): its value read and
            // checked before this one's, as path_sym goes from the left.
            let base = match &ix.outer {
                Some(o) => elem_at(off, o),
                None => off.to_string(),
            };
            format!(
                "rpgrt::native::elem_at({base}, {}, {}, &{})?",
                ix.stride,
                ix.dim,
                value(&ix.of)
            )
        }
    }
}

/// Where a target operand of lowered code is (see `place`), an array
/// element with a run-time index included.
fn place_of(o: &ir::Operand) -> Option<(String, String, String)> {
    match o {
        ir::Operand::Var {
            off,
            index: None,
            based: Some(b),
            ..
        } => Some((
            format!("    let (sb, so) = {};\n", based_at(b, *off)),
            "sb".to_string(),
            "so".to_string(),
        )),
        ir::Operand::Var { based: Some(_), .. } => None,
        // An element of a procedure's local array (S2.63): its place in the
        // frame (`rt.local`), as `Machine::sym` then `elem_off` give it.
        // A varying-dimension one is not lowered.
        ir::Operand::Var {
            buf,
            off,
            index: Some(ix),
            local: true,
            ..
        } => match &ix.vary {
            Some(_) => None,
            None => Some((
                format!(
                    "    let o = {};\n    let (sb, so) = rt.local({buf}, o)?;\n",
                    elem_at(*off, ix)
                ),
                "sb".to_string(),
                "so".to_string(),
            )),
        },
        ir::Operand::Var {
            buf,
            off,
            index: Some(ix),
            ..
        } => Some((
            match &ix.vary {
                // Written (S2.45): a DIM(*AUTO) array grows to the index,
                // before the store, as the interpreter's assign does.
                Some((name, auto)) => format!(
                    "    let i = {};\n    let n = rt.elems({name});\n    let (so, grow) = rpgrt::native::elem_grow({off}, {}, {}, &i, n, {auto})?;\n    if let Some(k) = grow {{\n        rt.set_elem_count({name}, k);\n    }}\n",
                    value(&ix.of),
                    ix.stride,
                    ix.dim,
                    name = json_str(name)
                ),
                None => format!("    let so = {};\n", elem_at(*off, ix)),
            },
            buf.to_string(),
            "so".to_string(),
        )),
        ir::Operand::Var {
            buf, off, local, ..
        } => Some(place(*buf, *off, *local)),
        _ => None,
    }
}

/// The buffer of an array element whose offset `o` is worked out (S2.63):
/// a global's buffer, or for a procedure local its place in the frame.
fn elem_place(buf: usize, local: bool) -> String {
    if local {
        format!("let (b, o) = rt.local({buf}, o)?;")
    } else {
        format!("let b = {buf};")
    }
}

/// Output fields' places as `fields` (S2.78, S2.79).
fn out_slots(
    body: &mut String,
    fields: &[(String, ir::Operand)],
    bad: &dyn Fn() -> Error,
) -> Result<(), Error> {
    let fm = if fields.is_empty() { "" } else { "mut " };
    let _ = writeln!(
        body,
        "    let {fm}fields: Vec<rpgrt::native::OutSlot> = Vec::new();"
    );
    for (n, o) in fields {
        let ir::Operand::Var { ty: t, .. } = o else {
            return Err(bad());
        };
        let at = scalar_place(o).ok_or_else(bad)?;
        let _ = writeln!(
            body,
            "    {{\n        let (buf, off) = {at};\n        fields.push(rpgrt::native::OutSlot {{ name: {}.to_string(), buf, off, ty: {} }});\n    }}",
            json_str(n),
            ty(t)
        );
    }
    Ok(())
}

/// The SQLCA stored from the outcome `o` (S2.69-S2.71), as exec_sql.rs
/// exec_sql assigns it: SQLCODE, SQLSTATE, SQLERRMC, SQLERRD(3) and
/// SQLERRD(5), each with the truncating fit.
fn sqlca_store(
    body: &mut String,
    sqlca: &[ir::Operand; 5],
    bad: &dyn Fn() -> Error,
) -> Result<(), Error> {
    let values = [
        "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.code))",
        "rpgrt::rpg::data::Value::str(&o.state)",
        "rpgrt::rpg::data::Value::str(&o.text)",
        "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.rows))",
        "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.errd5))",
    ];
    for (target, v) in sqlca.iter().zip(values) {
        let ir::Operand::Var { ty: t, .. } = target else {
            return Err(bad());
        };
        let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
        let _ = writeln!(
            body,
            "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &{v},\n        rpgrt::rpg::data::Fit::Truncate {{ half_adjust: false }},\n        false,\n    )?;",
            ty(t)
        );
    }
    Ok(())
}

/// A multiple-row statement's row count (S2.74) as `count`: a number as
/// written (exec_sql.rs: Dec::parse), or a host variable's value.
fn sql_count(c: &ir::SqlCount) -> String {
    match c {
        ir::SqlCount::Lit(t) => format!(
            "    let count = rpgrt::rpg::dec::Dec::parse({})?;\n",
            json_str(t)
        ),
        ir::SqlCount::Var(o) => format!("    let count = {};\n", number(o)),
    }
}

/// A host structure array's subfields as `fields` (S2.74).
fn sql_row_fields(body: &mut String, fields: &[ir::SqlRowField]) {
    let parts: Vec<String> = fields
        .iter()
        .map(|f| {
            format!(
                "rpgrt::native::SqlRowSlot {{ buf: {}, off: {}, stride: {}, ty: {}, numeric: {} }}",
                f.buf,
                f.off,
                f.stride,
                ty(&f.ty),
                f.numeric
            )
        })
        .collect();
    let _ = writeln!(body, "    let fields = vec![{}];", parts.join(", "));
}

/// The USING literals (S2.71), one per host variable as exec_sql.rs
/// using_values makes them: `NULL` for a negative indicator, else
/// host_literal's literals of its fields joined with ", ".
fn sql_inputs(body: &mut String, groups: &[Vec<ir::SqlIn>]) {
    let lm = if groups.is_empty() { "" } else { "mut " };
    let _ = writeln!(body, "    let {lm}lits: Vec<String> = Vec::new();");
    for g in groups {
        let null = match g.first().and_then(|i| i.ind.as_ref()) {
            Some(ind) => format!("{}.is_negative()", number(ind)),
            None => "false".to_string(),
        };
        let _ = writeln!(
            body,
            "    if {null} {{\n        lits.push(\"NULL\".to_string());\n    }} else {{"
        );
        let gm = if g.is_empty() { "" } else { "mut " };
        let _ = writeln!(body, "        let {gm}parts: Vec<String> = Vec::new();");
        for i in g {
            let cl = match i.char_len {
                Some(n) => format!("Some({n})"),
                None => "None".to_string(),
            };
            let _ = writeln!(
                body,
                "        {{\n            let v = {};\n            parts.push(rpgrt::native::sql_literal(&v, {cl})?);\n        }}",
                value(&i.value)
            );
        }
        body.push_str("        lits.push(parts.join(\", \"));\n    }\n");
    }
}

/// Where a field with no index that is not based is, as one `(buf, off)`
/// expression (S2.70): a global's own place, or a procedure local's place
/// for this call (`rt.local`).
fn scalar_place(o: &ir::Operand) -> Option<String> {
    match o {
        ir::Operand::Var {
            buf,
            off,
            local,
            index: None,
            based: None,
            ..
        } => Some(if *local {
            format!("rt.local({buf}, {off})?")
        } else {
            format!("({buf}, {off})")
        }),
        _ => None,
    }
}

/// Where a field written by lowered code is: a statement to put first
/// (empty for a global), then the buffer and offset expressions. A
/// procedure local's place is the runtime's, for this call.
fn place(buf: usize, off: usize, local: bool) -> (String, String, String) {
    if local {
        (
            format!("    let (sb, so) = rt.local({buf}, {off})?;\n"),
            "sb".to_string(),
            "so".to_string(),
        )
    } else {
        (String::new(), buf.to_string(), off.to_string())
    }
}

/// Where a based field is now (S2.48): the pointer's value, then the
/// interpreter's `based_place` (`Runtime::based_at`).
fn based_at(b: &ir::Based, off: usize) -> String {
    format!(
        "{{ let p = {}; rt.based_at(&p, {off}, {})? }}",
        value(&b.ptr),
        b.size
    )
}

/// An operand as a `Value` expression (inside a helper with `rt`).
fn value(o: &ir::Operand) -> String {
    match o {
        ir::Operand::Var {
            off,
            ty: t,
            based: Some(b),
            ..
        } => format!(
            "{{ let (b, o) = {}; rpgrt::native::load(rt.mem(), b, o, {})? }}",
            based_at(b, *off),
            ty(t)
        ),
        ir::Operand::Var {
            buf,
            off,
            ty: t,
            index: Some(ix),
            local,
            ..
        } => format!(
            "{{ let o = {}; {} rpgrt::native::load(rt.mem(), b, o, {})? }}",
            elem_at(*off, ix),
            elem_place(*buf, *local),
            ty(t)
        ),
        ir::Operand::Var {
            buf,
            off,
            ty: t,
            local,
            ..
        } => {
            if *local {
                format!(
                    "{{ let (b, o) = rt.local({buf}, {off})?; rpgrt::native::load(rt.mem(), b, o, {})? }}",
                    ty(t)
                )
            } else {
                format!("rpgrt::native::load(rt.mem(), {buf}, {off}, {})?", ty(t))
            }
        }
        ir::Operand::Num { m, s } => {
            format!("rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::new({m}, {s}))")
        }
        ir::Operand::Str(b) => format!("rpgrt::rpg::data::Value::Char(vec!{b:?})"),
        ir::Operand::Fig(f) => fig(f),
        ir::Operand::Occur { buf, .. } => {
            format!("rpgrt::rpg::data::Value::Num(rt.occurrence({buf})?)")
        }
    }
}

/// A pointer value (S2.47, S2.49) as a `Value` expression: a field, the
/// pointer to a field's place, or one moved by numbers in turn
/// (`rpgrt::native::ptr_move`, ptrs.rs ptr_arith).
fn ptr_val(pc: usize, pv: &ir::PtrVal, helpers: &mut String) -> Option<String> {
    Some(match pv {
        ir::PtrVal::Var(o) => value(o),
        ir::PtrVal::Addr { of, skip } => {
            let (pre, b, o) = place_of(of)?;
            format!(
                "{{\n{pre}    let at = usize::checked_add({o}, {skip}).ok_or_else(|| anyhow::anyhow!(\"native: an address\"))?;\n    rt.addr_of({b}, at)\n    }}"
            )
        }
        ir::PtrVal::Move { base, by } => {
            let mut code = format!("{{\n    let mut p = {};\n", ptr_val(pc, base, helpers)?);
            for (neg, n) in by {
                let n = num_expr(pc, n, helpers);
                let _ = writeln!(code, "    p = rpgrt::native::ptr_move(&p, &{n}, {neg})?;");
            }
            code.push_str("    p\n    }");
            code
        }
        ir::PtrVal::Null => fig(&ir::FigOp::Null),
        ir::PtrVal::Alloc(n) => {
            let n = num_expr(pc, n, helpers);
            format!("{{\n    let n = {n};\n    rt.alloc(&n)?\n    }}")
        }
        ir::PtrVal::Realloc { p, size } => {
            let pv = ptr_val(pc, p, helpers)?;
            let n = num_expr(pc, size, helpers);
            format!("{{\n    let p = {pv};\n    let n = {n};\n    rt.realloc(&p, &n)?\n    }}")
        }
    })
}

/// A figurative constant as a `Value` expression.
fn fig(f: &ir::FigOp) -> String {
    let p = "rpgrt::rpg::data";
    match f {
        ir::FigOp::Blanks => format!("{p}::Value::Fig({p}::Fig::Blanks)"),
        ir::FigOp::Zeros => format!("{p}::Value::Fig({p}::Fig::Zeros)"),
        ir::FigOp::HiVal => format!("{p}::Value::Fig({p}::Fig::HiVal)"),
        ir::FigOp::LoVal => format!("{p}::Value::Fig({p}::Fig::LoVal)"),
        ir::FigOp::All(b) => format!("{p}::Value::Fig({p}::Fig::All(vec!{b:?}))"),
        ir::FigOp::Null => format!("{p}::Value::Fig({p}::Fig::Null)"),
    }
}

/// An operand as a `Dec` expression.
fn number(o: &ir::Operand) -> String {
    match o {
        ir::Operand::Var {
            off,
            ty: t,
            based: Some(b),
            ..
        } => format!(
            "{{ let (b, o) = {}; rpgrt::native::load_num(rt.mem(), b, o, {})? }}",
            based_at(b, *off),
            ty(t)
        ),
        ir::Operand::Var {
            buf,
            off,
            ty: t,
            index: Some(ix),
            local,
            ..
        } => format!(
            "{{ let o = {}; {} rpgrt::native::load_num(rt.mem(), b, o, {})? }}",
            elem_at(*off, ix),
            elem_place(*buf, *local),
            ty(t)
        ),
        ir::Operand::Var {
            buf,
            off,
            ty: t,
            local,
            ..
        } => {
            if *local {
                format!(
                    "{{ let (b, o) = rt.local({buf}, {off})?; rpgrt::native::load_num(rt.mem(), b, o, {})? }}",
                    ty(t)
                )
            } else {
                format!(
                    "rpgrt::native::load_num(rt.mem(), {buf}, {off}, {})?",
                    ty(t)
                )
            }
        }
        ir::Operand::Num { m, s } => format!("rpgrt::rpg::dec::Dec::new({m}, {s})"),
        // Never in a numeric place (the lowering checks); as in the
        // interpreter, a character value's number is an error.
        ir::Operand::Str(b) => format!("rpgrt::rpg::data::Value::Char(vec!{b:?}).num()?"),
        // Never in a numeric place either (the lowering checks); as in the
        // interpreter, its number is `Value::num`'s.
        ir::Operand::Fig(f) => format!("{}.num()?", fig(f)),
        ir::Operand::Occur { buf, .. } => format!("rt.occurrence({buf})?"),
    }
}

fn name_of(o: &ir::Operand) -> String {
    match o {
        ir::Operand::Var { name, .. } => comment(name),
        ir::Operand::Num { m, s } => format!("{m}e-{s}"),
        ir::Operand::Str(b) => format!("a character literal of {} bytes", b.len()),
        ir::Operand::Fig(f) => format!("{f:?}"),
        ir::Operand::Occur { name, .. } => format!("%OCCUR({})", comment(name)),
    }
}

/// A comparison's side as a `Value` expression.
fn side(pc: usize, s: &ir::CmpSide, helpers: &mut String) -> String {
    match s {
        ir::CmpSide::Num(e) => {
            format!("rpgrt::rpg::data::Value::Num({})", num_expr(pc, e, helpers))
        }
        ir::CmpSide::Char(e) => char_expr(pc, e, helpers),
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
        ir::Cond::CmpExpr(op, a, b) => {
            let op = match op {
                ir::CmpOp::Eq => "Eq",
                ir::CmpOp::Ne => "Ne",
                ir::CmpOp::Lt => "Lt",
                ir::CmpOp::Le => "Le",
                ir::CmpOp::Gt => "Gt",
                ir::CmpOp::Ge => "Ge",
            };
            // The sides' helpers go after this function.
            let mut extra = String::new();
            let x = side(pc, a, &mut extra);
            let y = side(pc, b, &mut extra);
            let _ = writeln!(out, "    // an expression {op} an expression");
            let _ = writeln!(out, "    let x = {x};");
            let _ = writeln!(out, "    let y = {y};");
            let _ = writeln!(
                out,
                "    rpgrt::native::cmp(rpgrt::native::CmpOp::{op}, &x, &y)\n}}"
            );
            out.push_str(&extra);
        }
        ir::Cond::Ind(name) => {
            let _ = writeln!(out, "    Ok(rt.ind({}))\n}}", json_str(name));
        }
        ir::Cond::FileStatus(bit, file) => {
            let f = match file {
                Some(f) => format!("Some({})", json_str(f)),
                None => "None".to_string(),
            };
            let _ = writeln!(
                out,
                "    rt.file_status(rpgrt::native::FileBit::{bit:?}, {f})\n}}"
            );
        }
        ir::Cond::Not(x) => {
            *n = n.saturating_add(1);
            let xi = format!("c{pc}_{n}");
            let _ = writeln!(out, "    Ok(!{xi}(rt)?)\n}}");
            cond_fns(pc, x, &xi, out, n);
        }
        ir::Cond::Chain(and, conds) => {
            // In order, stopping as the interpreter's AND/OR do: AND at the
            // first false, OR at the first true.
            let mut names = Vec::with_capacity(conds.len());
            for _ in conds {
                *n = n.saturating_add(1);
                names.push(format!("c{pc}_{n}"));
            }
            let stop = if *and { "!" } else { "" };
            for nm in &names {
                let _ = writeln!(
                    out,
                    "    if {stop}{nm}(rt)? {{\n        return Ok({});\n    }}",
                    !*and
                );
            }
            let _ = writeln!(out, "    Ok({and})\n}}");
            for (nm, c) in names.iter().zip(conds) {
                cond_fns(pc, c, nm, out, n);
            }
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
    // ADR 0007 (S2.37): lowered code runs only natively; the runtime
    // refuses a routine outside its own context before any of it runs, so
    // there is no fallback to the interpreter here.
    let guarded = |interp: String, native: String| {
        let _ = interp;
        native
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
            let ir::Operand::Var { ty: t, .. } = var else {
                return Err(bad());
            };
            let (prelude, sb, so) = place_of(var).ok_or_else(bad)?;
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
                "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &rpgrt::rpg::data::Value::Num(n),\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: false }},\n        false,\n    )\n}}",
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
            let ir::Operand::Var { ty: t, .. } = target else {
                return Err(bad());
            };
            let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
            // The expression's helpers first, then this one whole.
            let v = num_expr(pc, value, helpers);
            let _ = writeln!(
                helpers,
                "\nfn ev{pc}(rt: &mut dyn Runtime) -> NResult<()> {{"
            );
            let _ = writeln!(helpers, "    let v = {v};");
            let _ = writeln!(
                helpers,
                "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &rpgrt::rpg::data::Value::Num(v),\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: {half_adjust} }},\n        {right},\n    )\n}}",
                ty(t)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ev{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::EvalChar {
                target,
                value,
                half_adjust,
                right,
            },
        ) => {
            let ir::Operand::Var { ty: t, .. } = target else {
                return Err(bad());
            };
            let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
            let v = char_expr(pc, value, helpers);
            let _ = writeln!(
                helpers,
                "\nfn ec{pc}(rt: &mut dyn Runtime) -> NResult<()> {{"
            );
            let _ = writeln!(helpers, "    let v = {v};");
            let _ = writeln!(
                helpers,
                "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &v,\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: {half_adjust} }},\n        {right},\n    )\n}}",
                ty(t)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ec{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::EvalSubst {
                base,
                start,
                len,
                value,
                half_adjust,
                right,
            },
        ) => {
            // The interpreter's order: the value (Stmt::Eval), then in
            // assign the field, the start and the length; then the field
            // stored again with the part replaced.
            let ir::Operand::Var { ty: t, .. } = base else {
                return Err(bad());
            };
            let (prelude, sb, so) = place_of(base).ok_or_else(bad)?;
            let v = char_expr(pc, value, helpers);
            let st = num_expr(pc, start, helpers);
            let ln = match len {
                Some(l) => format!("Some({})", num_expr(pc, l, helpers)),
                None => "None".to_string(),
            };
            let fit = format!("rpgrt::rpg::data::Fit::Strict {{ half_adjust: {half_adjust} }}");
            let _ = writeln!(
                helpers,
                "\nfn es{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n    let v = {v};\n{prelude}    let cur = rpgrt::native::load(rt.mem(), {sb}, {so}, {})?.chars()?;\n    let st = {st};\n    let ln = {ln};\n    let cur = rpgrt::native::subst_target(cur, &st, ln.as_ref(), &v, {fit}, {right})?;\n    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &rpgrt::rpg::data::Value::Char(cur),\n        {fit},\n        false,\n    )\n}}",
                ty(t),
                ty(t)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("es{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::Dsply(msg)) => {
            // The interpreter's text_of: characters as text, a number as
            // %CHAR gives it with a period and no DECEDIT (char_of_num).
            let text = match msg {
                None => "String::new()".to_string(),
                Some(ir::ValExpr::Char(e)) => {
                    format!(
                        "rpgrt::rpg::data::a(&{}.chars()?)",
                        char_expr(pc, e, helpers)
                    )
                }
                Some(ir::ValExpr::Num(e)) => format!(
                    "rpgrt::rpg::data::a(&rpgrt::native::char_of_num(&{}, '.', false).chars()?)",
                    num_expr(pc, e, helpers)
                ),
            };
            let _ = writeln!(
                helpers,
                "\nfn dy{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n    let text = {text};\n    rt.dsply(text);\n    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("dy{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::FileChange {
                keys,
                fields,
                width,
            },
        ) => {
            // The interpreter's file_op for WRITE, UPDATE and DELETE: the
            // start and the authority check (rt.file_change_start), the
            // record from the program's fields (record_of), the key, then
            // the change and the end (rt.file_change).
            // DELETE fills none of the fields: no `mut` (unused_mut).
            let m = if fields.is_empty() { "" } else { "mut " };
            let mut rec_fn = format!(
                "\nfn fw{pc}(rt: &mut dyn Runtime) -> NResult<Vec<Option<String>>> {{\n    let {m}rec: Vec<Option<String>> = vec![None; {width}];\n"
            );
            for f in fields {
                let ir::Operand::Var { ty: t, .. } = &f.target else {
                    return Err(bad());
                };
                let (prelude, sb, so) = place_of(&f.target).ok_or_else(bad)?;
                let _ = writeln!(
                    rec_fn,
                    "{prelude}    let v = rpgrt::native::load(rt.mem(), {sb}, {so}, {})?;\n    if let Some(slot) = rec.get_mut({}) {{\n        *slot = Some(rpgrt::native::field_text(&v, rpgrt::native::MoveKind::{:?}, {})?);\n    }}",
                    ty(t),
                    f.col,
                    f.kind,
                    f.decimals
                );
            }
            rec_fn.push_str("    Ok(rec)\n}\n");
            helpers.push_str(&rec_fn);
            let mut body = format!("    rt.file_change_start({pc})?;\n    let rec = fw{pc}(rt);\n");
            match keys {
                None => body.push_str("    let keys = None;\n"),
                Some(ks) => {
                    body.push_str("    let keys = Some(vec![\n");
                    for k in ks {
                        let v = match k {
                            ir::ValExpr::Num(e) => {
                                format!("rpgrt::native::KeyArg::Num({})", num_expr(pc, e, helpers))
                            }
                            ir::ValExpr::Char(e) => format!(
                                "rpgrt::native::KeyArg::Char({}.chars()?)",
                                char_expr(pc, e, helpers)
                            ),
                        };
                        let _ = writeln!(body, "        {v},");
                    }
                    body.push_str("    ]);\n");
                }
            }
            let _ = writeln!(
                helpers,
                "\nfn fc{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    rt.file_change({pc}, keys, rec)\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("fc{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::FileRead { keys, moves }) => {
            // The interpreter's file_op: the key's values (key_values),
            // the operation (rt.file_read), the record's fields into the
            // program (fill_fields), then the status, the indicators and
            // the file information data structure (rt.file_done), which
            // also takes an error of the moves.
            let mut body = String::new();
            match keys {
                None => body.push_str("    let keys = None;\n"),
                Some(ks) => {
                    body.push_str("    let keys = Some(vec![\n");
                    for k in ks {
                        let v = match k {
                            ir::ValExpr::Num(e) => {
                                format!("rpgrt::native::KeyArg::Num({})", num_expr(pc, e, helpers))
                            }
                            ir::ValExpr::Char(e) => format!(
                                "rpgrt::native::KeyArg::Char({}.chars()?)",
                                char_expr(pc, e, helpers)
                            ),
                        };
                        let _ = writeln!(body, "        {v},");
                    }
                    body.push_str("    ]);\n");
                }
            }
            body.push_str("    let rec = rt.file_read(");
            let _ = writeln!(body, "{pc}, keys)?;");
            if moves.is_empty() {
                body.push_str("    let _ = rec;\n    let moved = Ok(());\n");
            } else {
                // The moves in a function of their own, so that an error
                // of one comes back to file_done.
                let mut mv_fn = format!(
                    "\nfn fm{pc}(rt: &mut dyn Runtime, rec: Option<Vec<Option<String>>>) -> NResult<()> {{\n    let Some(rec) = rec else {{ return Ok(()) }};\n"
                );
                for mv in moves {
                    let ir::Operand::Var { ty: t, .. } = &mv.target else {
                        return Err(bad());
                    };
                    let (prelude, sb, so) = place_of(&mv.target).ok_or_else(bad)?;
                    let get = match mv.col {
                        0 => "rec.first()".to_string(),
                        c => format!("rec.get({c})"),
                    };
                    let _ = writeln!(
                        mv_fn,
                        "    if let Some(text) = {get}.and_then(|t| t.as_deref()) {{\n        let v = rpgrt::native::field_value(text, rpgrt::native::MoveKind::{:?})?;\n{prelude}        rpgrt::native::store(\n            rt.mem(),\n            {sb},\n            {so},\n            {},\n            &v,\n            rpgrt::rpg::data::Fit::Strict {{ half_adjust: false }},\n            false,\n        )?;\n    }}",
                        mv.kind,
                        ty(t)
                    );
                }
                mv_fn.push_str("    Ok(())\n}\n");
                helpers.push_str(&mv_fn);
                let _ = writeln!(body, "    let moved = fm{pc}(rt, rec);");
            }
            let _ = writeln!(
                helpers,
                "\nfn fr{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    rt.file_done({pc}, moved)\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("fr{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::EvalInd { name, value }) => {
            // Stmt::Eval of an indicator: the value's truth, then set_ind.
            let v = match value {
                ir::IndValue::On => "true".to_string(),
                ir::IndValue::Off => "false".to_string(),
                ir::IndValue::Cond(c) => {
                    let mut n = 0;
                    cond_fns(pc, c, &format!("ci{pc}"), helpers, &mut n);
                    format!("ci{pc}(rt)?")
                }
            };
            let _ = writeln!(
                helpers,
                "\nfn ei{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n    let v = {v};\n    rt.set_ind({}, v);\n    Ok(())\n}}",
                json_str(name)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ei{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::SetInd { on, inds }) => {
            let mut body = String::new();
            for i in inds {
                let _ = writeln!(body, "    rt.set_ind({}, {on});", json_str(i));
            }
            let _ = writeln!(
                helpers,
                "\nfn si{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("si{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::SetElems {
                name,
                buf,
                off,
                stride,
                dim,
                ty: t,
                value,
            },
        ) => {
            // The interpreter's assign_value "ELEM" arm: the value, the
            // range check, new elements initialized, the count set.
            let v = num_expr(pc, value, helpers);
            let _ = writeln!(
                helpers,
                "\nfn se{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n    let v = {v};\n    let cur = rt.elems({nm});\n    let arr = rpgrt::native::ArrayPlace {{ buf: {buf}, off: {off}, stride: {stride}, dim: {dim}, ty: {} }};\n    let n = rpgrt::native::set_elems(rt.mem(), &arr, cur, &v)?;\n    rt.set_elem_count({nm}, n);\n    Ok(())\n}}",
                ty(t),
                nm = json_str(name)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("se{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::ExecSqlDyn {
                sqlca,
                kind,
                name,
                text,
                inputs,
            },
        ) => {
            // exec_sql.rs sql_statement's PREPARE, EXECUTE IMMEDIATE and
            // EXECUTE: the statement's text (statement_text), the USING
            // literals (using_values: host_literal per host variable),
            // then the runtime; then the SQLCA.
            let mut body = String::new();
            match text {
                Some(t) => {
                    let _ = writeln!(body, "    let text = Some({});", value(t));
                }
                None => body.push_str("    let text = None;\n"),
            }
            sql_inputs(&mut body, inputs);
            let k = match kind {
                ir::SqlDyn::Prepare => "Prepare",
                ir::SqlDyn::Immediate => "Immediate",
                ir::SqlDyn::Execute => "Execute",
            };
            let _ = writeln!(
                body,
                "    let o = rt.exec_sql_dyn({pc}, rpgrt::native::SqlDyn::{k}, {}, text, lits)?;",
                json_str(name)
            );
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn sd{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sd{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::SqlOpen {
                sqlca,
                name,
                pieces,
                inputs,
                prepared,
                using,
            },
        ) => {
            // exec_sql.rs sql_open: the declared query's host literals
            // (bind_host) or the USING literals (using_values), then the
            // runtime; then the SQLCA.
            let mut body = String::new();
            let qm = if inputs.is_empty() { "" } else { "mut " };
            let _ = writeln!(body, "    let {qm}qlits: Vec<String> = Vec::new();");
            for i in inputs {
                let null = match &i.ind {
                    Some(ind) => format!("{}.is_negative()", number(ind)),
                    None => "false".to_string(),
                };
                let cl = match i.char_len {
                    Some(n) => format!("Some({n})"),
                    None => "None".to_string(),
                };
                let _ = writeln!(
                    body,
                    "    if {null} {{\n        qlits.push(\"NULL\".to_string());\n    }} else {{\n        let v = {};\n        qlits.push(rpgrt::native::sql_literal(&v, {cl})?);\n    }}",
                    value(&i.value)
                );
            }
            sql_inputs(&mut body, using);
            let pcs: Vec<String> = pieces.iter().map(|p| json_str(p)).collect();
            let prep = match prepared {
                Some(p) => format!("Some({})", json_str(p)),
                None => "None".to_string(),
            };
            let _ = writeln!(
                body,
                "    let o = rt.sql_open({pc}, {}, &[{}], qlits, {prep}, lits)?;",
                json_str(name),
                pcs.join(", ")
            );
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn so{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("so{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::SqlFetch {
                sqlca,
                name,
                orient,
                n,
                outputs,
            },
        ) => {
            // exec_sql.rs sql_fetch: the INTO targets' storage, then the
            // runtime (the cursor's position, the row, assign_hosts); then
            // the SQLCA.
            let mut body = String::new();
            let into = outputs.is_some();
            let outs = outputs.as_deref().unwrap_or(&[]);
            let om = if outs.is_empty() { "" } else { "mut " };
            let _ = writeln!(
                body,
                "    let {om}outs: Vec<rpgrt::native::SqlSlot> = Vec::new();"
            );
            for o in outs {
                let ir::Operand::Var { ty: t, .. } = &o.target else {
                    return Err(bad());
                };
                let at = scalar_place(&o.target).ok_or_else(bad)?;
                let ind = match &o.ind {
                    Some(ind) => {
                        let ir::Operand::Var { ty: it, .. } = ind else {
                            return Err(bad());
                        };
                        let iat = scalar_place(ind).ok_or_else(bad)?;
                        format!("{{ let (b, o) = {iat}; Some((b, o, {})) }}", ty(it))
                    }
                    None => "None".to_string(),
                };
                let _ = writeln!(
                    body,
                    "    {{\n        let (buf, off) = {at};\n        let ind = {ind};\n        outs.push(rpgrt::native::SqlSlot {{ buf, off, ty: {}, numeric: {}, ind }});\n    }}",
                    ty(t),
                    o.numeric
                );
            }
            let _ = writeln!(
                body,
                "    let o = rt.sql_fetch({pc}, {}, {}, {n}, outs, {into})?;",
                json_str(name),
                json_str(orient)
            );
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn sf{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sf{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::SqlPositioned {
                sqlca,
                cursor,
                table,
                pieces,
                inputs,
            },
        ) => {
            // exec_sql.rs positioned_change: the host literals of the text
            // before WHERE (bind_host), then the runtime; then the SQLCA.
            let mut body = String::new();
            let qm = if inputs.is_empty() { "" } else { "mut " };
            let _ = writeln!(body, "    let {qm}qlits: Vec<String> = Vec::new();");
            for i in inputs {
                let null = match &i.ind {
                    Some(ind) => format!("{}.is_negative()", number(ind)),
                    None => "false".to_string(),
                };
                let cl = match i.char_len {
                    Some(n) => format!("Some({n})"),
                    None => "None".to_string(),
                };
                let _ = writeln!(
                    body,
                    "    if {null} {{\n        qlits.push(\"NULL\".to_string());\n    }} else {{\n        let v = {};\n        qlits.push(rpgrt::native::sql_literal(&v, {cl})?);\n    }}",
                    value(&i.value)
                );
            }
            let pcs: Vec<String> = pieces.iter().map(|p| json_str(p)).collect();
            let _ = writeln!(
                body,
                "    let o = rt.sql_positioned({pc}, {}, {}, &[{}], qlits)?;",
                json_str(cursor),
                json_str(table),
                pcs.join(", ")
            );
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn sp{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sp{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::SqlFetchRows {
                sqlca,
                name,
                orient,
                n,
                count,
                dim,
                host,
                fields,
            },
        ) => {
            // exec_sql.rs fetch_rows: the row count, the subfields' places,
            // then the runtime; then the SQLCA.
            let mut body = sql_count(count);
            sql_row_fields(&mut body, fields);
            let _ = writeln!(
                body,
                "    let o = rt.sql_fetch_rows({pc}, {}, {}, {n}, count, {dim}, {}, fields)?;",
                json_str(name),
                json_str(orient),
                json_str(host)
            );
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn sr{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sr{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::SqlInsertRows {
                sqlca,
                target,
                count,
                dim,
                host,
                fields,
            },
        ) => {
            // exec_sql.rs sql_insert_rows: the row count, the subfields'
            // places, then the runtime; then the SQLCA.
            let mut body = sql_count(count);
            sql_row_fields(&mut body, fields);
            let _ = writeln!(
                body,
                "    let o = rt.sql_insert_rows({pc}, {}, count, {dim}, {}, fields)?;",
                json_str(target),
                json_str(host)
            );
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn si{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("si{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::SqlCall { sqlca, name, args }) => {
            // exec_sql.rs sql_call: each argument's expression (a host
            // variable as host_literal makes it), and where an OUT or INOUT
            // value goes; then the runtime; then the SQLCA.
            let mut body = String::new();
            let em = if args.is_empty() { "" } else { "mut " };
            let _ = writeln!(body, "    let {em}exprs: Vec<String> = Vec::new();");
            let _ = writeln!(
                body,
                "    let {em}outs: Vec<Option<rpgrt::native::SqlSlot>> = Vec::new();"
            );
            for a in args {
                match a {
                    ir::SqlArg::Text(t) => {
                        let _ = writeln!(
                            body,
                            "    exprs.push({}.to_string());\n    outs.push(None);",
                            json_str(t)
                        );
                    }
                    ir::SqlArg::Host { kw, input, out } => {
                        let null = match &input.ind {
                            Some(ind) => format!("{}.is_negative()", number(ind)),
                            None => "false".to_string(),
                        };
                        let cl = match input.char_len {
                            Some(n) => format!("Some({n})"),
                            None => "None".to_string(),
                        };
                        let _ = writeln!(
                            body,
                            "    if {null} {{\n        exprs.push(format!(\"{{}}NULL\", {}));\n    }} else {{\n        let v = {};\n        exprs.push(format!(\"{{}}{{}}\", {}, rpgrt::native::sql_literal(&v, {cl})?));\n    }}",
                            json_str(kw),
                            value(&input.value),
                            json_str(kw)
                        );
                        match out {
                            None => body.push_str("    outs.push(None);\n"),
                            Some(o) => {
                                let ir::Operand::Var { ty: t, .. } = &o.target else {
                                    return Err(bad());
                                };
                                let at = scalar_place(&o.target).ok_or_else(bad)?;
                                let ind = match &o.ind {
                                    Some(ind) => {
                                        let ir::Operand::Var { ty: it, .. } = ind else {
                                            return Err(bad());
                                        };
                                        let iat = scalar_place(ind).ok_or_else(bad)?;
                                        format!(
                                            "{{ let (b, o) = {iat}; Some((b, o, {})) }}",
                                            ty(it)
                                        )
                                    }
                                    None => "None".to_string(),
                                };
                                let _ = writeln!(
                                    body,
                                    "    {{\n        let (buf, off) = {at};\n        let ind = {ind};\n        outs.push(Some(rpgrt::native::SqlSlot {{ buf, off, ty: {}, numeric: {}, ind }}));\n    }}",
                                    ty(t),
                                    o.numeric
                                );
                            }
                        }
                    }
                }
            }
            let _ = writeln!(
                body,
                "    let o = rt.sql_call({pc}, {}, exprs, outs)?;",
                json_str(name)
            );
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn sk{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sk{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::Sorta {
                arr: (b, o, st, d, t, lead),
                descend,
            },
        ) => guarded(
            format!("rt.stmt({pc}).map(|()| {next})"),
            format!(
                "rt.sorta({pc}, rpgrt::native::ArrPlace {{ buf: {b}, off: {o}, stride: {st}, dim: {d}, ty: {}, lead: {lead} }}, {descend}).map(|()| {next})",
                ty(t)
            ),
        ),
        (ir::Kind::Stmt, ir::Lowered::Movea { from, to, pad }) => {
            // ops.rs movea: the sides' places, then the runtime.
            let side = |m: &ir::MoveaOp| -> Result<String, Error> {
                Ok(match m {
                    ir::MoveaOp::Array {
                        arr: (b, o, st, d, t),
                        index,
                    } => format!(
                        "rpgrt::native::MoveaSide::Array {{ arr: rpgrt::native::ArrPlace {{ buf: {b}, off: {o}, stride: {st}, dim: {d}, ty: {}, lead: 0 }}, index: {} }}",
                        ty(t),
                        match index {
                            Some(i) => {
                                let ir::Operand::Var { ty: it, .. } = i else {
                                    return Err(bad());
                                };
                                let at = scalar_place(i).ok_or_else(bad)?;
                                format!(
                                    "{{ let (buf, off) = {at}; Some(rpgrt::native::OutSlot {{ name: String::new(), buf, off, ty: {} }}) }}",
                                    ty(it)
                                )
                            }
                            None => "None".to_string(),
                        }
                    ),
                    ir::MoveaOp::Bytes(b) => format!("rpgrt::native::MoveaSide::Bytes(vec!{b:?})"),
                    ir::MoveaOp::Field(f) => {
                        let ir::Operand::Var { ty: ft, .. } = f else {
                            return Err(bad());
                        };
                        let at = scalar_place(f).ok_or_else(bad)?;
                        format!(
                            "{{ let (buf, off) = {at}; rpgrt::native::MoveaSide::Field(rpgrt::native::OutSlot {{ name: String::new(), buf, off, ty: {} }}) }}",
                            ty(ft)
                        )
                    }
                })
            };
            let mut body = String::new();
            let _ = writeln!(
                body,
                "    let from = {};\n    let to = {};\n    rt.movea({pc}, from, to, {pad})?;",
                side(from)?,
                side(to)?
            );
            let _ = writeln!(
                helpers,
                "\nfn mv{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("mv{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::ArrOp(d)) => {
            // ops.rs lookup and XFOOT: the argument, the places, then the
            // runtime.
            let ir::ArrOpData {
                lookup,
                xfoot,
                arr,
                stride,
                dim,
                hi,
                lo,
                eq,
            } = d.as_ref();
            let name = |n: &Option<String>| match n {
                Some(n) => format!("Some({}.to_string())", json_str(n)),
                None => "None".to_string(),
            };
            let slot = |o: &ir::Operand| -> Result<String, Error> {
                let ir::Operand::Var { ty: t, .. } = o else {
                    return Err(bad());
                };
                let at = scalar_place(o).ok_or_else(bad)?;
                Ok(format!(
                    "{{ let (buf, off) = {at}; rpgrt::native::OutSlot {{ name: String::new(), buf, off, ty: {} }} }}",
                    ty(t)
                ))
            };
            let ir::Operand::Var {
                ty: at, buf, off, ..
            } = arr
            else {
                return Err(bad());
            };
            let lk = match lookup {
                Some((arg, ix)) => format!(
                    "Some(({}, {}))",
                    value(arg),
                    match ix {
                        Some(i) => format!("Some({})", slot(i)?),
                        None => "None".to_string(),
                    }
                ),
                None => "None".to_string(),
            };
            let xf = match xfoot {
                Some((r, h)) => format!("Some(({}, {h}))", slot(r)?),
                None => "None".to_string(),
            };
            let mut body = String::new();
            let _ = writeln!(
                body,
                "    let op = rpgrt::native::ArrOpIn {{\n        lookup: {lk},\n        xfoot: {xf},\n        arr: ({buf}, {off}, {stride}, {dim}, {}),\n        hi: {},\n        lo: {},\n        eq: {},\n    }};\n    rt.array_op({pc}, op)?;",
                ty(at),
                name(hi),
                name(lo),
                name(eq)
            );
            let _ = writeln!(
                helpers,
                "\nfn ao{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ao{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::BitOp(d)) => {
            // ops.rs bit_op: the result's place, the bits, then the runtime.
            let ir::BitOpData {
                code,
                target,
                mask,
                mask_field,
                hi,
                lo,
                eq,
            } = d.as_ref();
            let name = |n: &Option<String>| match n {
                Some(n) => format!("Some({}.to_string())", json_str(n)),
                None => "None".to_string(),
            };
            let ir::Operand::Var { ty: t, .. } = target else {
                return Err(bad());
            };
            let at = scalar_place(target).ok_or_else(bad)?;
            let m = match mask {
                Some(m) => format!("Some({m})"),
                None => "None".to_string(),
            };
            let mb = match mask_field {
                Some(o) => format!("Some({}.chars()?)", value(o)),
                None => "None".to_string(),
            };
            let mut body = String::new();
            let _ = writeln!(
                body,
                "    let (buf, off) = {at};\n    let op = rpgrt::native::BitOpIn {{\n        code: rpgrt::native::BitCode::{code:?},\n        slot: rpgrt::native::OutSlot {{ name: String::new(), buf, off, ty: {} }},\n        mask: {m},\n        mask_bytes: {mb},\n        hi: {},\n        lo: {},\n        eq: {},\n    }};\n    rt.bit_op({pc}, op)?;",
                ty(t),
                name(hi),
                name(lo),
                name(eq)
            );
            let _ = writeln!(
                helpers,
                "\nfn bo{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("bo{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::StrOp(d)) => {
            let ir::StrOpData {
                code,
                f1,
                f1_chars,
                f1_num,
                f2,
                f2_num,
                pad,
                lo,
                eq,
                result,
            } = d.as_ref();
            // ops.rs string_op: the factors' values, the result's place,
            // then the runtime.
            let chars = |o: &Option<ir::Operand>| match o {
                Some(o) => format!("Some({}.chars()?)", value(o)),
                None => "None".to_string(),
            };
            let num = |o: &Option<ir::Operand>| match o {
                Some(o) => format!("Some({})", number(o)),
                None => "None".to_string(),
            };
            let name = |n: &Option<String>| match n {
                Some(n) => format!("Some({}.to_string())", json_str(n)),
                None => "None".to_string(),
            };
            let slot = |o: &ir::Operand| -> Result<String, Error> {
                let ir::Operand::Var { ty: t, .. } = o else {
                    return Err(bad());
                };
                let at = scalar_place(o).ok_or_else(bad)?;
                Ok(format!(
                    "{{ let (buf, off) = {at}; rpgrt::native::OutSlot {{ name: String::new(), buf, off, ty: {} }} }}",
                    ty(t)
                ))
            };
            let res = match result {
                ir::StrRes::None => "rpgrt::native::StrOut::None".to_string(),
                ir::StrRes::Positions { target, array } => format!(
                    "rpgrt::native::StrOut::Positions {{ slot: {}, array: {} }}",
                    slot(target)?,
                    match array {
                        Some((st, d)) => format!("Some(({st}, {d}))"),
                        None => "None".to_string(),
                    }
                ),
                ir::StrRes::Chars(o) => format!("rpgrt::native::StrOut::Chars({})", slot(o)?),
            };
            let c = format!("{code:?}");
            let mut body = String::new();
            let _ = writeln!(
                body,
                "    let op = rpgrt::native::StrOpIn {{\n        code: rpgrt::native::StrCode::{c},\n        f1: {},\n        f1_chars: {},\n        f1_num: {},\n        f2: {}.chars()?,\n        f2_num: {},\n        pad: {pad},\n        lo: {},\n        eq: {},\n    }};\n    let result = {res};\n    rt.str_op({pc}, op, result)?;",
                chars(f1),
                chars(f1_chars),
                num(f1_num),
                value(f2),
                num(f2_num),
                name(lo),
                name(eq)
            );
            let _ = writeln!(
                helpers,
                "\nfn st{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("st{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::DataArea {
                input,
                lock,
                fields,
            },
        ) => {
            // interp.rs data_area: the fields' places for this call, then
            // the runtime, field by field.
            let slot = |n: &str, o: &ir::Operand| -> Result<String, Error> {
                let ir::Operand::Var { ty: t, .. } = o else {
                    return Err(bad());
                };
                let at = scalar_place(o).ok_or_else(bad)?;
                Ok(format!(
                    "{{ let (buf, off) = {at}; rpgrt::native::OutSlot {{ name: {}.to_string(), buf, off, ty: {} }} }}",
                    json_str(n),
                    ty(t)
                ))
            };
            let mut body = String::new();
            let fm = if fields.is_empty() { "" } else { "mut " };
            let _ = writeln!(
                body,
                "    let {fm}fields: Vec<rpgrt::native::DtaSlot> = Vec::new();"
            );
            for (n, o, v) in fields {
                let var = match v {
                    Some(v) => format!("Some({})", slot("", v)?),
                    None => "None".to_string(),
                };
                let _ = writeln!(
                    body,
                    "    {{\n        let field = {};\n        let var = {var};\n        fields.push(rpgrt::native::DtaSlot {{ field, var }});\n    }}",
                    slot(n, o)?
                );
            }
            let _ = writeln!(body, "    rt.data_area({pc}, {input}, {lock}, fields)?;");
            let _ = writeln!(
                helpers,
                "\nfn da{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("da{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::PrinterOp) => guarded(
            format!("rt.stmt({pc}).map(|()| {next})"),
            format!("rt.printer_op({pc}).map(|()| {next})"),
        ),
        (ir::Kind::Stmt, ir::Lowered::PrinterWrite { fields, indds }) => {
            // errors.rs file_op and printer.rs printer_write: the fields'
            // places for this call, then the runtime.
            let mut body = String::new();
            out_slots(&mut body, fields, &bad)?;
            let ind = match indds {
                Some(o) => {
                    let ir::Operand::Var { ty: t, .. } = o else {
                        return Err(bad());
                    };
                    let at = scalar_place(o).ok_or_else(bad)?;
                    format!(
                        "{{ let (buf, off) = {at}; Some(rpgrt::native::OutSlot {{ name: String::new(), buf, off, ty: {} }}) }}",
                        ty(t)
                    )
                }
                None => "None".to_string(),
            };
            let _ = writeln!(
                body,
                "    let indds = {ind};\n    rt.printer_write({pc}, fields, indds)?;"
            );
            let _ = writeln!(
                helpers,
                "\nfn pw{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("pw{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::Except { name, fields }) => {
            // interp.rs except: the fields' places for this call, then the
            // runtime prints the records.
            let mut body = String::new();
            out_slots(&mut body, fields, &bad)?;
            let nm = match name {
                Some(n) => format!("Some({})", json_str(n)),
                None => "None".to_string(),
            };
            let _ = writeln!(body, "    rt.except({pc}, {nm}, fields)?;");
            let _ = writeln!(
                helpers,
                "\nfn ex{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ex{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::SqlClose { sqlca, name }) => {
            let mut body = format!("    let o = rt.sql_close({pc}, {})?;\n", json_str(name));
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn sc{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sc{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::SqlDiag {
                sqlca,
                condition,
                items,
            },
        ) => {
            // exec_sql.rs get_diagnostics: each item's value about the last
            // statement, assigned to its fields (host_targets) with the
            // truncating fit; then the SQLCA (the outcome is not kept as
            // the last).
            let names: Vec<String> = items.iter().map(|(n, _)| json_str(n)).collect();
            let mut body = format!(
                "    let (vals, o) = rt.sql_diagnostics({pc}, {condition}, &[{}])?;\n",
                names.join(", ")
            );
            for (k, (_, targets)) in items.iter().enumerate() {
                if targets.is_empty() {
                    continue;
                }
                // `first()` for the first (clippy::get_first in generated
                // code).
                let get = if k == 0 {
                    "first()".to_string()
                } else {
                    format!("get({k})")
                };
                let _ = writeln!(body, "    if let Some(v) = vals.{get} {{");
                for target in targets {
                    let ir::Operand::Var { ty: t, .. } = target else {
                        return Err(bad());
                    };
                    let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
                    let _ = writeln!(
                        body,
                        "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        v,\n        rpgrt::rpg::data::Fit::Truncate {{ half_adjust: false }},\n        false,\n    )?;",
                        ty(t)
                    );
                }
                body.push_str("    }\n");
            }
            sqlca_store(&mut body, sqlca, &bad)?;
            let _ = writeln!(
                helpers,
                "\nfn sg{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sg{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::ExecSqlHosts {
                sqlca,
                pieces,
                inputs,
                outputs,
                into,
            },
        ) => {
            // bind_host: each input's indicator, then its value, as a
            // literal (host_literal); the INTO targets' places; the runtime
            // runs the statement and assigns the row (assign_hosts); then
            // the SQLCA as for ExecSql.
            // `mut` only when something is pushed (unused_mut in generated
            // code).
            let lm = if inputs.is_empty() { "" } else { "mut " };
            let mut body = format!("    let {lm}lits: Vec<String> = Vec::new();\n");
            for i in inputs {
                let null = match &i.ind {
                    Some(ind) => format!("{}.is_negative()", number(ind)),
                    None => "false".to_string(),
                };
                let cl = match i.char_len {
                    Some(n) => format!("Some({n})"),
                    None => "None".to_string(),
                };
                let _ = writeln!(
                    body,
                    "    if {null} {{\n        lits.push(\"NULL\".to_string());\n    }} else {{\n        let v = {};\n        lits.push(rpgrt::native::sql_literal(&v, {cl})?);\n    }}",
                    value(&i.value)
                );
            }
            let om = if outputs.is_empty() { "" } else { "mut " };
            let _ = writeln!(
                body,
                "    let {om}outs: Vec<rpgrt::native::SqlSlot> = Vec::new();"
            );
            for o in outputs {
                let ir::Operand::Var { ty: t, .. } = &o.target else {
                    return Err(bad());
                };
                let at = scalar_place(&o.target).ok_or_else(bad)?;
                let ind = match &o.ind {
                    Some(ind) => {
                        let ir::Operand::Var { ty: it, .. } = ind else {
                            return Err(bad());
                        };
                        let iat = scalar_place(ind).ok_or_else(bad)?;
                        format!("{{ let (b, o) = {iat}; Some((b, o, {})) }}", ty(it))
                    }
                    None => "None".to_string(),
                };
                let _ = writeln!(
                    body,
                    "    {{\n        let (buf, off) = {at};\n        let ind = {ind};\n        outs.push(rpgrt::native::SqlSlot {{ buf, off, ty: {}, numeric: {}, ind }});\n    }}",
                    ty(t),
                    o.numeric
                );
            }
            let pcs: Vec<String> = pieces.iter().map(|p| json_str(p)).collect();
            let _ = writeln!(
                body,
                "    let o = rt.exec_sql_hosts({pc}, &[{}], lits, outs, {into})?;",
                pcs.join(", ")
            );
            let values = [
                "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.code))",
                "rpgrt::rpg::data::Value::str(&o.state)",
                "rpgrt::rpg::data::Value::str(&o.text)",
                "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.rows))",
                "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.errd5))",
            ];
            for (target, v) in sqlca.iter().zip(values) {
                let ir::Operand::Var { ty: t, .. } = target else {
                    return Err(bad());
                };
                let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
                let _ = writeln!(
                    body,
                    "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &{v},\n        rpgrt::rpg::data::Fit::Truncate {{ half_adjust: false }},\n        false,\n    )?;",
                    ty(t)
                );
            }
            let _ = writeln!(
                helpers,
                "\nfn sh{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sh{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::ExecSql { sqlca }) => {
            // exec_sql.rs exec_sql: the outcome, then each SQLCA field
            // assigned in its order with the truncating fit.
            let values = [
                "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.code))",
                "rpgrt::rpg::data::Value::str(&o.state)",
                "rpgrt::rpg::data::Value::str(&o.text)",
                "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.rows))",
                "rpgrt::rpg::data::Value::Num(rpgrt::rpg::dec::Dec::from_i64(o.errd5))",
            ];
            let mut body = format!("    let o = rt.exec_sql({pc})?;\n");
            for (target, v) in sqlca.iter().zip(values) {
                let ir::Operand::Var { ty: t, .. } = target else {
                    return Err(bad());
                };
                let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
                let _ = writeln!(
                    body,
                    "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &{v},\n        rpgrt::rpg::data::Fit::Truncate {{ half_adjust: false }},\n        false,\n    )?;",
                    ty(t)
                );
            }
            let _ = writeln!(
                helpers,
                "\nfn sq{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("sq{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::SetOccur { buf, to, result }) => {
            // ops.rs set_occurrence (EVAL %OCCUR(ds) = n, OCCUR's factor
            // 1) through Runtime::set_occurrence, then OCCUR's result
            // field gets the current occurrence (strict fit).
            let mut body = String::new();
            if let Some(n) = to {
                let v = num_expr(pc, n, helpers);
                let _ = writeln!(body, "    let n = {v};\n    rt.set_occurrence({buf}, &n)?;");
            }
            if let Some(r) = result {
                let ir::Operand::Var { ty: t, .. } = r else {
                    return Err(bad());
                };
                let (prelude, sb, so) = place_of(r).ok_or_else(bad)?;
                let _ = writeln!(
                    body,
                    "    let cur = rt.occurrence({buf})?;\n{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &rpgrt::rpg::data::Value::Num(cur),\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: false }},\n        false,\n    )?;",
                    ty(t)
                );
            }
            let _ = writeln!(
                helpers,
                "\nfn oc{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{body}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("oc{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::Dealloc { target, null }) => {
            // ptrs.rs storage_op's DEALLOC: the pointer, the storage freed
            // (Runtime::dealloc), then with (N) *NULL assigned.
            let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
            let clear = if *null {
                format!(
                    "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        rpgrt::rpg::data::DType::Pointer,\n        &rpgrt::rpg::data::Value::Fig(rpgrt::rpg::data::Fig::Null),\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: false }},\n        false,\n    )?;\n"
                )
            } else {
                String::new()
            };
            let _ = writeln!(
                helpers,
                "\nfn dl{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n    let p = {};\n    rt.dealloc(&p)?;\n{clear}    Ok(())\n}}",
                value(target)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("dl{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Stmt, ir::Lowered::EvalPtr { target, value: pv }) => {
            // Stmt::Eval of a pointer: the value, then assign (data::store
            // of a pointer). %ADDR is the interpreter's ptr_to of the
            // field's place (Runtime::addr_of).
            let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
            // A field with no index: the storage it now addresses gets
            // room for the variables based on it (assign's room_for_based).
            let room = match target {
                ir::Operand::Var {
                    name, index: None, ..
                } => format!("    rt.pointer_set({}, &v);\n", json_str(name)),
                _ => String::new(),
            };
            let v = format!(
                "    let v = {};\n",
                ptr_val(pc, pv, helpers).ok_or_else(bad)?
            );
            let _ = writeln!(
                helpers,
                "\nfn ep{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n{v}{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        rpgrt::rpg::data::DType::Pointer,\n        &v,\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: false }},\n        false,\n    )?;\n{room}    Ok(())\n}}"
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ep{pc}(rt).map(|()| {next})"),
            )
        }
        (ir::Kind::Return, ir::Lowered::Return(v)) => {
            // The interpreter's RETURN: the value evaluated, then the
            // routine ends with it (ReturnSignal, which settle turns into
            // Flow::Return); the routine returns it here directly.
            let val = match v {
                None => "None".to_string(),
                Some(ir::ValExpr::Char(e)) => format!("Some({})", char_expr(pc, e, helpers)),
                Some(ir::ValExpr::Num(e)) => format!(
                    "Some(rpgrt::rpg::data::Value::Num({}))",
                    num_expr(pc, e, helpers)
                ),
            };
            let _ = writeln!(
                helpers,
                "\nfn rv{pc}(rt: &mut dyn Runtime) -> NResult<Option<rpgrt::rpg::data::Value>> {{\n    Ok({val})\n}}"
            );
            format!(
                "match rv{pc}(rt) {{\n                    Ok(v) => return Ok(Flow::Return(v)),\n                    Err(e) => Err(e),\n                }}"
            )
        }
        (ir::Kind::Stmt, ir::Lowered::Call(c)) => {
            // The helper's result, not `?`: an error goes to `settle`.
            let call = call_fn(pc, c, helpers);
            let call = call.strip_suffix('?').unwrap_or(&call);
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("{call}.map(|_| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::EvalFig {
                target,
                fig: f,
                half_adjust,
                right,
            },
        ) => {
            let ir::Operand::Var { ty: t, .. } = target else {
                return Err(bad());
            };
            let (prelude, sb, so) = place_of(target).ok_or_else(bad)?;
            let _ = writeln!(
                helpers,
                "\nfn ef{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n    let v = {};",
                fig(f)
            );
            let _ = writeln!(
                helpers,
                "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &v,\n        rpgrt::rpg::data::Fit::Strict {{ half_adjust: {half_adjust} }},\n        {right},\n    )\n}}",
                ty(t)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ef{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::Move {
                left,
                pad,
                from,
                to,
            },
        ) => {
            let ir::Operand::Var { ty: t, .. } = to else {
                return Err(bad());
            };
            let (prelude, sb, so) = place_of(to).ok_or_else(bad)?;
            // A literal has no type (the interpreter's `type_of`).
            let from_ty = match from {
                ir::Operand::Var { ty: f, .. } => format!("Some({})", ty(f)),
                _ => "None".to_string(),
            };
            let _ = writeln!(
                helpers,
                "\nfn mv{pc}(rt: &mut dyn Runtime) -> NResult<()> {{\n    let a = {};\n    let src = rpgrt::native::move_image(&a, {from_ty})?;\n    let b = {};\n    let dst = rpgrt::native::move_image(&b, Some({}))?;\n    let v = rpgrt::native::move_into({left}, {pad}, &src, dst, {})?;",
                value(from),
                value(to),
                ty(t),
                ty(t)
            );
            let _ = writeln!(
                helpers,
                "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &v,\n        rpgrt::rpg::data::Fit::Truncate {{ half_adjust: false }},\n        false,\n    )\n}}",
                ty(t)
            );
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("mv{pc}(rt).map(|()| {next})"),
            )
        }
        (
            ir::Kind::Stmt,
            ir::Lowered::Arith {
                op,
                f1,
                f2,
                result,
                half_adjust,
                inds,
            },
        ) => {
            let ir::Operand::Var { ty: t, .. } = result else {
                return Err(bad());
            };
            let (prelude, sb, so) = place_of(result).ok_or_else(bad)?;
            let _ = writeln!(
                helpers,
                "\nfn ar{pc}(rt: &mut dyn Runtime) -> NResult<()> {{"
            );
            let value = match op {
                ir::FixOp::ZAdd => number(f2),
                ir::FixOp::ZSub => format!("{}.neg()", number(f2)),
                ir::FixOp::Mvr => "rt.remainder()".to_string(),
                ir::FixOp::Div { decs } => {
                    let a1 = number(f1.as_ref().unwrap_or(result));
                    let _ = writeln!(helpers, "    let a1 = {a1};");
                    let _ = writeln!(helpers, "    let a2 = {};", number(f2));
                    let with_h = if *half_adjust {
                        decs.saturating_add(1)
                    } else {
                        *decs
                    };
                    let _ = writeln!(helpers, "    let q = a1.div_scale(&a2, {with_h})?;");
                    let _ = writeln!(helpers, "    let qt = q.rescale({decs}, false)?;");
                    let _ = writeln!(helpers, "    rt.set_remainder(a1.sub(&qt.mul(&a2)?)?);");
                    "q".to_string()
                }
                ir::FixOp::Add | ir::FixOp::Sub | ir::FixOp::Mult => {
                    let a1 = number(f1.as_ref().unwrap_or(result));
                    let _ = writeln!(helpers, "    let a1 = {a1};");
                    let _ = writeln!(helpers, "    let a2 = {};", number(f2));
                    let f = match op {
                        ir::FixOp::Add => "add",
                        ir::FixOp::Sub => "sub",
                        _ => "mul",
                    };
                    format!("a1.{f}(&a2)?")
                }
            };
            let _ = writeln!(helpers, "    let v = {value};");
            let _ = writeln!(
                helpers,
                "{prelude}    rpgrt::native::store(\n        rt.mem(),\n        {sb},\n        {so},\n        {},\n        &rpgrt::rpg::data::Value::Num(v),\n        rpgrt::rpg::data::Fit::Truncate {{ half_adjust: {half_adjust} }},\n        false,\n    )?;",
                ty(t)
            );
            if *inds {
                let _ = writeln!(
                    helpers,
                    "    let stored = rpgrt::native::load_num(rt.mem(), {sb}, {so}, {})?;\n    rt.num_inds({pc}, &stored);",
                    ty(t)
                );
            } else {
                // As the interpreter: the result is read back.
                let _ = writeln!(
                    helpers,
                    "    let _stored = rpgrt::native::load_num(rt.mem(), {sb}, {so}, {})?;",
                    ty(t)
                );
            }
            let _ = writeln!(helpers, "    Ok(())\n}}");
            guarded(
                format!("rt.stmt({pc}).map(|()| {next})"),
                format!("ar{pc}(rt).map(|()| {next})"),
            )
        }
        _ => return Err(bad()),
    })
}

/// A helper's name for a place in a statement's expression: l, r and x
/// for the first, second and third operand on the way from the root.
/// The helpers with their runtime parameter named `_rt` where the body
/// does not use it (no unused-variable warning in generated code).
fn unused_rt(helpers: &str) -> String {
    const SIG: &str = "(rt: &mut dyn Runtime)";
    let mut out = String::with_capacity(helpers.len());
    for (k, chunk) in helpers.split("\nfn ").enumerate() {
        if k > 0 {
            out.push_str("\nfn ");
        }
        let body = chunk.split_once(SIG).map(|(_, b)| b);
        match body {
            Some(b) if !uses_ident(b, "rt") => {
                out.push_str(&chunk.replacen(SIG, "(_rt: &mut dyn Runtime)", 1))
            }
            _ => out.push_str(chunk),
        }
    }
    out
}

/// Whether Rust source text uses the identifier `id`. String literals
/// and comments are not skipped; generated helpers put no `rt` in either
/// (RPG names are upper case).
fn uses_ident(src: &str, id: &str) -> bool {
    src.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .any(|w| w == id)
}

/// A native call (S2.35): a helper that evaluates the arguments in
/// parameter order and calls the procedure through the runtime; the
/// expression that runs it.
fn call_fn(pc: usize, c: &ir::Call, helpers: &mut String) -> String {
    let name = format!("cl{pc}_{}", path_id(&c.path));
    // Each argument into its own local, in parameter order, then the
    // vector of them (no push after creation).
    let mut body = String::new();
    let mut names = Vec::new();
    for (k, a) in c.args.iter().enumerate() {
        let an = format!("a{k}");
        names.push(an.clone());
        match a {
            ir::CallArg::Ref(o) => match (place_of(o), o) {
                // The argument's own type goes with it (S2.51: loc_of).
                (Some((pre, b, off)), ir::Operand::Var { ty: t, .. }) => {
                    let _ = write!(
                        body,
                        "    let {an} = {{\n{pre}    rpgrt::native::CallArg::Ref {{ buf: {b}, off: {off}, dtype: {} }}\n    }};\n",
                        ty(t)
                    );
                }
                // Lowering passes only variables by reference.
                _ => {
                    let _ = writeln!(
                        body,
                        "    return Err(anyhow::anyhow!(\"native: argument of {} is not a variable\"));",
                        c.proc
                    );
                }
            },
            ir::CallArg::Num(e) => {
                let x = num_expr(pc, e, helpers);
                let _ = writeln!(
                    body,
                    "    let {an} = rpgrt::native::CallArg::Val(rpgrt::rpg::data::Value::Num({x}));"
                );
            }
            ir::CallArg::Char(e) => {
                let x = char_expr(pc, e, helpers);
                let _ = writeln!(body, "    let {an} = rpgrt::native::CallArg::Val({x});");
            }
            ir::CallArg::Ptr(pv) => match ptr_val(pc, pv, helpers) {
                Some(x) => {
                    let _ = writeln!(body, "    let {an} = rpgrt::native::CallArg::Val({x});");
                }
                None => {
                    let _ = writeln!(
                        body,
                        "    return Err(anyhow::anyhow!(\"native: a pointer argument of {} has no place\"));",
                        c.proc
                    );
                }
            },
        }
    }
    let _ = writeln!(
        helpers,
        "\nfn {name}(rt: &mut dyn Runtime) -> NResult<Option<rpgrt::rpg::data::Value>> {{\n{body}    let args = vec![{}];\n    rt.call({pc}, {}, args)\n}}",
        names.join(", "),
        json_str(&c.proc)
    );
    format!("{name}(rt)?")
}

/// The value a called procedure returned, where the expression needs one.
fn returned(call: String, proc: &str) -> String {
    format!(
        "{call}.ok_or_else(|| anyhow::anyhow!(\"procedure {} returned no value\"))?",
        proc.replace(['\\', '"'], "")
    )
}

fn path_id(path: &[u8]) -> String {
    // Steps 0 and 1 are `l` and `r`; a higher step (a function's third
    // argument on) is `x<k>x`, so that two paths never share a name.
    // A run of more than three equal steps (a chain's left operands) is
    // `z<n>l` or `z<n>r`, so names stay short (and snake case; `z` is in
    // no other step).
    let mut s = String::new();
    let mut i = 0;
    while let Some(&b) = path.get(i) {
        let run = path.iter().skip(i).take_while(|&&x| x == b).count();
        match b {
            0 | 1 if run > 3 => {
                let c = if b == 0 { 'l' } else { 'r' };
                let _ = write!(s, "z{run}{c}");
                i = i.saturating_add(run);
                continue;
            }
            0 => s.push('l'),
            1 => s.push('r'),
            k => {
                let _ = write!(s, "x{k}x");
            }
        }
        i = i.saturating_add(1);
    }
    s
}

/// A path as the argument of `Runtime::precision`: the steps written out,
/// or, when the path is long (a chain's left operands), as runs that
/// `rpgrt::native::path_rle` expands, so the code stays linear in size.
fn path_arg(path: &[u8]) -> String {
    if path.len() <= 16 {
        let v: Vec<String> = path.iter().map(|b| b.to_string()).collect();
        return format!("&[{}]", v.join(", "));
    }
    let mut runs = Vec::new();
    let mut i = 0;
    while let Some(&b) = path.get(i) {
        let run = path.iter().skip(i).take_while(|&&x| x == b).count();
        runs.push(format!("({b}, {run})"));
        i = i.saturating_add(run.max(1));
    }
    format!("&rpgrt::native::path_rle(&[{}])", runs.join(", "))
}

/// A character expression as a `Value` expression; each concatenation is
/// a helper `s<pc>_<path>` that joins its operands' characters as the
/// interpreter does (`chars` of each, the second after the first).
fn char_expr(pc: usize, e: &ir::CharExpr, helpers: &mut String) -> String {
    match e {
        ir::CharExpr::Leaf(o) => value(o),
        ir::CharExpr::LeftRight { left, s, n, path } => {
            let name = format!("lr{pc}_{}", path_id(path));
            let x = char_expr(pc, s, helpers);
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let s = {x}.chars()?;\n    let n = {};\n    rpgrt::native::left_right(&s, &n, {left})\n}}",
                value(n)
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::Ind(name) => format!(
            "rpgrt::rpg::data::Value::Char(vec![if rt.ind({}) {{ 0xF1 }} else {{ 0xF0 }}])",
            json_str(name)
        ),
        ir::CharExpr::Call(c) => returned(call_fn(pc, c, helpers), &c.proc),
        ir::CharExpr::Chain { parts, path } => {
            // Each part evaluated in order and appended (the interpreter's
            // concatenations, left to right).
            let name = format!("sq{pc}_{}", path_id(path));
            let mut body = String::from("    let mut s: Vec<u8> = Vec::new();\n");
            for x in parts {
                let v = char_expr(pc, x, helpers);
                let _ = writeln!(body, "    s.extend({v}.chars()?);");
            }
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n{body}    Ok(rpgrt::rpg::data::Value::Char(s))\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::Cat { left, right, path } => {
            let id = path_id(path);
            let name = format!("s{pc}_{id}");
            let x = char_expr(pc, left, helpers);
            let y = char_expr(pc, right, helpers);
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let x = {x};\n    let y = {y};\n    let mut s = x.chars()?;\n    s.extend(y.chars()?);\n    Ok(rpgrt::rpg::data::Value::Char(s))\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::Subst {
            inner,
            start,
            len,
            path,
        } => {
            let id = path_id(path);
            let name = format!("u{pc}_{id}");
            let x = char_expr(pc, inner, helpers);
            // As the interpreter: the string, then the start (checked),
            // then the length.
            let take = match len {
                Some(l) => format!(
                    "    let n = {};\n    rpgrt::native::subst_take(&s, from, Some(&n))",
                    value(l)
                ),
                None => "    rpgrt::native::subst_take(&s, from, None)".to_string(),
            };
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let s = {x}.chars()?;\n    let st = {};\n    let from = rpgrt::native::subst_start(&s, &st)?;\n{take}\n}}",
                value(start)
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::TemporalAdd {
            base,
            unit,
            n,
            minus,
            dur_first,
        } => {
            let b = char_expr(pc, base, helpers);
            let k = num_expr(pc, n, helpers);
            let lets = if *dur_first {
                format!("let n = {k};\n    let b = {b};")
            } else {
                format!("let b = {b};\n    let n = {k};")
            };
            format!(
                "{{\n    {lets}\n    rpgrt::native::temporal_add(&b, rpgrt::rpg::data::DurUnit::{unit}, &n, {minus})?\n    }}"
            )
        }
        ir::CharExpr::TemporalBif {
            kind,
            arg,
            fmt,
            dft,
        } => {
            let k = match kind {
                ir::TemporalBifKind::Date => "Date",
                ir::TemporalBifKind::Time => "Time",
                ir::TemporalBifKind::Ts => "Ts",
            };
            let v = match arg.as_deref() {
                None => "None".to_string(),
                Some(ir::ValExpr::Num(e)) => format!(
                    "Some(rpgrt::rpg::data::Value::Num({}))",
                    num_expr(pc, e, helpers)
                ),
                Some(ir::ValExpr::Char(e)) => format!("Some({})", char_expr(pc, e, helpers)),
            };
            let f = match fmt {
                Some(f) => format!("Some({f:?})"),
                None => "None".to_string(),
            };
            format!(
                "rpgrt::native::temporal_bif(rpgrt::native::TemporalBif::{k}, {v}, {f}, {dft:?})?"
            )
        }
        ir::CharExpr::TemporalChar { of, fmt } => {
            let f = match fmt {
                ir::TemporalFmt::Date(n, sep) => format!(
                    "rpgrt::native::TemporalFmt::Date(rpgrt::rpg::dates::DateFmt {{ name: {n:?}, sep: {sep:?} }})"
                ),
                ir::TemporalFmt::Time(n, sep) => format!(
                    "rpgrt::native::TemporalFmt::Time(rpgrt::rpg::dates::TimeFmt {{ name: {n:?}, sep: {sep:?} }})"
                ),
                ir::TemporalFmt::Ts { iso0 } => {
                    format!("rpgrt::native::TemporalFmt::Ts {{ iso0: {iso0} }}")
                }
            };
            format!("rpgrt::native::temporal_char(&{}, {f})?", value(of))
        }
        ir::CharExpr::CharOfNum {
            inner,
            point,
            zero,
            path,
        } => {
            let name = format!("h{pc}_{}", path_id(path));
            let x = num_expr(pc, inner, helpers);
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let x = {x};\n    Ok(rpgrt::native::char_of_num(&x, {point:?}, {zero}))\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::EditC {
            inner,
            code,
            point,
            zero,
            own,
            third,
            path,
        } => {
            let name = format!("k{pc}_{}", path_id(path));
            let x = num_expr(pc, inner, helpers);
            let mut ip = path.clone();
            ip.push(0);
            // The precision is asked for only when the value is not of its
            // own digits (a literal or a field): no unused static then.
            let p = if *own {
                "    let p = None;".to_string()
            } else {
                format!(
                    "    static P: std::sync::OnceLock<Option<(u32, u32)>> = std::sync::OnceLock::new();\n    let p = *P.get_or_init(|| rt.precision({pc}, {}));",
                    path_arg(&ip)
                )
            };
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let v = {x};\n{p}\n    {}\n}}",
                match third {
                    None => format!("rpgrt::native::editc(&v, p, {code:?}, {point:?}, {zero})"),
                    Some(t) => format!(
                        "rpgrt::native::editc_third(&rpgrt::native::editc(&v, p, {code:?}, {point:?}, {zero})?, {})",
                        json_str(t)
                    ),
                }
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::EditW {
            inner,
            word,
            prec,
            path,
        } => {
            let name = format!("o{pc}_{}", path_id(path));
            let x = num_expr(pc, inner, helpers);
            let p = match prec {
                Some((d, s)) => format!("Some(({d}, {s}))"),
                None => "None".to_string(),
            };
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let v = {x};\n    rpgrt::native::editw(&v, {p}, {word:?})\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::ScanRpl {
            pat,
            rep,
            s,
            start,
            len,
            path,
        } => {
            let name = format!("v{pc}_{}", path_id(path));
            let a = char_expr(pc, pat, helpers);
            let r = char_expr(pc, rep, helpers);
            let x = char_expr(pc, s, helpers);
            let opt = |o: &Option<ir::Operand>| match o {
                Some(o) => format!("Some({})", value(o)),
                None => "None".to_string(),
            };
            // As the interpreter: the pattern, the replacement, the string,
            // the start, the length.
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let a = {a}.chars()?;\n    let r = {r}.chars()?;\n    let s = {x}.chars()?;\n    let st = {};\n    let n = {};\n    rpgrt::native::scanrpl(&a, &r, &s, st.as_ref(), n.as_ref())\n}}",
                opt(start),
                opt(len)
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::Replace {
            rep,
            s,
            start,
            len,
            path,
        } => {
            let name = format!("w{pc}_{}", path_id(path));
            let r = char_expr(pc, rep, helpers);
            let x = char_expr(pc, s, helpers);
            let opt = |o: &Option<ir::Operand>| match o {
                Some(o) => format!("Some({})", value(o)),
                None => "None".to_string(),
            };
            // As the interpreter: the replacement, the string, the start,
            // the length.
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let r = {r}.chars()?;\n    let s = {x}.chars()?;\n    let st = {};\n    let n = {};\n    rpgrt::native::replace(&r, &s, st.as_ref(), n.as_ref())\n}}",
                opt(start),
                opt(len)
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::Xlate {
            from,
            to,
            s,
            start,
            path,
        } => {
            let name = format!("x{pc}_{}", path_id(path));
            let f = char_expr(pc, from, helpers);
            let o = char_expr(pc, to, helpers);
            let x = char_expr(pc, s, helpers);
            // As the interpreter: from, to, the string, the start.
            let st = match start {
                Some(o) => format!("Some({})", value(o)),
                None => "None".to_string(),
            };
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let f = {f}.chars()?;\n    let o = {o}.chars()?;\n    let s = {x}.chars()?;\n    let st = {st};\n    rpgrt::native::xlate(&f, &o, &s, st.as_ref())\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::CharExpr::Trim {
            inner,
            left,
            right,
            path,
        } => {
            let id = path_id(path);
            let name = format!("t{pc}_{id}");
            let x = char_expr(pc, inner, helpers);
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::data::Value> {{\n    let x = {x};\n    rpgrt::native::trim(&x, {left}, {right})\n}}"
            );
            format!("{name}(rt)?")
        }
    }
}

/// A numeric expression as a `Dec` expression; each operation becomes a
/// helper `e<pc>_<path>` that keeps its precision in a `OnceLock` (the
/// precision depends only on the operands' types, which the fingerprint
/// covers).
fn num_expr(pc: usize, e: &ir::NumExpr, helpers: &mut String) -> String {
    match e {
        ir::NumExpr::Leaf(o) => number(o),
        ir::NumExpr::TemporalDiff { x, y, unit } => {
            let a = char_expr(pc, x, helpers);
            let b = char_expr(pc, y, helpers);
            format!(
                "{{\n    let x = {a};\n    let y = {b};\n    rpgrt::native::temporal_diff(&x, &y, rpgrt::rpg::data::DurUnit::{unit})?\n    }}"
            )
        }
        ir::NumExpr::TemporalSubdt { x, unit } => {
            let a = char_expr(pc, x, helpers);
            format!(
                "{{\n    let x = {a};\n    rpgrt::native::temporal_subdt(&x, rpgrt::rpg::data::DurUnit::{unit})?\n    }}"
            )
        }
        ir::NumExpr::Chain { first, steps, path } => {
            // Straight-line code: the running value, then each operation
            // with its own precision (as each Bin node's helper does).
            let name = format!("eq{pc}_{}", path_id(path));
            let mut body = String::new();
            let a = num_expr(pc, first, helpers);
            let _ = writeln!(body, "    let mut t = {a};");
            for (k, (op, x, p)) in steps.iter().enumerate() {
                let f = match op {
                    ir::ArOp::Add => "add",
                    ir::ArOp::Sub => "sub",
                    ir::ArOp::Mul => "mul",
                    // Lowering puts no division in a chain; were one there,
                    // the generated code would not compile (no silent
                    // multiplication).
                    ir::ArOp::Div => "no_division_in_a_chain",
                };
                let y = num_expr(pc, x, helpers);
                let _ = writeln!(
                    body,
                    "    let y = {y};\n    static P{k}: std::sync::OnceLock<Option<(u32, u32)>> = std::sync::OnceLock::new();\n    let p = *P{k}.get_or_init(|| rt.precision({pc}, {}));\n    t = rpgrt::native::rule_scale(p, t.{f}(&y)?);",
                    path_arg(p)
                );
            }
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{\n{body}    Ok(t)\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::NumExpr::Bin {
            op,
            left,
            right,
            path,
        } => {
            let id = path_id(path);
            let name = format!("e{pc}_{id}");

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
                "    let p = *P.get_or_init(|| rt.precision({pc}, {}));",
                path_arg(path)
            );
            let f = match op {
                ir::ArOp::Add => "add",
                ir::ArOp::Sub => "sub",
                ir::ArOp::Mul => "mul",
                ir::ArOp::Div => "",
            };
            if *op == ir::ArOp::Div {
                // The interpreter's `divide`: each operand's precision (its
                // node's), not the quotient's.
                let mut l = path.clone();
                l.push(0);
                let mut r = path.clone();
                r.push(1);
                let _ = writeln!(
                    body,
                    "    static PL: std::sync::OnceLock<Option<(u32, u32)>> = std::sync::OnceLock::new();\n    let pl = *PL.get_or_init(|| rt.precision({pc}, {}));\n    static PR: std::sync::OnceLock<Option<(u32, u32)>> = std::sync::OnceLock::new();\n    let pr = *PR.get_or_init(|| rt.precision({pc}, {}));\n    let _ = p;\n    rpgrt::native::divide(&a, &b, pl, pr)\n}}",
                    path_arg(&l),
                    path_arg(&r)
                );
            } else {
                let _ = writeln!(body, "    Ok(rpgrt::native::rule_scale(p, a.{f}(&b)?))\n}}");
            }
            helpers.push_str(&body);
            format!("{name}(rt)?")
        }
        ir::NumExpr::DivRem {
            left,
            right,
            rem,
            path,
        } => {
            let name = format!("q{pc}_{}", path_id(path));
            let x = num_expr(pc, left, helpers);
            let y = num_expr(pc, right, helpers);
            let op = if *rem {
                "x.rem(&y)"
            } else {
                "x.div_scale(&y, 0)"
            };
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{\n    let x = {x};\n    let y = {y};\n    {op}\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::NumExpr::MinMax { args, max, path } => {
            let name = format!("mm{pc}_{}", path_id(path));
            let want = if *max { "Greater" } else { "Less" };
            let mut body = String::new();
            for (k, a) in args.iter().enumerate() {
                let x = num_expr(pc, a, helpers);
                if k == 0 {
                    let _ = writeln!(body, "    let mut best = {x};");
                } else {
                    let _ = write!(
                        body,
                        "    let v = {x};\n    if v.cmp(&best) == std::cmp::Ordering::{want} {{\n        best = v;\n    }}\n"
                    );
                }
            }
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{\n{body}    Ok(best)\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::NumExpr::Check { set, s, rev, path } => {
            let name = format!("ck{pc}_{}", path_id(path));
            let a = char_expr(pc, set, helpers);
            let x = char_expr(pc, s, helpers);
            // As the interpreter: the set, then the string.
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{\n    let set = {a}.chars()?;\n    let s = {x}.chars()?;\n    rpgrt::native::check(&set, &s, {rev})\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::NumExpr::Call(c) => format!("{}.num()?", returned(call_fn(pc, c, helpers), &c.proc)),
        ir::NumExpr::Abs(inner) => format!("({}).abs()", num_expr(pc, inner, helpers)),
        ir::NumExpr::Elems(name) => format!(
            "rpgrt::rpg::dec::Dec::from_i64(i64::from(rt.elems({})))",
            json_str(name)
        ),
        ir::NumExpr::Status => "rpgrt::rpg::dec::Dec::from_i64(i64::from(rt.status()))".to_string(),
        ir::NumExpr::VarLen { var } => format!(
            "rpgrt::rpg::dec::Dec::from_i64(i64::try_from({}.chars()?.len())?)",
            value(var)
        ),
        ir::NumExpr::Lookup {
            x,
            buf,
            off,
            stride,
            dim,
            ty: t,
            start,
            n,
            how,
        } => {
            let opt = |o: &Option<Box<ir::Operand>>| match o {
                Some(o) => format!("Some({})", value(o)),
                None => "None".to_string(),
            };
            // As the interpreter: the argument, the array, the start, the
            // number of elements.
            format!(
                "{{ let x = {}; let all = rpgrt::native::lookup_list(rt.mem(), ({buf}, {off}, {stride}, {dim}), {})?; let st = {}; let n = {}; rpgrt::native::search(&x, &all, st.as_ref(), n.as_ref(), {how:?})? }}",
                value(x),
                ty(t),
                opt(start),
                opt(n)
            )
        }
        ir::NumExpr::Int { inner, half, path } => {
            let name = format!("i{pc}_{}", path_id(path));
            let x = num_expr(pc, inner, helpers);
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{\n    let x = {x};\n    x.rescale(0, {half})\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::NumExpr::Dec {
            inner,
            to,
            half,
            path,
        } => {
            let name = format!("d{pc}_{}", path_id(path));
            let x = num_expr(pc, inner, helpers);
            let body = match to {
                Some((p, s)) => format!("rpgrt::native::dec_to(&x, {p}, {s}, {half})"),
                None => "Ok(x)".to_string(),
            };
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{\n    let x = {x};\n    {body}\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::NumExpr::Scan {
            pat,
            s,
            start,
            path,
        } => {
            let name = format!("n{pc}_{}", path_id(path));
            let p = char_expr(pc, pat, helpers);
            let x = char_expr(pc, s, helpers);
            // As the interpreter: the pattern, the string, the start.
            let st = match start {
                Some(o) => format!("Some({})", value(o)),
                None => "None".to_string(),
            };
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{\n    let p = {p}.chars()?;\n    let s = {x}.chars()?;\n    let st = {st};\n    rpgrt::native::scan(&p, &s, st.as_ref())\n}}"
            );
            format!("{name}(rt)?")
        }
        ir::NumExpr::ArrFold {
            kind,
            buf,
            off,
            stride,
            dim,
            ty: t,
        } => format!(
            "{{ let all = rpgrt::native::lookup_list(rt.mem(), ({buf}, {off}, {stride}, {dim}), {})?; rpgrt::native::arr_fold(&all, {kind:?})? }}",
            ty(t)
        ),
        ir::NumExpr::ScanR {
            pat,
            s,
            start,
            len,
            path,
        } => {
            let name = format!("n{pc}_{}", path_id(path));
            let p = char_expr(pc, pat, helpers);
            let x = char_expr(pc, s, helpers);
            // As the interpreter: the pattern, the string, the start, the
            // length.
            let opt = |o: &Option<ir::Operand>| match o {
                Some(o) => format!("Some({})", value(o)),
                None => "None".to_string(),
            };
            let (st, ln) = (opt(start), opt(len));
            let _ = writeln!(
                helpers,
                "\nfn {name}(rt: &mut dyn Runtime) -> NResult<rpgrt::rpg::dec::Dec> {{\n    let p = {p}.chars()?;\n    let s = {x}.chars()?;\n    let st = {st};\n    let ln = {ln};\n    rpgrt::native::scanr(&p, &s, st.as_ref(), ln.as_ref())\n}}"
            );
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
        if let Some(why) = &i.refused {
            return Err(Error::Refused {
                at,
                line: i.line,
                why: why.clone(),
            });
        }
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
            "{{\"pc\":{pc},\"line\":{},\"kind\":{},\"what\":{},\"via\":{},\"rust_line\":{src_line}}}",
            i.line,
            json_str(kind_name(&i.kind)),
            json_str(&i.what),
            json_str(via(i))
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
    src.push_str(&unused_rt(&helpers));
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
            refused: None,
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
            local: false,
            index: None,
            based: None,
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
        // ADR 0007 (S2.37): no interpreter fallback in lowered code.
        for gone in ["native_ok", "rt.loop_past(0)", "rt.truth(1)"] {
            assert!(!g.source.contains(gone), "{gone} in\n{}", g.source);
        }
        for want in [
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
            local: false,
            index: None,
            based: None,
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
        // ADR 0007 (S2.37): no interpreter fallback in lowered code.
        for gone in ["native_ok", "rt.stmt(0)"] {
            assert!(!g.source.contains(gone), "{gone} in\n{}", g.source);
        }
        for want in [
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
    fn lowered_procedure_locals() {
        use super::ir::{Lowered, NumExpr, Operand, Ty};
        let local = |name: &str, buf| Operand::Var {
            name: name.to_string(),
            buf,
            off: 4,
            ty: Ty::Int(10),
            local: true,
            index: None,
            based: None,
        };
        let mut ev = ins(Kind::Stmt, 2, false);
        ev.lowered = Some(Lowered::Eval {
            target: local("R", 1),
            value: NumExpr::Leaf(local("P", 0)),
            half_adjust: false,
            right: false,
        });
        let r = Routine {
            fingerprint: fp(),
            ins: vec![ev],
        };
        let Ok(g) = generate(&r) else {
            return assert!(generate(&r).is_ok());
        };
        // ADR 0007 (S2.37): no interpreter fallback in lowered code.
        assert!(
            !g.source.contains("native_ok"),
            "native_ok in\n{}",
            g.source
        );
        for want in [
            "rt.local(0, 4)?; rpgrt::native::load_num(rt.mem(), b, o,",
            "let (sb, so) = rt.local(1, 4)?;",
            "rt.mem(),\n        sb,\n        so,",
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
                refused: None,
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
