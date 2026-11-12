//! A tripwire for the native engine (docs/native/adr/0005): the generated
//! code's runtime (core/src/rpg/native_rt.rs) copies what
//! `Machine::run_code_at` does for each instruction and how it routes
//! errors. When upstream changes that code, or the error functions it
//! calls, this test fails: review native_rt.rs against the new code,
//! then record the new hash here, saying why in the commit.
//!
//! This is not a fidelity test; it compares the source text with the one
//! native_rt.rs was last reviewed against.

const INTERP: &str = include_str!("../src/rpg/interp.rs");
const ERRORS: &str = include_str!("../src/rpg/errors.rs");

/// The text of a function, from its signature to its closing brace at
/// the same indentation.
fn function(src: &str, signature: &str, indent: &str) -> String {
    let Some((_, rest)) = src.split_once(signature) else {
        panic!("{signature} is gone: review native_rt.rs");
    };
    let close = format!("\n{indent}}}\n");
    let Some((body, _)) = rest.split_once(&close) else {
        panic!("no end for {signature}");
    };
    format!("{signature}{body}{close}")
}

fn sha256(text: &str) -> String {
    let d = ring::digest::digest(&ring::digest::SHA256, text.as_bytes());
    d.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn native_runtime_matches_the_interpreter_it_copies() {
    let watched = [
        function(
            INTERP,
            "    fn run_code_at(&mut self, code: &Code, start: usize) -> Result<Flow> {",
            "    ",
        ),
        function(
            ERRORS,
            "pub(super) fn is_signal(err: &anyhow::Error) -> bool {",
            "",
        ),
        function(
            ERRORS,
            "pub(super) fn on_error_takes(entry: &str, code: u32) -> bool {",
            "",
        ),
    ];
    let hash = sha256(&watched.concat());
    // Reviewed against native_rt.rs on 2026-11-11 (upstream 29f23f6 plus
    // the native hook at the start of run_code_at).
    let reviewed = "7491eb2a14daf1760610cb04adcfe070b8fdfa9743be7e4b3c64f158c26cbee6";
    assert_eq!(
        hash, reviewed,
        "run_code_at or the error functions changed: review native_rt.rs, then record the new hash"
    );
}
