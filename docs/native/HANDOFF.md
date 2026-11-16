# Native RPG backend: handoff

Last update: 2026-11-14, before moving part of the work to a Mac M1. Both native branches and the held upstream fix branches are pushed to origin (guigra/AS4tainr). The branch is `native/engine`, stacked on `native/runtime-extraction`, both rebased on upstream main dd47e28. Nothing of these branches is pushed. The previous tip is kept as `native/engine-pre-rebase2`.

## State

- **Stage 1 is complete.** Every program runs natively under `AS4TAINR_RPG_ENGINE=rust|auto`, with the interpreter as the reference.
- **Stage 2, S2.1 to S2.20, is committed** (ADR 0006). S2.21 (array elements with field indexes) is committed too. S2.22-S2.24 are committed and validated on x86_64. S2.25 (%SIZE, %ELEM) is validated on aarch64 (Mac M1, 2026-11-14: native-diff PASS, evidence/2026-11-14-s225-aarch64-mac.json), not yet on x86_64. Lowered:
  - conditions, loops, numeric and character EVAL;
  - fixed-form arithmetic, DIV/MVR, `/`, %REM, %DIV;
  - %SUBST, %SCAN, %TRIM, %XLATE, %CHAR, %INT, %DEC, %LEN;
  - MOVE/MOVEL;
  - array elements with literal indexes, qualified subfields, varying fields, procedures.
- **Tools:**
  - `as4tainr-native-emit` (native-tools), with docs/native/production.md;
  - `scripts/native-diff.sh`, `scripts/native-e2e.sh` (the 5250 tests through Playwright) and `scripts/native-bench.sh` (NBENCH1, NBENCH2).
- **Evidence:** docs/native/evidence/2026-11-14-rebased2.json (0b9d585, both platforms).
- **Upstream:** this fork's issues #3 to #8 and PRs #9 to #17 are in kyleturley-cyber/AS4tainr.
  - Merged: #9, #10 and #13.
  - #11, #12 and #14 were updated after review.
  - #15, #16 and #17 are new.
  - #12 must be rebased once #11 is merged; their check.rs changes conflict.
  - The PR branches are `fix/*`, in worktrees /home/ctl/data/as4tainr-up*. Validate and publish them with /home/ctl/data/as4tainr-up-logs/vp.sh.
