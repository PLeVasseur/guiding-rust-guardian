# Compact safety case

Complete this during the three-hour route. One or two sentences per cell is
enough. Red metrics and missing evidence belong here; do not hide them.

## System boundary and human ownership

- Which parts of the arbiter, skeleton, runner review, tests, and thresholds
  were human-owned? Which parts were agent-generated?
- What state crosses decision cycles, and which safety-relevant policy does it
  encode?
- What does the arbiter do when radar and camera disagree, or when one channel
  reports nothing?

## Claim/evidence table

Complete at least two rows in the workshop. Add the rest afterward.

| Claim | Evidence observed | Assumption | Known limitation | Next verification action |
| --- | --- | --- | --- | --- |
| A low-confidence track cannot cause `Brake` on its own | | | | |
| A hard-braking lead produces a timely `Brake` often enough | | | | |
| Empty-road ghosts rarely produce an intervention | | | | |
| The decision does not chatter unacceptably near a threshold | | | | |
| The two-channel policy tolerates a short dropout | | | | |

## Functional-safety frame

- Both a missed intervention and an unwarranted brake are hazardous. Which
  evidence speaks to each direction?
- The architecture assumes B(D) + B(D) decomposition with sufficiently
  independent channels. Name one dependent failure that could invalidate that
  assumption. Does using the same generated tracking code in both channels
  belong on the list?
- The arbiter owns the combined decision. What envelope or monitor would sit
  outside generated code, and which bad trace from today would make it fire?
- Which agent failure is the current test/runner barrier intended to detect?
  What failure could still pass through it?

## Thresholds and evidence status

| Threshold or policy | Value | Origin | Evidence status |
| --- | --- | --- | --- |
| Warn engage | | spec / policy / empirical / published | measured / statistically supported / unverified |
| Brake engage | | spec / policy / empirical / published | measured / statistically supported / unverified |
| Persistence or hold policy | | spec / policy / empirical / published | measured / statistically supported / unverified |

## After-workshop expansion

- Add confidence bounds and sample-size rationale to every rate claim.
- Trace requirements to examples, properties, metrics, and monitors.
- Expand the dependent-failure list and channel-independence argument.
- Record protocol-derived thresholds with document name, version, and units.
