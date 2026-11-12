//! The native engine (docs/native/PLAN.md, ADR 0001-0005): which engine
//! runs RPG (`AS4TAINR_RPG_ENGINE`), the fingerprints of compiled
//! routines, and each program's table of generated routines. The
//! dispatcher that runs them is `interp::native_rt`.

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

pub mod lower;

use super::code::{Code, Routines};
use super::program::Program;
use rpgrt::native::RoutineFn;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, OnceLock};

/// The variable that chooses the engine.
pub const ENGINE_VAR: &str = "AS4TAINR_RPG_ENGINE";

/// Which engine runs RPG programs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    /// The interpreter (the default; upstream's behavior).
    Interp,
    /// Generated code; a program without it does not run.
    Rust,
    /// Generated code when a program has it for every routine, else the
    /// interpreter.
    Auto,
}

impl Engine {
    /// The engine a value of `AS4TAINR_RPG_ENGINE` names.
    pub fn parse(value: Option<&str>) -> Result<Engine, String> {
        match value {
            None | Some("") | Some("interp") => Ok(Engine::Interp),
            Some("rust") => Ok(Engine::Rust),
            Some("auto") => Ok(Engine::Auto),
            Some("native") => Err(format!(
                "{ENGINE_VAR}=native is no longer used (it chose the Rust interpreter over Jariko, \
                 which is gone); use interp, rust or auto"
            )),
            Some(v) => Err(format!(
                "{ENGINE_VAR}={v:?} is not valid; use interp, rust or auto"
            )),
        }
    }
}

thread_local! {
    /// A test's choice of engine for this thread (`with_engine`).
    static OVERRIDE: Cell<Option<Engine>> = const { Cell::new(None) };
    /// Where a test collects what programs left when they ended
    /// (`observe_into`).
    static SINK: std::cell::RefCell<Option<Sink>> = const { std::cell::RefCell::new(None) };
}

/// What a program left when its call ended, for comparing engines
/// (docs/native/adr/0005): its global storage (the PSDS's date and time
/// positions blanked), its indicators, and the engine that ran it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub program: String,
    pub engine: Option<Engine>,
    pub globals: Vec<Vec<u8>>,
    /// The names of the variables and data structures in each global
    /// buffer, sorted (for reporting a difference).
    pub names: Vec<Vec<String>>,
    pub indicators: Vec<(String, bool)>,
}

/// Observations collected by a test.
pub type Sink = Arc<Mutex<Vec<Observation>>>;

/// The most observations a sink keeps.
const MAX_OBSERVATIONS: usize = 10_000;

/// Record an observation in this thread's sink, if a test set one.
pub fn record(o: Observation) {
    SINK.with(|s| {
        if let Ok(slot) = s.try_borrow() {
            if let Some(sink) = slot.as_ref() {
                let mut v = sink.lock().unwrap_or_else(|e| e.into_inner());
                if v.len() < MAX_OBSERVATIONS {
                    v.push(o);
                }
            }
        }
    });
}

/// Is a test collecting observations on this thread?
pub fn observing() -> bool {
    SINK.with(|s| s.try_borrow().map(|o| o.is_some()).unwrap_or(false))
}

/// Collect the observations of the programs this thread runs (and the
/// threads `Carry` reaches) until the guard goes.
pub fn observe_into(sink: Sink) -> SinkGuard {
    let before = SINK.with(|s| {
        s.try_borrow_mut()
            .map(|mut o| o.replace(sink))
            .ok()
            .flatten()
    });
    SinkGuard(before)
}

/// Restores the thread's earlier sink.
pub struct SinkGuard(Option<Sink>);

impl Drop for SinkGuard {
    fn drop(&mut self) {
        let before = self.0.take();
        SINK.with(|s| {
            if let Ok(mut o) = s.try_borrow_mut() {
                *o = before;
            }
        });
    }
}

/// How many generated routines this build has.
pub fn generated_routines() -> usize {
    rpgnative::count()
}

