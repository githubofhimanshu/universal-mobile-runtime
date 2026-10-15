# Workflow: Review a Change

Review for correctness and user value, then for security and maintainability.

## Review checklist

- Does the change satisfy the stated requirement and no unrelated requirement?
- Are platform-specific assumptions correct and isolated?
- Are backend interfaces minimal and error handling explicit?
- Is any external code/dependency added, and have version/license/maintenance risks been checked?
- Are package/file/path and IPC inputs validated?
- Are timeouts, cancellation, shutdown, and process cleanup correct?
- Can a backend error crash or hang the UI?
- Do tests cover normal, failure, unsupported, and malformed-input cases?
- Are claims supported by test evidence on the stated host/CPU/backend?
- Are secrets, app content, or private diagnostics accidentally logged or committed?
- Are docs, ADRs, and compatibility records accurate?

## Review output

Prioritize findings by severity. Give file/line references where available and distinguish blockers from non-blocking suggestions. Do not approve based only on a successful build.
