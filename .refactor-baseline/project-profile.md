# Project refactoring profile — cinematography-ir

- date: 2026-08-26
- depth: A (quick-fit)
- confirmed-by-user: 2026-08-26 (`apply all`)

## Investigation findings

- Verification: the Rust suite is fast and deterministic; the baseline is 79 tests with 0 failures.
- Toolchain: Rust 2021 with `cargo fmt`, `cargo clippy -D warnings`, `cargo test`, example smoke tests, and generated schemas.
- Git topology: one repository with `main` tracking `origin/main`; work runs on `refactor/compiled-guidance-ir-20260826` and pushes only at the session boundary.
- Domain risk: serialized IR, timeline semantics, geometry, generated control passes, and external Blender I/O make contract and execution changes high-risk.
- Hotspots: the requested change crosses `model`, `solve`, `view`, `prompt`, `execute`, and `compare`; recent history confirms these layers evolved together.
- Lifecycle: active early-stage compiler; Parallel Change is cheaper and safer than a rewrite.
- Ownership: the requested repository is fully in scope and has no local `AGENTS.md` or cross-team gate.
- Existing gates: `just check`, `just test`, `just examples`, and schema generation are the project-native gates.
- Cadence: one bounded migration session, at most 20 focused commits, with full verification before the final push.

## Invariant mapping

- I1 Behavior preservation: compatibility/refactor slices are separate from intentional correctness and feature slices.
- I2 Stay-green: every commit requires 0 test failures; final green also requires formatting, clippy, examples, schemas, and Blender script tests.
- I3 Fail-loud: unsupported adapter capabilities, missing focus for DOF, invalid passes, and incomplete output bundles return errors.
- I4 Pre-declared recovery: uncommitted work returns to `71351c6` with named-file `git restore`; committed work is reverted per focused commit.
- I5 Risk-proportional rigor: P0 fixes use focused tests; public IR and adapter migrations use Parallel Change plus contract tests.
- I6 Survey before intervention: the user supplied an evidence-based audit and it was rechecked against `71351c6` before edits.
- I7 Versioned process state: this profile and the progress ledger are committed with the migration.

## Workflow (adopted)

1. Pin and fix the seven P0 correctness defects plus DOF failure semantics.
2. Expand with a shared Compiled Guidance IR while retaining Solved IR compatibility.
3. Migrate view and prompt to compiled truth.
4. Expand the adapter contract, typed pass specs, shot bundles, and transactional output.
5. Expand comparison with screen-space, timing, and constraint results.
6. Add take/edit and model-profile contracts, update schemas/docs, run all gates, then push.

## Deviations from standard

| Standard element | Decision | Evidence / justification |
| --- | --- | --- |
| User confirmation per slice | modify | The user explicitly approved the complete P0–P3 migration with `apply all`. |
| G1/G2 saturation survey | modify | The supplied audit already identifies cross-layer duplicated truth; local cleanup is limited to prerequisites for the approved Parallel Change. |
| Architecture candidate report | drop | The user supplied the architecture report and selected every recommendation before intervention. |

## Budgets & cadence

- One session, target under 4 hours.
- At most 20 commits; each commit must be independently green.
- Push once, after every requested slice is complete and verified.

## Revision triggers hit so far

None.
