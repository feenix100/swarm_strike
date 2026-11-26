# Validation — Swarmstrike 0.7.0

Expanded-map checks on 2026-11-25:

- Extended the floor grid from 100 to 2,000 metres across. Fine lines at 10 metres,
  major lines at 50 metres and central checker tiles are batched into three meshes.
- Added 24 freestanding cover structures, bringing the total to 33. New ground,
  mid-altitude and raised positions leave open routes around and above cover.
- All 31 Rust tests pass, including safe spawns for all squad combinations,
  collision and occlusion, ballistic bursts, movement beyond old boundaries,
  independent neural state, and full-data neural response checks.
- cargo fmt --all --check and cargo clippy --locked --all-targets -- -D warnings pass.
- Native four-opponent check: 20 measured seconds after 3 seconds of warm-up,
  1440x900, AutoVsync; 60.02 average FPS; 17.758 ms p95 frame time; neural workers
  approximately 198 steps per active second. Screenshots were captured separately.
  [Raw profile](docs/validation/openfield-four-opponents.json).
- The official connectome file and neural_core.rs are unchanged. Recording uses
  the same full graph and core with fixed 200 Hz neural time and 60 Hz physics.
- Three new 36-second native gameplay exports passed full decoding and metadata
  checks: 1080 frames each, 1920x1080, 30 fps, H.264, stereo AAC at 48 kHz.
  Sound effects follow actual combat events. No damage or eliminations are staged.

The figures below document the earlier data migration and remain historical v0.6
measurements. They are not repeated measurements of the expanded v0.7 map.

## Earlier validation — Swarmstrike 0.6.0

Validated on 2026-11-25. Native Windows build, 1440 × 900, AutoVsync at
approximately 60 FPS, four friendly craft, autonomous battles. Each measured run
lasted 30 seconds after 3 seconds of rendered warm-up. Screenshots were captured
separately and are excluded from the measured runs. Machine: 12 logical CPUs and
about 68.4 GB physical RAM; GPU identity was not collected.

## Rendered performance

| Build and workload | Average FPS | 95th percentile frame time (ms) | Neural steps / active second |
|---|---:|---:|---:|
| v0.5 previous subset / 4 opponents | 60.02 | 17.648 | 199.9–200.0 |
| v0.6 full MaleCNS / 1 opponent | 59.95 | 17.740 | 199.2–199.2 |
| v0.6 full MaleCNS / 4 opponents | 60.02 | 17.786 | 195.7–195.9 |

Both builds maintained the 60 FPS presentation target in this test. This does not
measure uncapped frame-rate headroom or prove identical CPU cost. The full graph
uses more computation and about 210 MB of shared wiring in memory, plus independent
neuron state and rendering resources. Worker throughput is close to the 200-step
target. Rates are sampled across respawns, excluding dead time, and are approximate.
No lower-spec machine, extended session or other display resolution was benchmarked.

The full four-opponent battle recorded 339 shots and a 4–4 team score in this run.
Different observations produced different spike counts and motor readouts per
opponent. Latest neural outputs are consumed without waiting for a step; slower
workers reduce neural time progression rather than silently selecting fewer cells.

Raw reports: [old build](docs/validation/previous-four-opponents.json),
[one full circuit](docs/validation/malecns-one-opponent.json),
[four full circuits](docs/validation/malecns-four-opponents.json).

## Source fidelity

All three official download hashes match pinned inputs. An independent validator
compared every imported edge, original body ID and integer synapse count with
the official MaleCNS v1.0 files. Result:

- 166,700 annotated neurons, 25,582,938 directed connections.
- 124,177,617 synapses between those neurons.
- 10,299,701 connections of weight one and 101 self-connections retained.
- No functional-region or extra connection-weight threshold.
- Runtime SHA-256: `19fc8bda4bdd8bb70b008e499f25adcf92c01970ba035aadc75c642198f9bbd4`.

The raw table has 151,856,684 segment-pair rows. Rows with an endpoint outside
the official class-annotated neuron set are not in the runtime neuron graph;
the original full table remains untouched in data/malecns-v1.0. This is explicitly
recorded in provenance.json. [Full comparison result](docs/validation/official-data-validation.json).

## Automated and native checks

- 31 Rust tests passed, including full official graph dimensions, sensory response,
  connection ablation, independent state, burst timing, curved projectiles, cover,
  squads, scoring and respawns. Game-rule tests use small deterministic circuits;
  they are separate from the actual full-data tests and native worker runs.
- cargo fmt --all --check and cargo clippy --locked --all-targets -- -D warnings.
- Asset validator: original STL geometry checks plus new runtime file checksum,
  header, counts and size. Geometry was not changed by the MaleCNS import.
- Native one- and four-opponent runs exited successfully. Telemetry was inspected
  for the official counts, independent activity and actual worker update rates.

These checks validate implementation and provenance, not biological flight.
The model uses simplified LIF dynamics and engineered sensory and motor interfaces.
