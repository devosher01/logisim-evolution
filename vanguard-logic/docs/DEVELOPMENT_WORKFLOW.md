# Development Workflow

This product uses a protected-main, short-lived-branch workflow. The process is
intentionally small because the project has a single engineering team, but every
change must remain traceable, reviewable, reproducible, and reversible.

## Branches

- `main`: always releasable; never commit directly.
- `vanguard/feature/<short-name>`: user-visible capability.
- `vanguard/fix/<short-name>`: defect correction.
- `vanguard/refactor/<short-name>`: behavior-preserving structural work.
- `vanguard/docs/<short-name>`: documentation-only work.
- `vanguard/release/<version>`: optional release preparation only.

Branches should be short-lived and based on the current `main`. Do not create a
permanent `develop` branch: it adds merge drift without improving safety for a
single team.

## Commit convention

Use imperative, focused commits with a Conventional Commits prefix:

```text
feat(domain): add stable component identities
fix(simulator): preserve deterministic event ordering
refactor(netlist): isolate connectivity compilation
perf(renderer): batch visible component instances
test(domain): reject duplicate entity identifiers
docs(workflow): define release gates
chore(ci): validate the Rust workspace
```

A commit must compile, pass the checks for its scope, and represent one logical
change. Do not mix formatting churn, generated output, or unrelated cleanup into
an implementation commit.

## Pull requests

Every change enters `main` through a pull request, including changes made by the
sole maintainer. The pull request description records:

- problem and intended behavior;
- architectural impact;
- tests and benchmarks executed;
- security or compatibility impact;
- follow-up work, if intentionally deferred.

Use squash merge. The resulting commit on `main` must be a complete, readable
milestone. Rebase or update the branch before merging; do not merge `main` into
short-lived branches unless it is required to resolve a real integration issue.

## Quality gates

Required before merge:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Performance-sensitive changes also require a benchmark comparison. Parser,
format, and simulation changes require property tests or fuzz coverage before
being considered complete.

## Versions and releases

Use Semantic Versioning:

- `0.y.z`: public API may still evolve.
- `0.y.z` patch: compatible bug fix.
- `0.y+1.0`: planned incompatible change.
- `1.0.0`: stable document format and public API contract.

Create annotated, signed tags named `vanguard-v<MAJOR>.<MINOR>.<PATCH>` only
from `main` after all checks pass. Release notes describe user-visible changes,
known limitations, migrations, security fixes, and reproducibility information.

## Recovery rules

- Never rewrite published `main`.
- Revert a bad merge with a new commit.
- Preserve the failing test when fixing a regression.
- Delete a branch only after its commits are reachable from `main` or explicitly
  abandoned.
- Keep generated artifacts out of Git unless they are release inputs.
