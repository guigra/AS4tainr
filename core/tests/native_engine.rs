//! The engine switch of the native backend (docs/native/adr/0004): what
//! `AS4TAINR_RPG_ENGINE` does before any generated code exists. The
//! expected behavior is the one CLAUDE.native.md section 3 defines for
//! the flag; outputs are compared with the interpreter's own.
//!
//! Needs a PostgreSQL database (AS4TAINR_TEST_DATABASE_URL); without it
//! the tests say they are skipped.

use as4tainr_core::rpg::native::{scope, Engine, ENGINE_VAR};
use as4tainr_core::{catalog, rpg, runtime};

fn db_url() -> Option<String> {
    std::env::var("AS4TAINR_TEST_DATABASE_URL").ok()
}

macro_rules! require_db {
    () => {
        match db_url() {
            Some(u) => u,
            None => {
                eprintln!("skipped: AS4TAINR_TEST_DATABASE_URL not set");
                return;
            }
        }
    };
}

/// A cycle program with *INZSR and *PSSR, each saying it ran. The
/// process number is in its text, so no earlier run can have generated
/// native code for it (the refusal below depends on there being none).
fn program() -> String {
    format!(
        "**FREE
dsply 'HELLO {}';
*inlr = *on;
begsr *inzsr;
  dsply 'INZ';
endsr;
begsr *pssr;
  dsply 'PSSR';
endsr;
",
        std::process::id()
    )
}

async fn run(url: &str, lib: &str, name: &str) -> rpg::CallResult {
    rpg::call(
        url,
        vec![lib.to_string()],
        "QSECOFR".to_string(),
        None,
        None,
        None,
        name.to_string(),
        program(),
        vec![],
    )
    .await
    .unwrap_or_else(|e| rpg::CallResult {
        messages: vec![],
        logged: vec![],
        printed: vec![],
        error: Some(e),
        params: vec![],
    })
}

async fn library(url: &str) -> String {
    let client = runtime::connect_db(url).await.unwrap();
    catalog::init(&client).await.unwrap();
    let lib = format!("N{}", std::process::id() % 1_000_000);
    if catalog::create_library(&client, &lib, "native engine tests")
        .await
        .is_err()
    {
        // It is there from an earlier run of this process number.
    }
    lib
}

#[tokio::test]
async fn native_engine_switch() {
    let url = require_db!();
    let lib = library(&url).await;

    let g = scope(Engine::Interp);
    let interp = run(&url, &lib, "NATSW1").await;
    drop(g);
    assert!(interp.error.is_none(), "{:?}", interp.error);
    assert!(
        interp.messages.iter().any(|m| m.contains("HELLO")),
        "{:?}",
        interp.messages
    );

    // auto: no generated code for this program, so the interpreter runs
    // it, with the same results.
    let g = scope(Engine::Auto);
    let auto = run(&url, &lib, "NATSW1").await;
    drop(g);
    assert!(auto.error.is_none(), "{:?}", auto.error);
    assert_eq!(auto.messages, interp.messages);
    assert_eq!(auto.printed, interp.printed);

    // rust: the program does not start. No calculation runs, not even
    // *INZSR, and its *PSSR does not take the refusal.
    let g = scope(Engine::Rust);
    let rust = run(&url, &lib, "NATSW1").await;
    drop(g);
    let err = format!("{:#}", rust.error.expect("rust must refuse the program"));
    assert!(err.contains(&format!("{ENGINE_VAR}=rust")), "{err}");
    assert!(err.contains("NATSW1"), "{err}");
    assert!(err.contains("no native code"), "{err}");
    assert!(
        !rust
            .messages
            .iter()
            .any(|m| m.contains("INZ") || m.contains("HELLO") || m.contains("PSSR")),
        "{:?}",
        rust.messages
    );
}

/// Compiling is not running: commands that compile programs only to look
/// at them (CRTPGM, CRTSRVPGM, bound procedure lookup) work under `rust`
/// whether or not native code exists (docs/native/adr/0004).
#[tokio::test]
async fn native_rust_engine_still_compiles() {
    let url = require_db!();
    let lib = library(&url).await;
    let client = runtime::connect_db(&url).await.unwrap();
    let g = scope(Engine::Rust);
    let prog = rpg::compile_with(
        &client,
        std::slice::from_ref(&lib),
        "NATSW2",
        &program(),
        "",
    )
    .await;
    drop(g);
    assert!(prog.is_ok(), "{:?}", prog.err());
}
