# guardian: Specification

## guardian v0.1: forward collision warning arbiter

### Input

The starter repo provides `sensor-sim`, which produces a stream of
`TrackReport`s at 20 Hz for scripted scenarios (constant-speed lead
vehicle, hard-braking lead, cut-in, empty road). Two perception channels
observe the same world: each report carries a `sensor` field, radar or
camera. Reports carry an object id, range (m), range rate (m/s), and a
confidence in `[0, 1]`. Reports are noisy, and the channels differ:
radar's range rate is precise, the camera's is much noisier; each
channel drops tracks for 1 to 3 cycles on its own schedule; and each
channel produces its own spurious low-confidence ghosts. A cycle may
carry reports from both channels, one, or neither. Your arbiter consumes
the merged stream and owns the decision.

### Behavior

Each cycle, `guardian` ingests the current reports and emits exactly one
`Decision`:

- `Brake` when collision is imminent (time-to-collision below a hard
  threshold).
- `Warn` when a collision is plausible soon (TTC below a soft threshold).
- `NoAction` otherwise.

TTC for a closing track is `range / closing_speed`. Tracks with confidence
below 0.3 must not, on their own, trigger `Brake`.

### Acceptance targets (per scenario suite, over many runs)

| Metric                                                    | Target |
| --------------------------------------------------------- | ------ |
| Hard-braking-lead scenarios ending in `Brake`             | >= 99% |
| Empty-road runs with any `Warn` or `Brake` (false alarm)  | <= 2%  |
| Median cycles from threshold crossing to correct decision | <= 3   |

Classification semantics, shared by every evaluator of this table:
"ending in `Brake`" means `Brake` appears within the final 20 cycles
(one second) of the run. Latency medians are computed over responders
only; a run that never reaches the required decision is a miss, which
lowers the corresponding rate row and contributes no latency sample.
Zero responders reports `n/a`. Latency output states its response
coverage (responders over total runs) alongside the median.

### Deliverables

The `guardian` crate; a runner binary that executes scenario suites and
reports the metrics; and a test suite you would stake the release on.

## Scope note

The statistical verdict layer (the `feotest` tests) is stretch work.
The required finish line is the property-test layer plus a passing
1,000-trial metrics run with replayable failure seeds. If you get the
statistical layer standing too, excellent; don't start it before the
metrics pass.

## Protocol alignment (stretch)

Real FCW systems are tested against published protocols. Two matter
here, and they're different kinds of document. Euro NCAP's AEB
Car-to-Car protocol is a consumer rating: it scripts rear-end test
scenarios and awards FCW points when the warning comes at
TTC >= 1.70 s.[^ncap] FMVSS No. 127 is a US regulation: it requires
FCW between 10 and 145 km/h whenever a collision is imminent, and full
avoidance of a lead vehicle at speeds up to 62 mph, on new light
vehicles from September 2029.[^fmvss]

The protocol's CCRb scenario is close to this project's world: two
vehicles at 50 km/h, then the lead brakes at -2 or -6 m/s^2, from a
headway of 12 or 40 m. Four cells.

The sim gives you the mechanism and nothing more:
`Scenario::Scripted(ScriptedLead { .. })` takes an initial range, an
initial closing speed, a start time, a closing acceleration, and a
closing-speed cap. Building the four CCRb cells out of it is your
agent's work, under your review:

1. Ask the agent to translate the protocol cells into `ScriptedLead`
   values. Review the unit conversions and the closing-speed cap (what
   does a stopped lead do to closing speed when both cars started at
   50 km/h?).
2. Ask it for a grid runner: N seeded trials per cell, median TTC at
   first `Warn` per cell, measured against noiseless ground truth.
3. Add the acceptance row: median TTC at first warn >= 1.70 s on every
   CCRb cell. This row has a published origin; cite it.
4. Then ask the room's question: your arbiter can pass this row by
   warning constantly. Which other acceptance row stops that?

Simplifications to note in your SAFETY-NOTES: this world is
one-dimensional, so the protocol's lateral overlap grid has no
meaning here, and the sim applies the lead's deceleration as a
constant rather than the protocol's ramp-in tolerance.

[^ncap]: Euro NCAP, "AEB Car-to-Car Test Protocol" v4.3 and
    "Assessment Protocol, Safety Assist, Collision Avoidance",
    <https://cdn.euroncap.com/cars/assets/euro_ncap_aeb_c2c_test_protocol_v43_1e6ed06def.pdf>.
    FCW points are awarded at TTC >= 1.70 s.

[^fmvss]: NHTSA, FMVSS No. 127, final rule May 2024, amended November
    2024, <https://www.federalregister.gov/documents/2024/11/26/2024-27349/federal-motor-vehicle-safety-standards-automatic-emergency-braking-systems-for-light-vehicles>.