/// The engine for programs run by this thread: a test's choice, else
/// the variable (read once per process).
pub fn engine() -> Result<Engine, String> {
    if let Some(e) = OVERRIDE.with(Cell::get) {
        return Ok(e);
    }
    static FROM_ENV: OnceLock<Result<Engine, String>> = OnceLock::new();
    FROM_ENV
        .get_or_init(|| {
            let v = std::env::var(ENGINE_VAR).ok();
            Engine::parse(v.as_deref())
        })
        .clone()
}

/// Run `f` with programs on this thread (and the threads RPG calls run
/// on, see `Carry`) using `engine`. For tests that compare engines in one
/// process (docs/native/adr/0005).
pub fn with_engine<R>(engine: Engine, f: impl FnOnce() -> R) -> R {
    let _g = scope(engine);
    f()
}

/// Programs on this thread use `engine` until the guard goes (for async
/// tests: the guard lasts across `.await` on a single-threaded runtime,
/// and `rpg::call_any` carries it to the program's thread).
pub fn scope(engine: Engine) -> CarryGuard {
    Carry(Some(engine), None).enter()
}

/// A thread's engine choice and observation sink, carried to the thread
/// a program runs on (`rpg::call_any` and `call_procedure` run programs
/// with `spawn_blocking`).
#[derive(Clone, Debug)]
pub struct Carry(Option<Engine>, Option<Sink>);

/// This thread's engine choice and sink.
pub fn carry() -> Carry {
    Carry(
        OVERRIDE.with(Cell::get),
        SINK.with(|s| s.try_borrow().ok().and_then(|o| o.clone())),
    )
}

impl Carry {
    /// Make the choice (and the sink) this thread's until the guard goes.
    pub fn enter(self) -> CarryGuard {
        let before = OVERRIDE.with(|o| o.replace(self.0));
        CarryGuard {
            before,
            _sink: self.1.map(observe_into),
        }
    }
}

/// Restores the thread's earlier choice (and, as it drops, its sink).
pub struct CarryGuard {
    before: Option<Engine>,
    _sink: Option<SinkGuard>,
}

impl Drop for CarryGuard {
    fn drop(&mut self) {
        OVERRIDE.with(|o| o.set(self.before));
    }
}

/// A routine's place in its program: what identifies it in every copy of
/// the compiled program.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    /// The main detail calculations.
    Detail,
    /// The total calculations (L1-L9, LR).
    Total,
    /// A subroutine of the main calculations.
    Sub(String),
    /// A part of a procedure.
    Proc(String, Part),
}

/// A part of a procedure.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    Detail,
    Total,
    Sub(String),
    /// The ON-EXIT section's calculations.
    OnExit,
    /// A total or a subroutine of the ON-EXIT section.
    OnExitOther(Box<Part>),
}

impl std::fmt::Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Place::Detail => write!(f, "the detail calculations"),
            Place::Total => write!(f, "the total calculations"),
            Place::Sub(s) => write!(f, "subroutine {s}"),
            Place::Proc(p, part) => match part {
                Part::Detail => write!(f, "procedure {p}"),
                Part::Total => write!(f, "procedure {p}, total calculations"),
                Part::Sub(s) => write!(f, "procedure {p}, subroutine {s}"),
                Part::OnExit => write!(f, "procedure {p}, ON-EXIT"),
                Part::OnExitOther(_) => write!(f, "procedure {p}, ON-EXIT section"),
            },
        }
    }
}

/// Every routine of a program with its place, in a fixed order (names
/// sorted), so tables and emitted files never depend on hash order.
pub fn routines(prog: &Program) -> Vec<(Place, &Code)> {
    let mut out = Vec::new();
    push_routines(&mut out, &prog.routines, &|p| p);
    let mut procs: Vec<_> = prog.procs.iter().collect();
    procs.sort_by(|a, b| a.0.cmp(b.0));
    for (name, pi) in procs {
        let n = name.clone();
        push_proc(&mut out, &pi.routines, &n, false);
        if let Some((_, exit)) = &pi.on_exit {
            push_proc(&mut out, exit, &n, true);
        }
    }
    out
}

