# Testing and Evidence Rules

- Tests must state which host OS, host CPU, backend version, and app/test fixture they cover.
- A successful compile proves compilation only—not runtime compatibility.
- Package parsing proves parsing only—not application launch.
- Mocks can test interfaces but must not be reported as proof of real Android/iOS execution.
- Add tests for failure paths, cancellation, timeouts, cleanup, and unsupported capabilities.
- Record manual tests with reproducible steps, machine/OS details, results, and date.
- Do not add unsupported combinations to the verified matrix.
- Avoid committing third-party app fixtures unless the license explicitly allows it; prefer project-owned or synthetic fixtures.
- Update support matrix and user-facing documentation when compatibility changes.
