# AGENTS.md

Guide for AI agents working in the `carapace-spec-clap` repository.

## Project Overview

A Rust library that generates [carapace-spec](https://github.com/carapace-sh/carapace-spec) YAML from [clap](https://github.com/clap-rs/clap) `Command` definitions. It implements `clap_complete::Generator` so consumers call `clap_complete::generate(Spec, &mut cmd, name, &mut stdout)` to emit a YAML spec consumable by [carapace](https://carapace.sh) shell completion.

## Commands

```sh
cargo build                         # build the library
cargo test                          # run snapshot tests (primary test suite)
cargo run --example carapace_spec   # print example spec to stdout
cargo run --example git             # print a git-like spec to stdout
```

CI (`.github/workflows/rust.yml`) runs `cargo build --verbose` and `cargo test --verbose`. There is no separate lint job.

## Release / Versioning Gotcha

`Cargo.toml` ships with `version = "0.1.0-PLACEHOLDER"`. The real version is substituted at release time by the CI workflow using `sed` on tag pushes (`refs/tags/v*`), which also runs `cargo publish --allow-dirty`. Do **not** commit a real version number into `Cargo.toml` — keep the `0.1.0-PLACEHOLDER` token. GoReleaser is configured but builds are skipped (`.goreleaser.yml` has `builds: - skip: true`); release artifacts come from the crate publish only.

## Architecture

The crate is intentionally small — a single implementation module plus a thin `lib.rs` re-export:

- `src/lib.rs` — declares `mod carapace_spec` and re-exports `Spec`.
- `src/carapace_spec.rs` — all logic. Public API surface is the `Spec` unit struct (implements `clap_complete::Generator`) plus serializable model structs (`Command`, `Completion`, `Documentation`).

### Data flow

1. `Spec::generate(&self, cmd: &clap::Command, buf: &mut dyn Write)` is the entry point invoked by `clap_complete::generate`.
2. `command_for(cmd)` recursively walks the clap `Command` tree into the crate's own `Command` model. Subcommands that are hidden (`is_hide_set`) are dropped at this step.
3. `filter_inherited_flags(&mut command, &mut inherited)` runs **after** the tree is built and strips `persistentflags` from children when an ancestor already declared them, so each level only lists the persistent flags it newly introduces. It uses a borrow-then-restore pattern: flags added at this level are inserted into the `inherited` map before recursing into children and `shift_remove`-ed afterward, so siblings don't inherit each other's persistent flags.
4. Output is serialized with `yaml_serde` (not `serde_yaml`) and prefixed with a fixed schema-header comment pointing at `https://carapace.sh/schemas/command.json`.

### Model → YAML mapping (non-obvious)

The YAML shape is governed by `serde` attributes on the model structs and is what the carapace spec schema expects. Several conventions are easy to break:

- **Flag signatures** are built by `flag_signature`: `-s, --long` when both exist, `--long` only, or `-s` only. Positional args are never flags.
- **Modifiers** appended to the signature string (`modifier_for`):
  - `=` — flag takes values, no `require_equals`.
  - `?` — flag takes values **and** `require_equals(true)` is set.
  - `!` — flag is required (`is_required_set`).
  - `&` — flag is **hidden** and takes values. For hidden flags the code ensures a `&` is present even on aliases (`hidden_modifier`).
  - `*` — action is `Append` or `Count` (repeatable).
  - Order of these matters and is **not** intuitive: `&` is pushed first, then `!`, then `=`/`?`, then `*` — so a fully-decorated hidden required repeatable flag with `require_equals` yields `&!?*`. See `modifier_for` for exact sequencing.
- **Persistent vs. local flags**: `flags_for(cmd, persistent)` splits on `a.is_global_set() == persistent`. Global flags become `persistentflags`; non-global become `flags`.
- **Flag ordering**: `sorted_args` sorts by `(get_long(), get_short())`. The YAML map preserves insertion order because `flags`/`persistentflags` use `indexmap::IndexMap`, **not** `BTreeMap`/`HashMap`. If you change the map type you will break snapshot tests and the expected spec ordering.
- **Flag values — plain vs. extended notation** (carapace-spec v1.8.0): each flag value is a `FlagValue` enum serialized with `#[serde(untagged)]`. The generator emits:
  - `FlagValue::Plain(String)` — the common case: just the description string. Used when the flag has no `nargs` and no default.
  - `FlagValue::Extended(ExtendedFlag)` — a map with `description`, `nargs`, and `default` fields. Triggered when `nargs != 0` **or** `default` is non-empty (matching carapace-spec's `Extended` struct and its marshal trigger `f.Nargs != 0 || f.Default != ""`).
  - Selection happens in `flag_value_for`. `default_for` guards on `arg.get_action().takes_values()` so only value-taking actions (`Set`/`Append`) can produce a `default`; boolean/count/help/version actions don't expose defaults through clap's `get_default_values()` anyway, but the guard keeps the intent explicit.
  - `nargs_for` maps clap's `num_args` range to the spec's integer: fixed `n != 1` → `n`, unbounded (`max_values() == usize::MAX`, e.g. `num_args(1..)`) → `-1`, otherwise `0`.
  - `default_for` joins multiple default values with `,`, matching how pflag's `stringSliceValue.Set` splits on comma for repeatable flags in carapace-spec. Only emitted for value-taking flags. Note: a non-repeatable `Set` flag with multiple defaults is an uncommon edge case — carapace-spec's `Default` is a single string and its `String` flag does not split on comma, so the joined value is kept verbatim there.
- **Completion sources**, in priority order within each entry: `action_for(value_hint)` first, then `values_for(arg)` (possible values from the value parser). Possible values are rendered as `name\thelp` when help text exists (tab-separated).
- **`positional` vs `positionalany`**: a positional whose `num_args().max_values() == usize::MAX` (variadic, e.g. `num_args(1..)`) goes to `positionalany`; all others go to `positional`. Empty positional entries are filtered out.
- **ValueHint → carapace macro** mapping lives in `action_for`. Only a subset is mapped: `AnyPath`/`FilePath`/`ExecutablePath` → `$files`, `DirPath` → `$directories`, `CommandName`/`CommandString` → `[$executables, $files]`, `Username` → `$carapace.os.Users`, `Hostname` → `$carapace.net.Hosts`. `Url`, `EmailAddress`, `Other`, `Unknown`, and `CommandWithArguments` currently map to **no action** (empty vec) — adding support for them is a known extension point.
- **Aliases**: visible aliases become regular flag entries with the same help; non-visible (hidden) aliases get the `&` hidden modifier. Visible short aliases are inserted with their own signature; non-visible short aliases are only added if not already present under the visible form. `flag_completions_for` mirrors this for completions keyed by `arg_key` (long, else short) and all aliases.

### Serialization skip rules

Every field on `Command`/`Completion`/`Documentation` uses `#[serde(skip_serializing_if = …)]` against `is_empty`/`is_default`/`Vec::is_empty`/`Map::is_empty`. When adding a field, follow this pattern or empty values will leak into the YAML and break snapshots. `Command` also has custom `is_empty` impls on `Completion` and `Documentation` (not derived). The `ExtendedFlag` struct follows the same convention (`String::is_empty` for `description`/`default`, a custom `is_zero` for the `nargs` i64).

## Testing

Tests are **snapshot tests** using `snapbox` with the `diff` feature.

- `tests/carapace_spec.rs` — one `#[test]` per fixture, each calling `common::assert_matches` against a file in `tests/snapshots/*.yaml`.
- `tests/common.rs` — builds clap `Command` fixtures (`basic_command`, `feature_sample_command`, `special_commands_command`, `quoting_command`, `aliases_command`, `sub_subcommands_command`, `value_hint_command`, `extended_notation_command`) and defines `assert_matches`. Marked `#![allow(dead_code)]` because helpers are shared across test modules.
- Snapshots live in `tests/snapshots/` and are the source of truth. Each snapshot begins with the schema-header comment line.

### Updating snapshots

`snapbox` is configured with `action_env(snapbox::assert::DEFAULT_ACTION_ENV)`, which resolves to the **`SNAPSHOTS`** environment variable (not `SNAPBOX_ACTION`). To regenerate snapshots after an intentional output change:

```sh
SNAPSHOTS=overwrite cargo test
```

Without that env var, mismatches fail the test with a diff. **Do not hand-edit snapshots** — regenerate them so the YAML exactly matches what the generator emits (including ordering and quoting).

### Adding a new fixture

1. Add a `pub fn <name>_command(name: &'static str) -> clap::Command` in `tests/common.rs`.
2. Add a `#[test] fn <name>()` in `tests/carapace_spec.rs` pointing at `snapshots/<name>.yaml`.
3. Run with `SNAPSHOTS=overwrite cargo test` to generate the initial snapshot, then review the diff before committing.

## Conventions

- Rust edition 2021. `clap` is imported with `default-features = false, features = ["std"]` — keep it minimal; don't introduce dependencies on clap features that aren't enabled.
- The crate is a library only (`[lib]` with no explicit name, no `[[bin]]`). Executables live under `examples/` and are illustrative, not shipped.
- `indexmap` is used specifically for insertion-order-preserving maps — do not swap it for `std::collections::HashMap` or `BTreeMap`.
- `yaml_serde` (the `yaml-serde` crate) is the serializer, not `serde_yaml`. Output formatting depends on its emitter behavior.
- No comments are used in source beyond the single `#![allow(dead_code)]` in `tests/common.rs`; match that style.
