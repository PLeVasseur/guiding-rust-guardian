# Safety argument sketch

Fill this in as the design settles; it's a deliverable. One or two
sentences per answer is enough. The course book's "Guardian in the
ISO 26262 Frame" page has the architecture this follows: camera and
radar channels at B(D), a combiner that keeps the D.

## Your channel

- Your arbiter consumes both channels and merges them: it's the
  decision element and the combiner. Which parts of
  it are agent-generated, and which parts are hand-written?
- What envelope would the combiner enforce on your channel's output?
  Name a check that would actually fire on the worst decision trace you
  saw today.

## Independence and dependent failures

- Both channels are in the code. Write your combiner's
  corroboration rule: what agreement does `Brake` require, and within
  how many cycles?
- List three candidate common causes for the two channels (ISO
  26262-9:2018, Clause 7 is the frame). Would generating both channels'
  trackers with the same agent and prompt belong on that list? Why?

## Evidence

- Which acceptance-table row does each layer of your test suite speak
  to?
- For each statistical threshold: where did the number come from?

## Threshold origins

- Which of your thresholds have a published origin, and which are
  yours? Cite the published ones (protocol name, version, and the
  number). For the ones that are yours, write down the measurement
  they came from.
