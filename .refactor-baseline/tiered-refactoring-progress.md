# Tiered refactoring progress

### Iteration 1

- grade: G3·Evolve (technique: Parallel Change, serves: P1 Single Source of Truth)
- risk tier: high-risk (router reason: public serialized contracts, dependency direction, and adapter I/O change together)
- executor: high-risk contract; commit: `86cdd6c`; postconditions: P0 regressions and golden tests pass

### Iteration 2

- grade: G3·Evolve (technique: Branch by Abstraction, serves: P1 Single Source of Truth)
- risk tier: high-risk (new public Compiled Guidance IR and compatibility bridge)
- executor: shared compiler product consumed by prompt, view, execute, and compare; commit: `4fc2607`

### Iteration 3

- grade: G3·Evolve (technique: Encapsulate Variable, serves: P2 Complete Mediation)
- risk tier: high-risk (execution contract, pass formats, and transaction boundary)
- executor: typed execution profiles, transactional shot bundles, and real Blender validation; commit: `4fc2607`

### Iteration 4

- grade: G2·Reshape (technique: Extract Module, serves: P3 Information-Rich Interfaces)
- risk tier: medium-risk (semantic diagram lowering and model-control separation)
- executor: semantic diagram IR, human review views, clean controls, and optional cue maps; commit: `4fc2607`

### Iteration 5

- grade: G2·Reshape (technique: Replace Derived Variable with Query, serves: P1 Single Source of Truth)
- risk tier: medium-risk (comparison changes without source compatibility break)
- executor: screen-space and constraint-driven comparison with Sim(3) and optional DTW; commit: `4fc2607`

- cumulative: 2/20 implementation commits before documentation synchronization
- validation: Rust tests and clippy pass; schemas/examples render; Blender 5.2.0 LTS produced all ten requested channels for frames 0-1 with a complete transactional manifest
