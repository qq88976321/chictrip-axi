# Changelog

All notable changes to this project are documented in this file.
The format is based on Keep a Changelog, and this project adheres to
Conventional Commits.

## [0.1.1] - 2026-09-29

### Features

- **api**: Add PUT transport
- **api**: Decode edit info, route lists, custom times, and day traffic
- **cli**: Add the clock, minutes, and point validators
- **trip**: Show notes, legs, and categories with trip view --full
- **trip**: Insert at the start or after a stop with trip add
- **trip**: Add trip edit for a stop's times and category and name
- **trip**: Add trip note for the trip and its stops
- **trip**: Add trip leg for how a stop is reached
- **trip**: Add trip traffic for a day's default mode
- **trip**: Add trip move to reorder a day
- **poi**: Add poi create for places chicTrip does not list
- **setup**: Teach the skill and the description the editing commands

### Refactor

- **api**: Share the update-time retry between add and remove

### Documentation

- **api**: Record the edit, note, leg, reorder, copy, and custom poi flows
- **design**: Record milestone 4 and what the live run corrected
- Document the twenty-four commands in the README
- Bring the repo constitution up to milestone 4
- **site**: Add the editing commands to the usage and index pages

### Testing

- Refuse the new member commands as a guest before any request

## [0.1.0] - 2026-09-18

### Features

- **output**: Add the TOON document layer and the error contract
- **auth**: Resolve tokens and keep the member session on disk
- **api**: Add the chicTrip client, response types, and trip recipes
- **cli**: Add the command tree, the home view, and auth commands
- **poi**: Add poi search and poi view
- **tour**: Add tour list, tour view, and tour copy
- **trip**: Add the write vertical slice over my trips
- **location**: Add location search for trip create keys
- **trip**: Add trip preview for any itinerary by id
- **setup**: Generate the agent skill from the command index
- **setup**: Install the Claude Code SessionStart hook on request

### Documentation

- Record CI, release, and Pages status
- **api**: Record the chicTrip API exploration
- **design**: Draft the AXI command interface
- **api**: Record the write flows and the shared-trip preview
- **design**: Re-scope milestone 2 to writing trips
- **api**: Record what the write flows actually answer
- Document the fourteen commands in the README
- **site**: Rewrite the usage page for the real command set
- Bring the repo constitution up to the write milestone
- **design**: Record what the live run corrected
- Document trip preview and the two agent integrations
- **site**: Add trip preview and the agent integrations
- **design**: Record milestone 3 and the TOON quoting evaluation
- Bring the repo constitution up to milestone 3
- **api**: Record what Preview answers for unknown and deleted trips

### Testing

- Drive the binary against a fixture server

### Build System

- **release**: Sign release tags
- Add the http, serde, and toon dependencies

### Continuous Integration

- **pages**: Mark the Pages deployment verified
- Gate the committed skill with setup skill --check

## [0.0.1] - 2026-09-17

### Features

- **dist**: Add install.sh

### Documentation

- Add the repo constitution (CLAUDE.md)
- Add README and the release runbook
- **site**: Add the Zensical docs site
- Record the published GitHub remote

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
