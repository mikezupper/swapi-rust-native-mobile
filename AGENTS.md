# Project instructions

Build a visually polished SWAPI explorer using SwiftUI/Swift, Jetpack
Compose/Kotlin, and one shared Rust core exposed through Mozilla UniFFI.
Use https://swapi.dev/api/. Holocron is a provisional name.

## Repository map

- [README](README.md): entry points and commands that currently exist.
- [Product](docs/product.md): confirmed requirements and proposed scope.
- [Architecture](ARCHITECTURE.md): dependency direction and state ownership.
- [Design](docs/design-system.md): visual identity and native UX expectations.
- [Environment](docs/development.md): local tools and macOS host strategy.
- [Testing](docs/testing.md): required evidence and scenario coverage.
- [Harness](docs/harness.md): rationale and source references.

## Work tracking

Read [.agents/skills/beads/SKILL.md](.agents/skills/beads/SKILL.md) for multi-step
work. Run `bd prime` at session start and after compaction, then inspect
`bd ready --json` and `bd list --status in_progress --json`.

Create a descriptive bead with acceptance criteria before implementation.
Read it, claim it with `bd update <id> --claim`, and keep findings in `bd note`.
Dependencies mean that the dependent needs the blocker. Close completed work
with observed evidence. File discovered work with a `discovered-from` edge.
Use `--json` for parsed output. Never invoke interactive `bd edit`.

Beads owns progress, blockers, and task status. Documentation owns durable
specifications and decisions. Do not create competing task lists or plan files.
The Dolt database is local until a remote is configured; JSONL is not sync.

## Boundaries

- Rust owns SWAPI networking, parsing, application state, and request policy.
- Native adapters own thread delivery and subscriptions; native views own
  layout, navigation, accessibility, gestures, and animation frames.
- Keep the UniFFI boundary typed, small, and independently verifiable.
- Generate Swift/Kotlin bindings and design constants reproducibly. Change
  their source definitions instead of editing generated files.
- Pin the binding generator and Rust UniFFI library to matching versions.
- Keep ordinary tests independent of the live SWAPI service.
- Implement one complete native feature before multiplying screens.
- Keep visual polish, accessibility, and lifecycle handling in feature scope.

## Execution and evidence

Use existing commands from README and docs; do not invent successful builds.
Before native UI implementation, read the design guidance. For UI changes,
inspect actual rendered native screens and verify the affected interactions.
Distinguish mockups, simulator evidence, and real-device evidence.

Run `python3 scripts/check_docs.py` after documentation edits. Run the
appropriate Rust/native checks once those projects exist. Report commands
run and material gaps. Do not mark unavailable platform checks as passed.

The user prefers an OSX-KVM experiment first, with their Mac over SSH as the
fallback. A booted VM alone does not satisfy the iOS environment milestone.
Keep VM disks and SDK downloads outside this repository.

External publication, source remotes, and issue synchronization must follow
the user's instructions. Preserve existing work and avoid unrelated changes.