fn push_routines<'p>(
    out: &mut Vec<(Place, &'p Code)>,
    r: &'p Routines,
    wrap: &dyn Fn(Place) -> Place,
) {
    out.push((wrap(Place::Detail), &r.detail));
    out.push((wrap(Place::Total), &r.total));
    let mut subs: Vec<_> = r.subroutines.iter().collect();
    subs.sort_by(|a, b| a.0.cmp(b.0));
    for (s, c) in subs {
        out.push((wrap(Place::Sub(s.clone())), c));
    }
}

fn push_proc<'p>(out: &mut Vec<(Place, &'p Code)>, r: &'p Routines, name: &str, on_exit: bool) {
    let part = |p: Part| {
        if on_exit {
            match p {
                Part::Detail => Part::OnExit,
                other => Part::OnExitOther(Box::new(other)),
            }
        } else {
            p
        }
    };
    out.push((Place::Proc(name.to_string(), part(Part::Detail)), &r.detail));
    out.push((Place::Proc(name.to_string(), part(Part::Total)), &r.total));
    let mut subs: Vec<_> = r.subroutines.iter().collect();
    subs.sort_by(|a, b| a.0.cmp(b.0));
    for (s, c) in subs {
        out.push((Place::Proc(name.to_string(), part(Part::Sub(s.clone()))), c));
    }
}

/// What a routine's fingerprint covers besides its code: the scheme, the
/// generator and runtime interface, and the compiler (ADR 0002).
pub fn fingerprint_header() -> String {
    format!(
        "as4tainr-native routine v1\nrpgcg {}\nrpgrt {}\nrustc {}\n",
        rpgcg::VERSION,
        rpgrt::VERSION,
        rpgnative::RUSTC_VERSION
    )
}

