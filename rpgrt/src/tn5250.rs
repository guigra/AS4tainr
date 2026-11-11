//! Character conversion the runtime shares with the 5250 display server.
//! Only the CCSID 37 tables are here; the display server itself stays in
//! as4tainr-core (`tn5250`), which re-exports this module.

// Moved unchanged from as4tainr-core (core/src/tn5250/); it keeps
// upstream's lints, not CLAUDE.native.md section 5's (see lib.rs).
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects
)]
pub mod ebcdic;
