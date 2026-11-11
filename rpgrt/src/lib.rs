//! as4tainr RPG runtime: what a running RPG program needs whichever
//! engine runs it (the interpreter in `as4tainr-core`, or generated
//! code). See docs/native/STATUS.md.
//!
//! The modules keep the paths they had in `as4tainr-core`
//! (`rpg::dec`, `rpg::data`, `rpg::mem`, `rpg::dates`, `tn5250::ebcdic`),
//! so their code moved unchanged; `as4tainr-core` re-exports them from
//! those paths.
//!
//! Lints (CLAUDE.native.md section 5): `unsafe_code` and
//! `unused_must_use` are forbidden for the whole crate. The clippy lints
//! are denied here rather than forbidden, because code moved unchanged
//! from upstream keeps upstream's lints and a forbidden lint cannot be
//! allowed again for it; the moved modules allow them where they are
//! declared (`rpg.rs`, `tn5250.rs`). Every new module must not allow any
//! of them.

#![forbid(unsafe_code, unused_must_use)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects
)]

pub mod rpg;
pub mod tn5250;