/// The fingerprint of a compiled routine: SHA-256 over the header, each
/// instruction's Debug text, and the storage layout its lowered code uses
/// (ADR 0002, 0006), in lower-case hexadecimal.
pub fn fingerprint(code: &Code, layout: &[String]) -> String {
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
    ctx.update(fingerprint_header().as_bytes());
    let mut line = String::new();
    for i in code {
        line.clear();
        // Writing to a String does not fail.
        let _ = writeln!(line, "{i:?}");
        ctx.update(line.as_bytes());
    }
    for l in layout {
        ctx.update(l.as_bytes());
        ctx.update(b"\n");
    }
    let mut hex = String::with_capacity(64);
    for b in ctx.finish().as_ref() {
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// A routine as the generator gets it, with its fingerprint.
pub fn routine(prog: &Program, place: &Place, code: &Code) -> rpgcg::ir::Routine {
    let (ins, layout) = lower::lower(prog, place, code);
    rpgcg::ir::Routine {
        fingerprint: fingerprint(code, &layout),
        ins,
    }
}

/// One routine of a program in its native table.
#[derive(Clone, Debug)]
pub struct Entry {
    pub place: Place,
    pub fingerprint: String,
    /// The first instruction's source line (0: the routine is empty).
    pub line: usize,
    pub run: Option<RoutineFn>,
}

/// A program's generated routines: built once per compiled program and
/// shared by its copies (ADR 0002).
#[derive(Debug)]
pub struct NativeTable {
    pub program: String,
    pub entries: Vec<Entry>,
    by_place: HashMap<Place, usize>,
    /// The main program's subroutines by name (found with no allocation).
    main_subs: HashMap<String, usize>,
    /// The first routine with no generated code (worked out once).
    first_missing: Option<usize>,
}

impl NativeTable {
    pub fn build(prog: &Program) -> NativeTable {
        let mut entries = Vec::new();
        let mut by_place = HashMap::new();
        let mut main_subs = HashMap::new();
        for (place, code) in routines(prog) {
            let fingerprint = routine(prog, &place, code).fingerprint;
            let run = rpgnative::lookup(&fingerprint);
            if let Place::Sub(n) = &place {
                main_subs.insert(n.clone(), entries.len());
            }
            by_place.insert(place.clone(), entries.len());
            entries.push(Entry {
                place,
                line: code.first().map(|i| i.line).unwrap_or(0),
                fingerprint,
                run,
            });
        }
        let first_missing = entries.iter().position(|e| e.run.is_none());
        NativeTable {
            program: prog.name.clone(),
            entries,
            first_missing,
            by_place,
            main_subs,
        }
    }

    /// The first routine with no generated code, if any.
    pub fn missing(&self) -> Option<&Entry> {
        self.first_missing.and_then(|i| self.entries.get(i))
    }

    pub fn get(&self, place: &Place) -> Option<&Entry> {
        self.by_place.get(place).and_then(|&i| self.entries.get(i))
    }

    /// The entry of the routine `code` is in `prog`, the program this
    /// table was built for (any copy of it): found by comparing addresses
    /// with the program's own routines, with no allocation (the
    /// dispatcher calls this at every routine entry).
    pub fn find(&self, prog: &Program, code: &Code) -> Option<&Entry> {
        let main = |r: &Routines| -> Option<Part> {
            if std::ptr::eq(&r.detail, code) {
                return Some(Part::Detail);
            }
            if std::ptr::eq(&r.total, code) {
                return Some(Part::Total);
            }
            r.subroutines
                .iter()
                .find(|(_, c)| std::ptr::eq(*c, code))
                .map(|(n, _)| Part::Sub(n.clone()))
        };
        // The main program's routines first (the common case).
        let r = &prog.routines;
        if std::ptr::eq(&r.detail, code) {
            return self.get(&Place::Detail);
        }
        if std::ptr::eq(&r.total, code) {
            return self.get(&Place::Total);
        }
        if let Some((n, _)) = r.subroutines.iter().find(|(_, c)| std::ptr::eq(*c, code)) {
            return self
                .main_subs
                .get(n.as_str())
                .and_then(|&i| self.entries.get(i));
        }
        for (name, pi) in &prog.procs {
            if let Some(part) = main(&pi.routines) {
                return self.get(&Place::Proc(name.clone(), part));
            }
            if let Some((_, exit)) = &pi.on_exit {
                if let Some(part) = main(exit) {
                    let part = match part {
                        Part::Detail => Part::OnExit,
                        other => Part::OnExitOther(Box::new(other)),
                    };
                    return self.get(&Place::Proc(name.clone(), part));
                }
            }
        }
        None
    }
}

/// The native table of a compiled program, made when first needed and
/// shared by every copy of it (`Program` is cloned at each activation).
#[derive(Clone, Default)]
pub struct Slot(Arc<OnceLock<NativeTable>>);

impl Slot {
    pub fn table(&self, prog: &Program) -> &NativeTable {
        self.0.get_or_init(|| NativeTable::build(prog))
    }
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Slot")
    }
}

/// The variable naming the directory generated routines are written to.
pub const EMIT_VAR: &str = "AS4TAINR_NATIVE_EMIT";

/// Generate the routines of a program just compiled into
/// `AS4TAINR_NATIVE_EMIT` (when it is set): `<fingerprint>.rs` and
/// `<fingerprint>.map.json` for each routine not there yet
/// (docs/native/adr/0004). A routine the generator refuses, or a file
/// that cannot be written, is reported on standard error; the program
/// then has no native code for it.
pub fn emit(prog: &Program) {
    let Some(dir) = std::env::var_os(EMIT_VAR).filter(|d| !d.is_empty()) else {
        return;
    };
    let report = emit_into(prog, std::path::Path::new(&dir));
    for e in &report.errors {
        use std::io::Write as _;
        // Standard error may be closed; there is nowhere else to say it.
        let _ = writeln!(std::io::stderr(), "as4tainr: {EMIT_VAR}: {e}");
    }
}

/// What generating a program's routines did.
#[derive(Debug, Default)]
pub struct EmitReport {
    /// Routines written now.
    pub written: usize,
    /// Routines whose code was there already.
    pub present: usize,
    /// One line per routine not written: the program, the routine, why.
    pub errors: Vec<String>,
}

/// Generate the routines of a compiled program into `dir`:
/// `<fingerprint>.rs` and `<fingerprint>.map.json` for each routine not
/// there yet. A routine the generator refuses, or a file that cannot be
/// written, is reported; the program then has no native code for it.
pub fn emit_into(prog: &Program, dir: &std::path::Path) -> EmitReport {
    let mut report = EmitReport::default();
    for (place, code) in routines(prog) {
        let ir = routine(prog, &place, code);
        let fp = ir.fingerprint.clone();
        if dir.join(format!("{fp}.rs")).exists() {
            report.present = report.present.saturating_add(1);
            continue;
        }
        let r = rpgcg::generate(&ir)
            .map_err(|e| e.to_string())
            .and_then(|g| {
                write_new(dir, &format!("{fp}.map.json"), &g.map)?;
                write_new(dir, &format!("{fp}.rs"), &g.source)
            });
        match r {
            Ok(()) => report.written = report.written.saturating_add(1),
            Err(e) => report
                .errors
                .push(format!("program {}, {place}: {e}", prog.name)),
        }
    }
    report
}

/// One program of a library, generated (`as4tainr-native-emit`).
#[derive(Debug)]
pub struct LibraryProgram {
    pub name: String,
    /// The routines, or why the program did not compile.
    pub result: Result<EmitReport, String>,
}

/// Compile every RPG program and service program of `library` as their
/// activation does, with the library list they run with (`libl`,
/// searched in order) and no file overrides, and generate their routines
/// into `dir`: a program's entry module and the modules bound into it
/// (CRTPGM, CRTBNDRPG with a binding directory), a service program's
/// modules. Objects are taken in name order; each line of the result is
/// one program or module.
pub async fn emit_library(
    client: &tokio_postgres::Client,
    libl: &[String],
    library: &str,
    dir: &std::path::Path,
) -> anyhow::Result<Vec<LibraryProgram>> {
    let library = library.to_ascii_uppercase();
    let mut out = Vec::new();
    let rows = client
        .query(
            "SELECT name FROM as4tainr.programs WHERE library = $1 AND attribute IN ('RPGLE', 'SQLRPGLE', 'RPG') ORDER BY name",
            &[&library],
        )
        .await?;
    for row in rows {
        let name: String = row.try_get(0)?;
        let found =
            crate::catalog::find_program(client, std::slice::from_ref(&library), &name).await?;
        let Some(p) = found else {
            out.push(LibraryProgram {
                name,
                result: Err("not found".to_string()),
            });
            continue;
        };
        let result = super::compile_ovr(client, libl, &name, &p.source, &p.options, &[])
            .await
            .map(|prog| emit_into(&prog, dir))
            .map_err(|e| format!("{e:#}"));
        out.push(LibraryProgram {
            name: name.clone(),
            result,
        });
        if let Some(b) = &p.binding {
            emit_modules(client, libl, &name, &b.modules, dir, &mut out).await;
        }
    }
    let rows = client
        .query(
            "SELECT name FROM as4tainr.service_programs WHERE library = $1 ORDER BY name",
            &[&library],
        )
        .await?;
    for row in rows {
        let name: String = row.try_get(0)?;
        match crate::ile::find_srvpgm(client, libl, &library, &name).await {
            Ok((_, b)) => emit_modules(client, libl, &name, &b.modules, dir, &mut out).await,
            Err(e) => out.push(LibraryProgram {
                name,
                result: Err(format!("{e:#}")),
            }),
        }
    }
    Ok(out)
}

/// The RPG modules of a binding, compiled as the activation compiles them
/// (`compile_with` with the caller's library list), into `dir`.
async fn emit_modules(
    client: &tokio_postgres::Client,
    libl: &[String],
    object: &str,
    modules: &[crate::ile::ModuleCopy],
    dir: &std::path::Path,
    out: &mut Vec<LibraryProgram>,
) {
    for m in modules.iter().filter(|m| m.attribute != "CLLE") {
        let result = super::compile_with(client, libl, &m.name, &m.source, &m.options)
            .await
            .map(|prog| emit_into(&prog, dir))
            .map_err(|e| format!("{e:#}"));
        out.push(LibraryProgram {
            name: format!("{object} module {}", m.name),
            result,
        });
    }
}

/// Write a file under a temporary name, then rename it into place, so a
/// build never sees half a file.
fn write_new(dir: &std::path::Path, name: &str, text: &str) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = dir.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, dir.join(name)).map_err(|e| format!("{}: {e}", tmp.display()))
}

