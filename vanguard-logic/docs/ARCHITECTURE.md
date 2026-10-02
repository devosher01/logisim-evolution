# Architecture Boundaries

The workspace is organized around dependency direction, not technical convenience.

```text
document-engine -> domain-model
document-engine -/-> UI, persistence, simulation, platform
```

`domain-model` owns validated entities and invariants. `document-engine` owns
commands and reversible history. Rendering, simulation, persistence, and product
adapters must depend inward through explicit APIs and must never be imported by
the domain crates.

Every new crate needs:

- a single responsibility;
- a documented dependency direction;
- tests for public invariants;
- strict formatting and lint checks;
- a reason for every external dependency.
