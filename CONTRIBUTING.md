# Contributing to ROM

Thanks for considering a contribution.

ROM is still early-stage, so the project values focused, technically clear changes over broad speculative rewrites.

## Before You Start

- Open an issue first for large changes, new subsystems, or architectural shifts.
- Keep pull requests small, reviewable, and single-purpose.
- Prefer compatibility improvements backed by tests over theoretical refactors.
- Read [README.md](./README.md) and [`docs/architecture.md`](./docs/architecture.md) before changing core behavior.

## Development Principles

- Use conventional commits: `feat:`, `fix:`, `docs:`, `chore:`, `ci:`.
- Keep files under 500 lines when practical; split by responsibility instead of growing monoliths.
- Follow clean architecture and keep coupling low.
- Reuse existing helpers instead of duplicating logic.
- Validate browser-facing behavior with tests whenever semantics change.
- Do not mix unrelated work in one pull request.

## Local Workflow

```bash
cargo test
```

Optional browser reference check:

```bash
npm install
npx playwright install chromium
npm run fingerprintjs:browser-reference
```

## Pull Request Expectations

Each pull request should include:

- a clear problem statement
- the chosen implementation approach
- any compatibility or behavioral tradeoffs
- tests for user-visible or browser-facing changes

Good pull requests usually:

- modify a narrow area
- explain why the change exists
- include regression coverage
- avoid unrelated formatting churn

## Areas That Need Extra Care

- browser-facing exception semantics
- WebCrypto behavior
- worker and messaging isolation
- long-lived networking behavior
- compatibility harness stability

## Code Style

- Prefer readable Rust and JavaScript over clever compression.
- Keep comments sparse and useful.
- Avoid hidden magic behavior and implicit cross-module coupling.
- Preserve existing naming and structure unless the change is intentionally a refactor.

## Reporting Bugs

When filing a bug, include:

- expected behavior
- actual behavior
- a minimal reproduction
- ROM commit or branch
- platform details if relevant

## Security

If you believe you found a security issue, please follow [SECURITY.md](./SECURITY.md) instead of opening a public issue first.
