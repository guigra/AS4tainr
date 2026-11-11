//! The RPG engine's run-time storage and values (see docs/rpg-engine.md):
//! exact decimals, IBM i data formats, program storage and pointers,
//! dates and times.

// Moved unchanged from as4tainr-core (core/src/rpg/); it keeps
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
pub mod data;
// Moved unchanged from as4tainr-core (core/src/rpg/); see `data`.
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
pub mod dates;
// Moved unchanged from as4tainr-core (core/src/rpg/); see `data`.
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
pub mod dec;
// Moved unchanged from as4tainr-core (core/src/rpg/); see `data`.
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
pub mod mem;
