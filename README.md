# guardian-starter (Project C)

Starter repository for Project C (`guardian`) from the Guiding Rust
course. A cargo workspace:

- [Student course book](https://petelevasseur.com/guiding-rust/)
- [Training resources](https://petelevasseur.com/training/)

Use this repository as a GitHub template for your pair, or clone it directly.
The full workspace is intentionally incomplete until you define the
participant-owned `ParticipantArbiter` skeleton. Root CI is expected to be red
before that milestone.

- `sensor-sim/`: provided, complete, and tested. Scripted scenarios at
  20 Hz with Gaussian noise, dropouts, and ghost tracks. Seedable. Read
  its module documentation, including what `SimConfig::noiseless()` is
  for and what it's not for.
- `guardian/`: yours. The `Decision` enum and its severity ordering are
  provided as shared vocabulary. Implement the small `Arbiter` adapter with a
  concrete `ParticipantArbiter`; the internal design remains yours. `proptest`
  and `feotest` are in dev-dependencies, pinned in the lockfile.
- `runner/`: compiles after `ParticipantArbiter` exists. It uses the stable
  adapter and grows the metrics from there.

As the spec says, the last deliverable is a test suite you would stake
the release on.

`SAFETY-NOTES.md` is the safety-argument sketch, a deliverable.
