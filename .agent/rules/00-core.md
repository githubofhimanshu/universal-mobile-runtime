# Core Agent Rules

- Read root `AGENTS.md` and the relevant documents before changing code.
- Preserve the stated Round 1 goal; do not quietly redefine it.
- Separate confirmed facts, assumptions, proposals, and unknowns in plans and reports.
- Prefer the smallest complete vertical slice over speculative broad scaffolding.
- Do not invent completed functionality, test results, support claims, or benchmark numbers.
- Do not create new dependencies without explaining why they are needed and checking maintenance, platform fit, security, and license.
- Keep public APIs and abstractions minimal until proven by a real caller/backend.
- If requirements conflict, stop and report the conflict rather than silently choosing a convenient interpretation.
- For meaningful architecture changes, propose an ADR and identify migration impact.
- At completion, summarize files changed, tests actually run, results, known limitations, and follow-up work.