/// The variable naming a file where each program that does not run on
/// generated code is noted (scripts/native-diff.sh measures coverage
/// with it): engine, program, fingerprint, reason; one line each.
pub const REFUSALS_VAR: &str = "AS4TAINR_NATIVE_REFUSALS";

/// The most lines a process adds to the refusals file.
const MAX_REFUSALS: usize = 10_000;

/// Note a program that does not run on generated code, when
/// `AS4TAINR_NATIVE_REFUSALS` names a file.
pub fn note_refusal(engine: Engine, program: &str, fingerprint: &str, reason: &str) {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static WRITTEN: AtomicUsize = AtomicUsize::new(0);
    let Some(path) = std::env::var_os(REFUSALS_VAR).filter(|p| !p.is_empty()) else {
        return;
    };
    // Once per program and fingerprint in a process.
    static SEEN: Mutex<Option<HashSet<String>>> = Mutex::new(None);
    {
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        let set = seen.get_or_insert_with(HashSet::new);
        if !set.insert(format!("{program}\t{fingerprint}")) || set.len() > MAX_REFUSALS {
            return;
        }
    }
    if WRITTEN.fetch_add(1, Ordering::Relaxed) >= MAX_REFUSALS {
        return;
    }
    let line = format!(
        "{engine:?}\t{program}\t{fingerprint}\t{}\n",
        reason.replace(['\t', '\n'], " ")
    );
    // One write per line, appended: lines from parallel tests stay whole.
    // A file that cannot be written leaves the measure short, nothing more.
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = f.write_all(line.as_bytes());
    }
}

