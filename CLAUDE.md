# chictrip-axi - repo constitution for agents

Agent-first (AXI) command-line client for the chicTrip travel API,
written in Rust. User-facing usage lives in README.md and the docs site
(website/); this file is the development contract.

Workspace-wide procedure (routing, checklists, commit rules) comes from
`../../agent-os/INDEX.md`. This file only adds repo-specific facts.

## Quick facts

- Milestone: infrastructure only (2026-09-17). The binary has no API
  commands yet: bare invocation prints the home view; `--help`,
  `--version`, and the 0/1/2 exit-code mapping exist. API commands,
  the TOON output layer, `skills/chictrip-axi/SKILL.md`, and
  `setup hooks` are the next milestone.
- Published 2026-09-17: PUBLIC GitHub repo `qq88976321/chictrip-axi`
  (remote `origin`, https), pushed by the user. VERIFIED the same day
  via the Actions API: the ci workflow is green on master and the
  release workflow published v0.0.1 (tag + GitHub Release with assets).
  Pages is enabled, but its first two runs failed at
  `actions/configure-pages` because they ran before Pages was switched
  to "GitHub Actions"; the site stays 404 until the workflow is re-run
  (`gh workflow run pages.yml`, USER-ONLY). Delete the STATUS block in
  pages.yml once a deployment succeeds. Development on branch `master`
  (cargo-release `allow-branch = ["master"]`). The version line starts
  at 0.0.1 (first tag v0.0.1, 2026-09-17); later releases are
  `just release patch|minor|major` and tags are SSH-signed
  (release.toml `sign-tag`). Pushing is USER-ONLY.
- Toolchain: `cargo`/rustc 1.97.1 locally. Rust edition 2024, MSRV =
  1.85 (the edition floor). The CI msrv job stays commented until a
  1.85 build is verified.
- Dependencies (do not add more without user confirmation): clap
  (derive), anyhow, thiserror. Scouted for the next milestone but NOT
  added: an HTTP client, serde, a TOON serializer (`toon-format` 0.5 or
  `serde_toon_format` 0.1 on crates.io). Dev tools installed
  out-of-band (NOT Cargo deps): cargo-release, git-cliff (CHANGELOG.md,
  config in cliff.toml), Zensical (docs, via uvx), shellcheck
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

- main.rs   entrypoint: parse CLI, print the home view, map errors to
            exit codes (AxiError::exit_code, otherwise 1)
- lib.rs    module list; the binary is thin so the pure parts are
            unit-tested without spawning a process
- cli.rs    clap `Cli` (no subcommands yet) + `home_view()` (pure)
- error.rs  thiserror `AxiError`: Usage -> exit 2, Failed -> exit 1

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
  `.github/workflows/pages.yml` on pushes touching website/. Enabling
  Pages (Settings -> Pages -> Source: GitHub Actions) is USER-ONLY; the
  workflow carries a STATUS: UNVERIFIED block until the first deployment
  succeeds. The Zensical pin lives in the justfile and in pages.yml;
  bump both together.

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
