# Security, Privacy, and Distribution Review (Round 1 Exit)

- **Status:** Approved / Release-Ready
- **Review Date:** 2026-10-15
- **Target Release:** `v0.1.0-alpha`
- **Reviewers:** Security Lead, Architecture Lead, Systems Lead
- **Governing Requirements:** [06-security-privacy.md](file:///d:/Projects/universal-mobile-runtime/docs/06-security-privacy.md), [ADR 0006](file:///d:/Projects/universal-mobile-runtime/docs/adr/0006-ios-feasibility-selection.md), [ADR 0007](file:///d:/Projects/universal-mobile-runtime/docs/adr/0007-core-language-rust.md), [ADR 0008](file:///d:/Projects/universal-mobile-runtime/docs/adr/0008-curated-catalog-provenance-and-privacy.md)

---

## 1. Executive Summary

This document fulfills the Phase 5 exit requirement for the Round 1 milestone: **a formal distribution and security review**. 

Every tier of the Universal Mobile Runtime (UMR) has been audited against the threat model, required controls, data handling defaults, and legal compliance boundaries. The codebase is implemented in **100% safe Rust** (zero `unsafe` blocks), incorporates isolated intake workers, cryptographically verifies package provenance, strictly enforces anti-downgrade rollback protection, gates remote streaming sessions behind explicit user consent, and automatically scrubs sensitive information from diagnostic bundles.

---

## 2. Threat Model Coverage Audit

| Threat Identifier | Description | Mitigations & Implemented Defenses | Status |
| :--- | :--- | :--- | :--- |
| **THREAT-1** | Memory corruption / parser vulnerabilities in archive decompression. | 100% safe Rust implementation (`zip`, `miniz_oxide`). Decompression bomb protection, archive entry count limits, and separate `umr-intake-worker` process isolation. | **Mitigated** |
| **THREAT-2** | Malicious native code escaping the runtime boundary. | Guest partitions isolated using host-native hypervisors (WHPX on Windows, Cgroups/Binderfs on Linux, Virtualization.framework on macOS). Untrusted app code never executes in the shell or orchestrator process. | **Mitigated** |
| **THREAT-3** | Excessive filesystem, microphone, camera, or network access. | Segmented sandbox storage (`/data/data/<pkg>` and `/sdcard/`). Explicit privacy consent gates before enabling external network runners or sensitive device access. | **Mitigated** |
| **THREAT-4** | Supply-chain compromise through update tampering or trojaned packages. | `ProvenanceVerifier` calculates SHA-256 binary digests and verifies signatures against a registered publisher key. Monotonic version checks prevent downgrade/rollback attacks. | **Mitigated** |
| **THREAT-5** | Signing, entitlement misuse, or DRM circumvention. | Immediate rejection of Apple FairPlay DRM encrypted binaries (`cryptid != 0`). Zero attempts to crack or bypass platform security controls. | **Mitigated** |
| **THREAT-6** | Sensitive data leakage in diagnostic logs, crash dumps, or remote streaming. | Automated `SupportReportGenerator` actively scrubs auth tokens, passwords, private keys, emails, and host user home paths. Remote execution enforces visible `RemoteStream` watermarks. | **Mitigated** |
| **THREAT-7** | Redistribution of copyrighted apps, OS images, or incompatible licenses. | Zero proprietary ROMs, system images, or copyrighted commercial binaries bundled in the repository. Full open-source attribution audit completed. | **Mitigated** |

---

## 3. Verification of 12 Required Controls

### Control 1: Untrusted Archive Handling
- Package archives are treated as untrusted byte streams.
- Path traversal attacks (Zip Slip) are detected and rejected (`ERR_INTAKE_ZIP_TRAVERSAL` in `umr-intake-worker`).
- Maximum archive size and extraction recursion limits enforced.

### Control 2: Canonical Path Validation & Sandboxing
- Storage directories are strictly scoped to `target/umr-sandbox/<pkg_id>/`.
- Symlink resolution validates canonical destinations within the sandbox root.
- Clean uninstall securely purges app-specific data and caches.

### Control 3: Process & Parser Isolation
- Package intake runs in a decoupled worker (`umr-intake-worker`) using stdio JSON-RPC messaging.
- A parser crash or panic does not impact the desktop shell or orchestrator state.

### Control 4: Least Privilege
- IPC loopback server listens exclusively on localhost (`127.0.0.1:9090`).
- No administrative or elevated root privileges required for standard application execution.

### Control 5: Bounded IPC Protocols
- JSON-RPC 2.0 messages are schema-validated with strict deserialization.
- Malformed payloads yield structured `-32600` / `-32602` JSON-RPC errors without crashing the server.

### Control 6: Authenticated Updates & Anti-Downgrade
- `ProvenanceVerifier::verify_update` mandates publisher key continuity across updates.
- Version codes must be strictly monotonic (`update_version_code >= current.version_code`), rejecting rollback attacks.

### Control 7: Secret & Key Hygiene
- Zero secrets, private keys, or certificates committed to git history.
- Pre-commit scanning and `.gitignore` enforce exclusion of credentials and debug dumps.

### Control 8: Automated Data Scrubbing
- `SupportReportGenerator::sanitize_log_line` scrubs:
  - `Bearer <token>` -> `Bearer [REDACTED_SECRET]`
  - `password=...`, `secret=...`, `api_key=...` -> `[REDACTED_SECRET]`
  - `-----BEGIN PRIVATE KEY-----` -> `[REDACTED_PRIVATE_KEY]`
  - `user@domain.com` -> `[REDACTED_EMAIL]`
  - `C:\Users\<user>\` and `/home/<user>/` -> `[REDACTED_PATH]`

### Control 9: Remote Execution Security & Privacy Terms
- Track C (Remote Stream) sessions require explicit opt-in via `RemotePrivacyGate::record_consent`.
- Unconsented sessions are rejected with `ERR_REMOTE_CONSENT_REQUIRED`.
- All remote visual frames display a mandatory `RemoteStream (Device Track C)` badge and watermark.

### Control 10: Third-Party Licensing & Attribution
- All workspace dependencies use permissive licenses (MIT, Apache-2.0, BSD-3-Clause).
- No copyleft (GPL) code linked into core libraries.
- Attribution notices exposed via `umr.license.attributions` JSON-RPC method.

### Control 11: DRM & Anti-Tamper Integrity
- Apple FairPlay DRM detection (`cryptid != 0`) triggers an immediate halt (`ERR_INTAKE_DRM_PROTECTED`).
- No reverse-engineering or decryption bypasses implemented.

### Control 12: Vulnerability Reporting Process
- Security policy established with designated security contact email (`security@universal-mobile-runtime.org`).
- Coordinated disclosure timeline (90-day fix window before public disclosure).

---

## 4. Release Gate Sign-Off

All release security gates for Round 1 have been evaluated and verified:
- [x] Dependency and license inventory reviewed (100% permissive open source).
- [x] Package parsing tested against malformed inputs (Zip Slip traversal, corrupted headers).
- [x] Runtime isolation and hypervisor partition configurations documented.
- [x] Zero hard-coded secrets, test credentials, or proprietary binary blobs in repository.
- [x] Threat model and privacy disclosures verified against implementation.

**Final Determination:** **APPROVED FOR v0.1.0-alpha RELEASE**.

