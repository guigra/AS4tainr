//! as4tainr: the registry of RPG routines compiled to Rust
//! (docs/native/adr/0003). The build script includes every generated
//! routine of `AS4TAINR_NATIVE_DIR` as a module of this crate; the
//! lints below apply to that generated code too.

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

use rpgrt::native::RoutineFn;

include!(concat!(env!("OUT_DIR"), "/programs.rs"));

/// The generated routine with this fingerprint.
pub fn lookup(fingerprint: &str) -> Option<RoutineFn> {
    let at = ROUTINES
        .binary_search_by(|(fp, _)| (*fp).cmp(fingerprint))
        .ok()?;
    ROUTINES.get(at).map(|(_, f)| *f)
}

/// How many routines this build has.
pub fn count() -> usize {
    ROUTINES.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_and_found() {
        // The build script sorts the table; lookup relies on it.
        assert!(ROUTINES.windows(2).all(|w| match w {
            [a, b] => a.0 < b.0,
            _ => true,
        }));
        for (fp, _) in ROUTINES {
            assert!(lookup(fp).is_some());
        }
        assert!(lookup("not a fingerprint").is_none());
    }
}
