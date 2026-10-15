# Security Rules

- Treat app packages, archives, metadata, paths, logs, and IPC input as hostile.
- Validate lengths, formats, paths, and resource limits before parsing/extraction/launch.
- Prevent path traversal and writes outside UMR-managed locations.
- Use least privilege, explicit capability grants, timeouts, process cleanup, and bounded IPC.
- Do not disable code-signing/security checks or implement DRM/integrity bypasses to make a demo work.
- Do not commit app binaries, proprietary OS images, credentials, signing keys, certificates, or personal diagnostic output.
- Avoid logging credentials, user content, keystrokes, or network payloads. Redact by default.
- New remote execution paths require authentication, transport security, session isolation, retention policy, and clear consent.
- Add negative tests for malformed or hostile inputs when package, process, or privilege boundaries change.
- Flag license/security uncertainty for human review; do not assume that public availability permits bundling or redistribution.
