# Changelog

All notable changes to this project are documented in this file.
The format is based on Keep a Changelog, and this project adheres to
Conventional Commits.

## [Unreleased]

### Features

- **dist**: Add install.sh

### Documentation

- Add the repo constitution (CLAUDE.md)
- Add README and the release runbook
- **site**: Add the Zensical docs site

### Testing

- **dist**: Add an offline round trip for install.sh

### Build System

- Add the justfile quality gate
- **release**: Configure cargo-release and git-cliff
- Start the version line at 0.0.1

### Continuous Integration

- Add the GitHub Actions gate
- **release**: Ship musl and darwin binaries with install.sh
- **pages**: Deploy the docs site to GitHub Pages

### Miscellaneous

- Scaffold the chictrip-axi crate
- Install the AXI design skill