/// The most programs `auto` mode names on standard error in a process.
const MAX_NOTED: usize = 4096;

/// `auto`: say once per program and process why it runs in the
/// interpreter.
pub fn note_fallback(program: &str, reason: &str) {
    static NOTED: Mutex<Option<HashSet<String>>> = Mutex::new(None);
    let mut noted = NOTED.lock().unwrap_or_else(|e| e.into_inner());
    let set = noted.get_or_insert_with(HashSet::new);
    if set.contains(program) {
        return;
    }
    if set.len() >= MAX_NOTED {
        set.clear();
    }
    set.insert(program.to_string());
    use std::io::Write as _;
    // Standard error may be closed; there is nowhere else to say it.
    let _ = writeln!(
        std::io::stderr(),
        "as4tainr: {ENGINE_VAR}=auto: {program} runs in the interpreter: {reason}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_values() {
        // The values CLAUDE.native.md section 3 defines; unset is interp.
        assert_eq!(Engine::parse(None), Ok(Engine::Interp));
        assert_eq!(Engine::parse(Some("interp")), Ok(Engine::Interp));
        assert_eq!(Engine::parse(Some("rust")), Ok(Engine::Rust));
        assert_eq!(Engine::parse(Some("auto")), Ok(Engine::Auto));
        for bad in ["native", "RUST", "x"] {
            let e = Engine::parse(Some(bad)).err().unwrap_or_default();
            assert!(e.contains(ENGINE_VAR), "{e}");
        }
    }

    #[test]
    fn override_is_scoped_and_carried() {
        let outside = carry();
        let inside = with_engine(Engine::Rust, || {
            let c = carry();
            let on_other_thread = std::thread::spawn(move || {
                let _g = c.enter();
                engine()
            })
            .join();
            (engine(), on_other_thread.ok())
        });
        assert_eq!(inside.0, Ok(Engine::Rust));
        assert_eq!(inside.1, Some(Ok(Engine::Rust)));
        assert_eq!(carry().0, outside.0);
        assert!(!observing());
    }

    #[test]
    fn fingerprint_is_hex_and_depends_on_code() {
        use super::super::code::{Ins, I};
        let a: Code = vec![I {
            ins: Ins::Nop,
            line: 1,
            cond: Default::default(),
        }];
        let mut b = a.clone();
        let fa = fingerprint(&a, &[]);
        assert_eq!(fa.len(), 64);
        assert!(fa.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(fa, fingerprint(&a.clone(), &[]));
        assert_ne!(
            fa,
            fingerprint(&a, &["layout X 1 0 Packed(5, 0)".to_string()])
        );
        if let Some(i) = b.first_mut() {
            i.line = 2;
        }
        assert_ne!(fa, fingerprint(&b, &[]));
    }
}
