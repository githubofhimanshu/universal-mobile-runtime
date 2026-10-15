# AI Agent Guidance

This directory is the project's **canonical, tool-neutral guidance** for vibe coding and agent-assisted engineering. It is not application runtime code.

## Structure

- `context/project-context.md` — compact, stable project context to include in agent sessions.
- `rules/` — always-on principles for scope, architecture, security, and testing.
- `workflows/` — repeatable playbooks for planning, implementation, debugging, and review.
- `templates/` — structure for feature specs, bugs, and architecture decisions.

## How to use it

1. Start each major task by reading root `AGENTS.md` and `context/project-context.md`.
2. Use the relevant workflow rather than asking an agent to “build everything.”
3. Give the agent one testable change at a time, with explicit files/modules and acceptance criteria.
4. Ask the agent to plan before coding when the change crosses module boundaries or touches a runtime/security boundary.
5. Review code and run tests yourself; agent-generated code is not automatically correct or safe.
6. Update requirements, ADRs, and the compatibility matrix when decisions or support claims change.

## Tool compatibility

Different coding tools discover instructions differently. Keep these Markdown documents canonical, then create small provider-specific adapters only when a chosen IDE/agent needs them. Do not copy entire rules into multiple files and allow them to drift. Link to the canonical rules instead when supported.

## Suggested task prompt

> Read `AGENTS.md`, `.agent/context/project-context.md`, the relevant `.agent/rules/`, and the related docs/ADRs. First summarize the requirement, boundaries, risks, files likely to change, and tests. Do not edit files until the plan is coherent. Implement only the stated vertical slice, run available tests, report actual results, and list unsupported cases. Do not pretend that stubs, mocks, package parsing, or a successful build prove runtime compatibility.
