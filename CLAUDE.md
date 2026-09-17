# chictrip-axi - repo constitution for agents

Agent-first (AXI) command-line client for the chicTrip travel API,
written in Rust. User-facing usage lives in README.md and the docs site
(website/); this file is the development contract.

Workspace-wide procedure (routing, checklists, commit rules) comes from
`../../agent-os/INDEX.md`. This file only adds repo-specific facts.

## Quick facts

- Milestone: the write vertical slice (2026-09-18). Fifteen commands
  (`auth set/status/clear`, `trip list/create/view/add/remove/delete`,
  `tour list/view/copy`, `poi search/view`, `location search`) plus the
  home view, over
  the shared HTTP/output/error/auth layers. Designed in
  `docs/design/axi-interface.md`, which also lists what was left out.
  Verified live the same day against api.chictrip.com.tw with a member
  token, `trip create` included once it sent a real system cover id:
  chicTrip's `TravelSchedule/AddV2` answers the generic `002 A non-empty
  request body is required` when `CoverMediaId` or the label id is
  empty or `LocationKey[]` is missing (`docs/api/protocol.md`). Next
  milestone:
  `skills/chictrip-axi/SKILL.md` generated from the home view's command
  index with a `--check` gate, and `setup hooks`.
- Published 2026-09-17: PUBLIC GitHub repo `qq88976321/chictrip-axi`
  (remote `origin`, https), pushed by the user. VERIFIED the same day
  via the Actions API: the ci workflow is green on master and the
  release workflow published v0.0.1 (tag + GitHub Release with assets).
  Pages is enabled and the docs site is live at
  https://qq88976321.github.io/chictrip-axi/ (a pages run that starts
  before Pages is switched to "GitHub Actions" fails at
  `actions/configure-pages`; re-run it). Development on branch `master`
  (cargo-release `allow-branch = ["master"]`). The version line starts
  at 0.0.1 (first tag v0.0.1, 2026-09-17); later releases are
  `just release patch|minor|major` and tags are SSH-signed
  (release.toml `sign-tag`). Pushing is USER-ONLY.
- Toolchain: `cargo`/rustc 1.97.1 locally. Rust edition 2024, MSRV =
  1.85 (the edition floor). The CI msrv job stays commented until a
  1.85 build is verified.
- Dependencies (do not add more without user confirmation): clap
  (derive), anyhow, thiserror, ureq 3 (`json` feature; sync, rustls, so
  the musl release build keeps working), serde (derive), serde_json,
  toon-format 0.5 (`default-features = false`: the default `cli`
  feature drags in a TUI stack). No dev-dependencies: the integration
  test uses `std::net::TcpListener` and `std::process::Command` with
  `env!("CARGO_BIN_EXE_chictrip-axi")`. Dev tools installed
  out-of-band (NOT Cargo deps): cargo-release, git-cliff (CHANGELOG.md,
  config in cliff.toml), Zensical (docs, via uvx; inside the sandbox it
  needs `UV_TOOL_DIR`/`UV_CACHE_DIR` pointed at `$TMPDIR`), shellcheck
  (install.sh).
- AXI design skill: installed (2026-09-17) with
  `npx skills add kunchenguid/axi`. The canonical copy is
  `.agents/skills/axi/SKILL.md`, `.claude/skills/axi` symlinks to it,
  and `skills-lock.json` pins the upstream revision. Load the `axi`
  skill before building, changing, or reviewing any command. Never
  hand-edit SKILL.md: refresh it with the same command (USER-ONLY, the
  sandbox blocks writes under .claude/skills) and commit all three
  paths together. https://axi.md/ is the human-readable reference.

## Commands

```
just gate      # THE quality gate: fmt --check, clippy -D warnings,
               #   cargo test, cargo build --release. Run before every
               #   commit; all four must pass.
just build     # debug build
just test      # unit tests only
just lint-sh   # shellcheck install.sh and scripts/test-install.sh.
               #   Deliberately NOT part of `gate` (would silently skip
               #   where shellcheck is missing); CI enforces it.
just test-install # offline install.sh round trip against a fake
                  #   release tree over file://. Needs
                  #   `rustup target add x86_64-unknown-linux-musl`.
just run -- ARGS  # run the binary, e.g. `just run -- --help`
just release LEVEL # cargo-release: gate, bump, regen CHANGELOG.md,
                   #   commit, tag. USER-ONLY.
