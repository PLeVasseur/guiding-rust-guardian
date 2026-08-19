# guardian-starter (Project C)

Starter repository for Project C (`guardian`) from the Guiding Rust
course. A cargo workspace:

- [Student course book](https://petelevasseur.com/guiding-rust/)
- [Training resources](https://petelevasseur.com/training/)

Use this repository as a GitHub template for your pair, or clone it directly.
The full workspace is intentionally incomplete until you define the
participant-owned `ParticipantArbiter` skeleton.

- `sensor-sim/`: provided, complete, and tested. Scripted scenarios at
  20 Hz with Gaussian noise, dropouts, and ghost tracks. Seedable. Read
  its module documentation, including what `SimConfig::noiseless()` is
  for and what it's not for.
- `guardian/`: yours. The `Decision` enum and its severity ordering are
  provided as shared vocabulary. Implement the small `Arbiter` adapter with a
  concrete `ParticipantArbiter`; the internal design remains yours. `proptest`
  and `feotest` are in dev-dependencies, pinned in the lockfile.
- `runner/`: a prepared, reviewable trace and multi-seed metrics harness. It
  compiles after `ParticipantArbiter` exists. Students review the metric
  semantics instead of spending the workshop on trial-loop plumbing.

As the spec says, the last deliverable is a test suite you would stake
the release on.

`SAFETY-NOTES.md` is the safety-argument sketch, a deliverable.

The template's CI temporarily injects a no-op adapter only when
`ParticipantArbiter` is absent, so the provided simulator and runner tests stay
verifiable. The adapter is not committed. As soon as your skeleton defines
`ParticipantArbiter`, CI uses your implementation instead.

## Three-hour route

After creating `ParticipantArbiter`, use short batches while revising and a
larger final batch:

```text
cargo run --release -p runner -- trace --scenario cut-in --seed 3007
cargo run --release -p runner -- metrics --trials 200
cargo run --release -p runner -- metrics --trials 1000
```

The metrics command exits nonzero while product targets are unmet. That is an
expected baseline result, not a broken starter. Replay at least one reported
seed before changing the policy.

Formal confidence bounds with `feotest`, protocol-grid work, mutation testing,
and proofs remain available as after-workshop extensions.
