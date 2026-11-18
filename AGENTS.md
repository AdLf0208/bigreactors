# Working on a native Horizon mod

> **This port is an SDK package (2026-11-18).** The retired `.hnmod` format described
> below does not apply: read `../_shared/AGENT_MOD_STRUCTURE.md` **§0** and this port's
> `horizon-mod.toml` instead. There is no `mixins/`, no per-hook helper and no
> `horizon-mixin.toml`; `src/runtime.rs` stays byte-identical and `src/lib.rs` is the
> one `impl Hooks`. The SDK ladder is `cargo test` → `clippy` → `cargo horizon-mod
> build/check/fits` directly in the port (see `README.md` and `notes.md`).

> **Build slots (v2, 2026-11-09):** every build/test/e2e goes through the
> campaign runner (`ports/tools/build-slot.sh` from the campaign root,
> `../tools/build-slot.sh` from this port). It **blocks in the kernel** until a
> slot is free (no polling), sets `CARGO_BUILD_JOBS` per step, journals every
> lease, and exits **75** on a wait timeout. Read `../_shared/build-slots-usage.md`
> before running anything; observe with `status`/`watch`/`events`/`stats`.
> Never poll, never background a build, never pass `--target-dir`.

Read README.md and compatibility.json before editing, and read the campaign's
structure guide `../_shared/AGENT_MOD_STRUCTURE.md`: it explains the two formats
(`.hmod` content, `.hnmod` code, one file per registry project), the four source
roles (pure `runtime.rs`, glue `support.rs`, one private function per helper
file, the `horizon-mixin.toml` plan), the `[[module]]`/`[[injection]]` model,
packaging, and the platform limits already measured. Keep this repository
separate from Horizon and horizon-mixins. Only export committed pinned Horizon
sources into ignored target trees; never modify the original checkout or player
resources. Do not commit binaries, exported sources or Minecraft assets.

The Cargo library and injected runtime module share runtime.rs. Keep helper
functions small and gameplay logic testable there. All AST changes go through
horizon-mixins plan/apply; do not add string-based Rust source rewriting.

Validate with cargo fmt, clippy with warnings denied, cargo test and
python3 tools/test_template.py. For bundle/compatibility changes also run
python3 tools/test_stage.py and python3 tools/horizon.py check/test against the
pinned source export. Keep validation claims and supported revisions honest.