just site-build / site-serve  # Zensical docs site (website/)
```

## Module map (src/) - lib + thin bin

- main.rs      entrypoint: parse argv (a clap rejection becomes the
               usage error on stdout with the command's flags inline),
               dispatch, render, exit
- lib.rs       module list; the binary is thin so the pure parts are
               unit-tested without spawning a process
- cli.rs       clap tree (Cli, GlobalArgs, one Command enum per noun),
               `Context` (global flags -> Client), dispatch,
               `flags_for_argv` for the usage error
- output.rs    ordered `Document`/`Table`, truncation, `--fields`
               projection, render_toon / render_json
- error.rs     `AxiError { code, message, help, flags, request_id }`
               and its exit code; `ErrorCode` is the stable vocabulary
- auth.rs      auth file (0600), token precedence, JWT decoding,
               GUEST_TOKEN with its provenance
- datetime.rs  UTC calendar maths (no date crate for 60 lines)
- api/mod.rs   Client: envelope parsing, apiStatus -> AxiError,
               003 -> refresh -> replay once
- api/types.rs all-optional serde structs, unknown fields ignored
- api/trips.rs the multi-call recipes and the updateTime chain
- commands/    mod.rs holds what more than one noun needs (validation,
               value conversions, the trip and stop tables); auth.rs,
               home.rs, location.rs, poi.rs, tour.rs, trip.rs are one noun each and
               return a Document, never formatting or error text
- tests/cli.rs the binary against a `TcpListener` fixture server, with
               trimmed captures under tests/fixtures/

## CLI design: AXI (agent-ergonomic)

Every new command and any output-shape change follows the AXI design
principles (https://axi.md/). Token budget is a primary constraint;
reliability is the other. Concretely:

- Efficiency: TOON on stdout (keep internal logic in JSON-shaped
  structs, convert at the output boundary); `--json` as the escape
  hatch; default list schemas of 3-4 fields with `--full` for the
  rest; verbose fields truncated with a size hint and the command
  that returns the whole thing.
- Robustness: list output carries pre-computed aggregates (`count: 30
  of 847 total`); an empty result prints a definitive `0 results`
  line, never silence; errors are structured on stdout with stable
  codes and inline the valid flags so the agent can self-correct in
  one turn; exit codes are 0 success (including no-ops), 1 error, 2
  usage; validate flags before any network call; never leak an
  upstream body or a stack trace; mutations are idempotent and nothing
  prompts interactively.
- Discoverability: bare invocation shows live content (the home view),
  not help text; attach contextual next-step commands after output;
  every subcommand keeps a concise `--help` with defaults and 2-3
  examples; `-V`/`--version` print the bare version and exit 0 before
  anything else loads.
- Integration (next milestone): `skills/chictrip-axi/SKILL.md` is
  generated from the same content as the home view (static: strip
  live state) and a `--check` gate fails CI when the committed skill
  drifts from the CLI; `setup hooks` installs a SessionStart hook on
  explicit request only, never on first run.
- Implement these once in a shared output/error layer; never
  re-implement the pattern inside a command module.
- The toon-format encoder quotes any value containing `-`, `:`, or a
  brace, so UUIDs, clock times, and most `help` lines come out quoted.
  That is the encoder's rule, not ours; do not hand-roll around it.

## Distribution

- install.sh is hosted as a RELEASE ASSET
  (`releases/latest/download/install.sh`), not from raw.githubusercontent
  or Pages, so the installer a user runs can never drift ahead of the
  assets it knows how to name. It resolves "latest" at runtime via the
  `releases/latest` redirect (no REST API, no rate limit); `--version`
  pins the binary, a pinned asset URL pins the script.
- Linux release binaries target MUSL ONLY (x86_64 + aarch64), so one
  build per arch runs on any distribution. Do NOT switch back to
  `*-linux-gnu`: on ubuntu-latest that pins the glibc floor to the
  runner image, which silently breaks older distributions. If musl
  ever fails to build, the fallback is cargo-zigbuild with
  `x86_64-unknown-linux-gnu.2.17`, not plain gnu.
- install.sh hardcodes the asset contract: `chictrip-axi-<target>.tar.gz`
  with the binary at the archive root, plus a sidecar named
  `chictrip-axi-<target>.sha256` (NOT `...tar.gz.sha256`) whose line
  names the tarball with no path. Changing release.yml's matrix, archive
  name, or checksum algorithm means changing install.sh and both
  platform tables (README, website/docs/install.md); `just test-install`
  catches the drift without cutting a release.
- Docs site: website/ (Zensical, `just site-build`), deployed by
  `.github/workflows/pages.yml` on pushes touching website/ (verified
  live 2026-09-17). Pages settings are USER-ONLY. The Zensical pin lives
  in the justfile and in pages.yml; bump both together.

## Conventions

- Conventional Commits, ASCII-only edits, MIT license.
- Commit scopes follow the module/area: feat(cli), feat(output),
  feat(<api-area>), feat(dist), build, ci, docs, docs(site), test.
- Commit after each coherent, self-contained change; never batch
  unrelated changes.
- Tests must not depend on the machine: no network, no git config, no
  HOME contents. CI runs with none of them.
- NOTE: `git add -A` fails here (the sandbox injects character-device
  dotfiles in the repo root); always stage explicit paths.
