# Native code in a production build

A production image ships the as4tainr binary with the native code of its RPG programs built in. The image contains no `cargo` and no `rustc` (CLAUDE.native.md section 3, ADR 0003). A program created after the build runs in the interpreter under `AS4TAINR_RPG_ENGINE=auto` until the next build.

## Steps

1. Create the application on a build database, as it will be in production (`as4tainr import`, the create commands).
2. Generate the native code of its libraries:

   ```sh
   AS4TAINR_DATABASE_URL=postgres://... \
     target/release/as4tainr-native-emit --out target/rpg-native/app --libl APP,QGPL APP
   ```

   `as4tainr-native-emit` compiles each RPG program as its activation compiles it, and writes its routines to `--out`. That covers the entry module, the modules bound into it (CRTPGM, CRTBNDRPG with BNDDIR) and the modules of the library's service programs. It prints a line per program or module. The exit status is 1 when something did not compile or a routine was not generated.

   `--libl` must be the library list the programs run with in production. A routine's code depends on the files and members it resolves through that list. With a different list the fingerprints do not match, and those routines run in the interpreter (`auto`) or are refused (`rust`).
3. Build with that code: `AS4TAINR_NATIVE_DIR=$PWD/target/rpg-native/app cargo build --locked --release`. `Cargo.toml` and `Cargo.lock` do not change.
4. Run with `AS4TAINR_RPG_ENGINE=auto`, or with `rust` to require native code for every program.

## Limits

- Programs that a job creates at run time are not in the build. Under `auto` they run in the interpreter.
- A CL program has no native code; only RPG is compiled.
- A program whose source changes between processes gets a different fingerprint every time, so it never has native code. Examples are programs a job generates, or tests that put their own library names in the source.

## Checked

2026-11-12, x86_64, release build of the native/engine branch:

- The samples were made on a new database (`scripts/samples.sh`).
- `as4tainr-native-emit --out target/rpg-native/n5 QGPL CONTACTS` wrote 4 routines; a fifth was already there. QGPL's sample programs are CL, or RPG run from source, so none were generated there.
- With PATH holding neither `cargo` nor `rustc`, `as4tainr call CONTACTS/CONTCNT` under `AS4TAINR_RPG_ENGINE=rust` gave the same output as under `interp`, with no refusal.
- CONTLST is interactive. It ends the same way under both engines when called without a display.

This was a development check, not evidence: there is no manifest.
