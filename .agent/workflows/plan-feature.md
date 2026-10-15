# Workflow: Plan a Feature

Use for work spanning multiple files, modules, backends, or security/runtime boundaries.

1. Read `AGENTS.md`, project context, relevant requirement, architecture docs, and ADRs.
2. Restate the user problem and define observable acceptance criteria.
3. List assumptions and unanswered questions; convert critical ones into a feasibility experiment rather than guessing.
4. Identify impacted modules/interfaces and the smallest end-to-end vertical slice.
5. Explain alternatives and why the proposed approach fits the existing architecture.
6. List security, privacy, license, host-platform, architecture, and failure-mode implications.
7. Specify tests, including negative/error cases and actual device/host requirements.
8. List documentation/ADR/support-matrix updates.
9. Present the plan for review before implementation when the change is significant.

## Plan output

- Objective
- Out of scope
- Assumptions / unknowns
- Proposed design
- Files/modules affected
- Risks and mitigations
- Tests and environments
- Acceptance criteria
- Documentation/ADR updates
