# Suggested Repository Layout

Keep code modular by responsibility and execution boundary. The precise language and build layout should be confirmed after Phase 0.

```text
/
├── README.md
├── AGENTS.md
├── CMakeLists.txt                 # only after C++ project setup is approved
├── docs/
│   ├── 00-project-brief.md
│   ├── 01-product-requirements.md
│   ├── 02-round-1-scope.md
│   ├── 03-architecture.md
│   ├── 04-runtime-backends.md
│   ├── 05-platform-support-matrix.md
│   ├── 06-security-privacy.md
│   ├── 07-test-and-acceptance-plan.md
│   ├── 08-roadmap.md
│   ├── 09-repository-layout.md
│   ├── research/
│   │   └── feasibility-spikes.md
│   └── adr/
│       ├── 0001-desktop-first.md
│       ├── 0002-multiple-backends.md
│       └── 0003-ios-feasibility-gate.md
├── .agent/
│   ├── README.md
│   ├── context/
│   ├── rules/
│   ├── workflows/
│   └── templates/
├── apps/
│   └── desktop-shell/             # product UI and app orchestration only
├── core/
│   ├── runtime-api/               # shared domain contracts and types
│   ├── orchestrator/              # lifecycle and capability discovery
│   ├── diagnostics/               # structured events and support reports
│   └── compatibility/             # matrix and compatibility records
├── backends/
│   ├── android/                   # Android guest/runtime integration
│   └── ios/                       # strategy selected after feasibility work
├── platform/
│   ├── windows/
│   ├── macos/
│   └── linux/
├── tests/
│   ├── unit/
│   ├── integration/
│   ├── e2e/
│   ├── security/
│   └── fixtures/                  # synthetic/owned fixtures only
├── scripts/
└── third_party/                   # only with explicit dependency/license review
```

## Layout rules

- Avoid a generic `utils/` dumping ground; place code with the component that owns it.
- Keep OS-specific code in `platform/<os>/` or a backend module, not scattered through UI code.
- `core/runtime-api/` defines contracts; it must not depend on concrete backend implementations.
- Do not create empty folders solely to make the tree look complete. Add them when the first implementation slice needs them.
- Add a third-party component inventory before vendoring or bundling external code.
- The `.agent/` directory is guidance for agents, not runtime code. Agent products may require additional adapter files.
