# Native RPG backend: handoff

Last update: 2026-11-14. The branch is `native/engine`, stacked on `native/runtime-extraction`, both rebased on upstream main dd47e28. Nothing of these branches is pushed. The previous tip is kept as `native/engine-pre-rebase2`.

## State

- **Stage 1 is complete.** Every program runs natively under `AS4TAINR_RPG_ENGINE=rust|auto`, with the interpreter as the reference.
- **Stage 2, S2.1 to S2.20, is committed** (ADR 0006). S2.21 (array elements with field indexes) is written and being validated. Lowered:
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

1. Validate S2.21 (aarch64 devrun21), then commit it.
2. Block A of stage 2: %EDITC/%EDITW, %REPLACE, %CHECK, %SCANRPL, %LOOKUP, figurative constants, whole data structure assignment, CALLP. EXSR without run_code_at needs the user's approval, because it touches interp.rs.
3. Priority 3:
   - a production image built with native-tools;
   - OrderDesk under `rust`;
   - gate3 (three clerks at once) under `rust`.
4. Follow the upstream PRs: answer reviews, and rebase #12 after #11.
