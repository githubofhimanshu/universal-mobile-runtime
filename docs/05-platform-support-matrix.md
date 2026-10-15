# Host and Application Support Matrix

Update this file only from reproducible tests or clearly labelled manual results. Do not mark a combination “supported” because it compiles or because the package can be parsed.

## Status meanings

- **Not evaluated:** No feasibility result yet.
- **Research:** Experiments are in progress; no support claim.
- **Prototype:** A demonstration works for a narrow case; not production support.
- **Partial:** A documented subset works, with known limitations.
- **Verified:** Release acceptance tests passed for the stated configuration.
- **Unsupported:** Known not to work for the declared case.
- **Blocked:** Depends on unavailable permissions, services, hardware, or external requirements.

## Initial matrix

| Host OS | Host CPU | Android strategy | Android status | iOS strategy | iOS status |
|---|---|---|---|---|---|
| Windows | To be decided | Research per-host guest/emulator option | Not evaluated | Local-vs-remote feasibility required | Not evaluated |
| macOS | Intel and/or Apple Silicon to be decided | Research per-host guest/emulator option | Not evaluated | Apple-supported dev workflow and other valid options evaluated separately | Not evaluated |
| Linux | x86-64 and/or ARM64 to be decided | Research container/VM option | Not evaluated | Local-vs-remote feasibility required | Not evaluated |

## Per-app test record

For each app/test fixture, record:

- Test ID and permission/provenance notes (never commit the app unless redistribution is allowed).
- App name and package/bundle identifier where safe to record.
- Package hash and app version/build.
- Host OS/version and host CPU architecture.
- Backend name/version/configuration.
- App/guest architecture and OS/framework level when known.
- Execution mode: local native, virtualized, translated, emulated, or remote.
- Test steps and expected outcomes.
- Result, logs, limitations, and date.
- Whether the result can be shared publicly.

## Required cohorts

Android:

- A simple managed-language app.
- A JNI/native-library app.
- A graphics-intensive app.
- A networking/storage example.
- An app relying on external services, recorded as a separate risk cohort.

 iOS:

- A legally obtained app within the candidate backend's intended era/framework range.
- A basic UI/framework workflow.
- A graphics/input workflow if relevant.
- A test identifying signing, entitlement, service, or device dependence.

The iOS cohort must validate the selected execution approach, not simply the existence of a file loader.