- The PRs and issues are framed as collaboration only: no mention of this fork or the native backend (the user's decision).

## Environment

- **Machines:**
  - this host: x86_64, 3.8 GB, short of memory under parallel cargo work;
  - jsr-test-arm: aarch64, work under /home/ctl/data.
  - Both have PostgreSQL 16 in podman `as4tainr-test-db`, with `--shm-size=1g`.
  - PUB400 is reachable over ssh (`ssh pub400`) from both.
  - Playwright and headless Chromium are on the VM, in /home/ctl/data/pw-venv and /home/ctl/data/pw-browsers.
- **Pushing to the fork:** `git -c credential.helper= -c credential.helper='!gh auth git-credential' push origin <branch>`. There is no GitHub ssh key.
- **Syncing a tree to the VM:** use `rsync -rlp --checksum`, not `-a`. Kept mtimes make cargo keep stale release builds of rpgcg and rpgrt.

## Known failures (not the backend's)

- clippy `chunks_exact_to_as_chunks` in the host servers, with rustc 1.99.1.
- `save_changed_objects` is intermittent under parallel runs.
- `host_servers_sign_on_and_database_attributes` and `ifs_statistics_json_defaults_and_new_activation_groups` hung once under the parallel release run on x86_64. Both pass alone.
- A release build with 2 jobs can run out of memory on the VM. Use CARGO_BUILD_JOBS=1.

## Next steps

2. Block A of stage 2: %EDITC/%EDITW, %REPLACE, %CHECK, %SCANRPL, %LOOKUP, figurative constants, whole data structure assignment, CALLP. EXSR without run_code_at needs the user's approval, because it touches interp.rs.
3. Priority 3:
   - a production image built with native-tools;
   - OrderDesk under `rust`;
   - gate3 (three clerks at once) under `rust`.
4. Follow the upstream PRs: answer reviews, and rebase #12 after #11.

## Continuing on a Mac M1 (2026-11-14)

The user can go on with the most blocking parts on a Mac M1, which has more memory than jsr-test-arm.

- **Get the code.** Clone guigra/AS4tainr and check out `native/engine`. It stacks on `native/runtime-extraction` and is rebased on upstream dd47e28. Add the remote `upstream` = kyleturley-cyber/AS4tainr.
- **Where to run it.** The scripts and tests assume Linux:
  - podman;
  - `/QSYS.LIB` present;
  - mount namespaces for QSH (`qshfs.rs`);
  - /proc in places.

  The safest setup is a Debian aarch64 VM on the Mac (UTM, Lima or `podman machine`), set up like jsr-test-arm:
  - rustup with the toolchain of `rust-toolchain.toml` (1.99.1);
  - PostgreSQL 16 in podman: `podman run -d --name as4tainr-test-db --shm-size=1g -p 127.0.0.1:5432:5432 -e POSTGRES_USER=as4tainr -e POSTGRES_PASSWORD=as4tainr -e POSTGRES_DB=as4tainr docker.io/library/postgres:16`;
  - `sudo mkdir /QSYS.LIB` (empty, owned by the user).

  Native macOS may build, but it is not tested here.
- **PUB400.** A new ed25519 key without a passphrase, installed in /home/GUIGRA26/.ssh/authorized_keys as `docs/native/pub400.md` says, and a `Host pub400` entry. Never copy keys or passwords through Git.
- **Playwright** (for `scripts/native-e2e.sh`): a Python venv with `pip install playwright` and `python -m playwright install chromium`.
- **Checks.** The commands in CLAUDE.native.md section 7. Development runs: `AS4TAINR_RPG_ENGINE=rust scripts/native-diff.sh`, `scripts/native-bench.sh` (NATIVE_BENCH_PGM=NBENCH2 too), `scripts/native-e2e.sh`.
- **The upstream hang (issue #18).** Full suite runs, native-diff included, can hang in `catalog::init`'s lock chain. While a run hangs, `pg_stat_activity` shows an `ALTER TABLE as4tainr.objects ADD COLUMN IF NOT EXISTS ...` waiting behind an `idle in transaction` session. Stop the `rla_test` process and run again. The fix is the held branch `fix/init-lock-timeout`. Carrying it on `native/engine` needs the user's approval: it touches core/src/catalog.rs.
- **First things to do there:**
  1. Validate S2.25: clippy, the native units, native-diff.
  2. Stage 2 block A, in this order:
     - CALLP and procedure calls;
     - %EDITC and %EDITW;
     - %LOOKUP;
     - %SCANRPL;
     - data structure arrays.
  3. Priority 3:
     - a production image with native-tools;
     - OrderDesk under `rust`;
     - gate3 under `rust`.
- **Upstream branches on origin, held** (no PRs until the work in progress is done, the user's decision):
  - `fix/figurative-varying-hival`;
  - `fix/replace-start`;
  - `fix/init-lock-timeout` (fixes issue #18);
  - `fix/size-numeric-constant`.

  The open PRs #11, #12 and #14 to #17 were being re-validated on x86_64 with a clean build per branch. Their earlier validations ran another branch's build: see the note in native-test-environment memory. Kyle must get a short note about it.

## Mac M1 session (2026-11-14)

- **Policy change (the user's decision, 2026-11-14).** Work only with guigra/AS4tainr. Publish nothing to kyleturley-cyber/AS4tainr: no issues, no PRs, no comments, no `upstream` remote pushes. This replaces "Follow the upstream PRs" (next steps, item 4), the `upstream` remote of "Get the code" and the note to Kyle above. Fixes go to this fork's `main` by PRs inside the fork.
- **Environment.** No Linux VM of its own: the podman machine (applehv, raised to 8 GiB, 6 CPUs) runs two containers:
  - `as4tainr-test-db`: PostgreSQL 16, `--shm-size=1g`, published on the Mac at 127.0.0.1:55432 (5432 is taken by a local PostgreSQL);
  - `as4tainr-dev`: Debian bookworm aarch64 with rustc 1.99.1, `--privileged`, `/QSYS.LIB`, the repository at /work, `target` in the volume `as4tainr-target`, on the database's network so 127.0.0.1:5432 works as in the scripts. Built from ~/.local/share/as4tainr-dev/Containerfile (outside Git).
  - Git does not work inside the container for worktrees (their `.git` points to the Mac path); cargo and the scripts do not need it.
  - With 6 GiB and CARGO_BUILD_JOBS=4 the kernel killed rustc building core's tests. Use 8 GiB and CARGO_BUILD_JOBS=3.
- **S2.25 validated on aarch64.** fmt PASS; clippy FAIL only on the upstream lint, PASS without it; native units 34/34 under interp and under rust; native-diff PASS (385 routines, 654 tests the same under auto). Evidence: docs/native/evidence/2026-11-14-s225-aarch64-mac.json.
- **Issue #18 reproduced here** on the first native-diff that ran tests (53 min hung). native-diff was run with `fix/init-lock-timeout` (79d5c03) applied without a commit, the user's decision; `native/engine` does not carry it. The fix was checked here: its test passes with it (3.2 s) and fails without it on main (34.6 s, the test's 30 s limit). It is proposed to this fork's main as https://github.com/guigra/AS4tainr/pull/1.
- **Not run here:** the full suite and release outside native-diff, the release build, native-e2e (no Playwright yet), golden (PUB400 not set up yet), anything on x86_64.
- **Next:** once PR #1 is merged, decide how `native/engine` picks it up (it touches core/src/catalog.rs, outside section 3's touch points). Then stage 2 block A, as listed above.

## Mac M1, S2.30 validation (2026-11-14)

Tasks from jsr-test's mac-tareas-2026-11-14.md, run at 8e212bd (S2.29 %EDITW, S2.30 subfields of data structure arrays) with the catalog::init lock fix (`git diff origin/main origin/fix/init-lock-timeout -- core/src/catalog.rs`) applied without a commit and removed before committing. There is no `upstream` remote here: origin/main is the same base (dd47e28).

| Step | Status |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --locked --all-targets -- -D warnings` | FAIL: upstream `chunks_exact_to_as_chunks` only (hostcmd.rs, hostdb.rs, hostifs.rs); PASS with that lint allowed |
| native_tripwire, native_units, native_engine | PASS: 1, 39, 2 |
| `AS4TAINR_RPG_ENGINE=rust scripts/native-diff.sh` | PASS: reference 658 results, 390 routines, level 1 identical, 658 the same under auto, 636 of 658 under rust (23 programs refused, none generated), demo the same; golden NOT_RUN (no PUB400 here) |
| `cargo test --locked --release` | PASS: 657 passed, 0 failed, 1 ignored (CARGO_BUILD_JOBS=2) |
| `cargo build --locked --release` | PASS: target/release/as4tainr SHA-256 83e217bb…694e |

save_changed_objects failed under interp and auto in native-diff (the known intermittent upstream failure) and passed in the release run. Evidence: docs/native/evidence/2026-11-14-s230-aarch64-mac.json; logs in the podman volume as4tainr-target, target/evidence/2026-11-14-s230-aarch64-mac.

Item 6 of the task file (an upstream PR for #18 from the x86 machine, then closing this fork's PR #1) was not done here: this session works only against guigra/AS4tainr (the user's decision), and PR #1 stays open until the user says otherwise.

## To look at (2026-11-14, x86_64)

- %STATUS after a call: on PUB400, %STATUS was 0 again after a call to a procedure that runs EXEC SQL (the first version of tools/golden/native40, saved in target/ibmi/golden as n40_stat before it was captured again). It is not known yet what sets it to 0 (the call, or the SQL statement) or whether the interpreter does the same. Capture both cases before changing anything.

## 2026-11-14, x86_64: S2.31-S2.34 and the x86_64 validation

- New steps: S2.31 %STATUS with no file, S2.32 %ABS, S2.33 %MAX/%MIN, S2.34 %CHECK/%CHECKR with no start (ADR 0006). PUB400 captures native40-43. New interpreter defect: %MAX/%MIN keep the chosen argument's scale; IBM i uses the arguments' greatest (fidelity.md).
- x86_64 at f89d678 (evidence/2026-11-14-s234-x86_64.json): native-diff PASS; release tests 632 passed. The 3 failures were leftovers in the shared test database, and they pass on a new one. Use a new database for evidence runs.
- aarch64 VM: S2.23-S2.28 and S2.29 development runs. native-diff PASS, 5250 end-to-end the same, OpenRPG 384/385 (test396: the interpreter's %SCANRPL panic).
- Upstream: nothing published. Five fix branches are validated and held, and there is an issue draft for %EDITW. The user decides when to publish (as4tainr-up-logs/revision-2026-11-14.md).
- Next: wait for the user's decisions: when to publish, the branch and worktree clean-up, and the next block (CALLP, then block B).

## 2026-11-15: the goal is a fully native engine (ADR 0007)

- The user's requirement: the `rust` engine must become 100% native, so the interpreter could be removed. Option A: behaviour that exists only in interp.rs is written again in our code (native_rt.rs and rpgrt), checked by the differential suite. Upstream code is not moved.
- Phase A: lower every instruction, with a dispatch count down to 0. Phase B: a runtime in rpgrt with no `Machine`.
- Next step: S2.35, native procedure calls within the module (ADR 0007, "S2.35"). First the PUB400 captures for its tests, then `enter_proc`/`leave_proc` in native_rt.rs, the IR for a call, and lowering.

## 2026-11-15 (later): S2.35-S2.43, the rules audit, coverage

- Done:
  - S2.35 native calls;
  - S2.36 DSPLY;
  - S2.37 no interpreter fallback;
  - S2.38 RETURN;
  - S2.39 indicators;
  - S2.40 EXSR;
  - S2.41 chains with no recursion, and IBM i's nesting limit of 100 with a refusal;
  - S2.42 named constants;
  - S2.43 the error path for program errors.
  
  See ADR 0007.
- Section 5 audit (d3ab6d7):
  - the generated code is now linted (native-diff's generated-lints step);
  - eprintln!, `as` casts and Index::index are fixed;
  - the refusals set is bounded.
  
  The user's decisions: errors stay anyhow (documented exception), and unwrap is allowed in tests.
- Section 7 before every commit: as4tainr-up-logs/section7.sh <snapshot> <tag> runs the full list on a `git stash create` snapshot, in its own worktree, on a new database, with the #18 fix applied without a commit.
- core/tests/native_coverage.rs:
  - each unit's count of interpreter instructions may not rise (a parity test cannot see a lowering that stops applying);
  - 100 nesting levels are generated, 101 are refused.
- Coverage of the OpenRPG corpus: 79.3% native at S2.39.
- PUB400 is unreachable for a few days (the IP is blocked, maybe by today's many captures). Space the captures out when it is back.
- Held for Kyle (publish at the end):
  - the five validated branches;
  - the %EDITW issue;
  - an issue and PR on expression nesting: over 100 levels compiles, and 20 000 aborts the process. The PR needs a PUB400 capture of the RNF ID.
- Next:
  - commit the chains block once s7-chain passes;
  - run section 7 on S2.42-S2.43 and commit;
  - rerun the VM bench with CARGO_BUILD_JOBS=1 (it was OOM-killed);
  - then %SUBST and %ELEM as targets, the remaining EVAL forms, and file I/O.
- Correction: f402885's message says AS4TAINR_NATIVE_WHY also writes CALLP and DSPLY shapes. That change was not in f402885; it is in the S2.46 commit.

## 2026-11-15 (night): state after S2.46, next is pointers

- Validated and pushed today, each with its full section 7 split between
  x86_64 and the VM:
  - 5b6f0c1 S2.41;
  - e2a1fb3 S2.42-S2.44;
  - f402885 S2.45;
  - d87059c S2.46.
  
  Corpus coverage is 72.8% native.
- AS4TAINR_NATIVE_WHY on the full suite: what is left is mostly pointers (EVAL targets of type pointer, %ADDR, pointer fields), calls through procedure pointers and C/CEE functions, file I/O and SQL.
- Plan for pointers:
  - S2.47:
    - pointer fields as operands (assignment, *NULL, comparisons; FigOp::Null);
    - %ADDR of global variables, and %ADDR(v:*DATA), with ptr_to written again in native_rt.rs and watched by the tripwire;
    - pointers kept out of numeric operands.
  - S2.48: BASED variables, read and written through `addr` with its checks.
  - Then pointer arithmetic and %ALLOC/%REALLOC/DEALLOC.
- PUB400: one connection at a time, captures batched (memory pub400-usage).

## 2026-11-16 (early): S2.47 done, next is BASED (S2.48)

- b0e8963 S2.47 (pointer fields, *NULL, %ADDR, room_for_based) was validated by a full section 7, split between x86_64 and the VM on snapshot 437b6f9, and pushed. Corpus coverage is 73.3% native.
- Two native-diff findings during S2.47, both fixed in b0e8963:
  - the pointer owner (an activation number) differs between runs, so the observation now clears it;
  - room_for_based was missing in the native store of a pointer.
- Lesson: never launch section 7 while a pre-check fails. The first S2.47 launch went out with a clippy error and had to be stopped.
- Next, S2.48: BASED variables read and written through the pointer (`addr_n` with its MCH3601/MCH0601 checks), with a PUB400 capture in one batch. Redo the nested-%ABS capture in the same batch.
