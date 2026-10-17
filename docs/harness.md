# Harness approach

Apply the user's supplied engineering guide by making intent, boundaries, and
feedback accessible inside this repository. Keep the process proportional to
a small native mobile app.

The source is Ryan Lopopolo's
[Harness engineering](https://openai.com/index/harness-engineering/) article,
provided by the user as `~/Downloads/openai-harness-engineer.md`.

## Repository knowledge

[AGENTS.md](../AGENTS.md) is a short entry point to focused documents. Product,
architecture, and design decisions live beside the source. Working commands
and verification evidence make those decisions actionable in future sessions.

Beads replaces the article's execution-plan tracking conventions for this
project. It owns acceptance criteria, dependencies, discoveries, progress,
and handoffs. Markdown documents describe the system, without duplicating
issue status. Read the user's local Beads skill through AGENTS.md.

## Feedback loops

Build one feature through Rust, UniFFI, and both native interfaces early.
Expose fixture scenarios, native screenshots, test reports, and structured
logs to the agent. Repeated mistakes should lead to a useful check or a clearer
boundary, rather than ever-longer instructions.

Introduce enforcement with the corresponding code: boundary checks for native
networking, reproducible bindings and tokens, formatting/static analysis, and
focused behavioral tests. Keep live API availability outside deterministic
gates. Prefer a small set of observed checks over untested automation.

## Review

Treat visual design and performance as part of feature completion. A screenshot
alone does not establish interaction quality; inspect navigation, motion,
failure recovery, and accessibility in the native app. Update durable guidance
when user feedback changes the standard. Record remaining work in Beads.

The article's large-scale observability and multi-agent operations are context,
not requirements to reproduce. Add tooling when it removes an observed obstacle.
