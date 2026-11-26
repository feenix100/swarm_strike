# Third-party material

## Anatomical photograph references

André Karwath (Aka), front/side/top photographs of Drosophila melanogaster,
16 July 2005, Wikimedia Commons, CC BY-SA 2.5. The photographs were studied as
anatomical references and are not distributed or used as textures. Exact photo
links and modeling notes are in `assets/models/PHOTO-REFERENCES.md`. The revised
procedural geometry is an original artistic interpretation, not a specimen scan.

## Official MaleCNS v1.0 neuron graph

Data obtained directly from the files linked at https://male-cns.janelia.org/download/.
CC BY 4.0: https://creativecommons.org/licenses/by/4.0/.
Credit: HHMI Janelia FlyEM Project Team, University of Cambridge, MRC Laboratory
of Molecular Biology, Google Research and collaborators.
Reference: Berg et al., Sexual dimorphism in the complete Drosophila male central
nervous system connectome, Cell (2026), https://doi.org/10.1016/j.cell.2026.08.015.

Unmodified source files are cached separately under data/malecns-v1.0. Exact URLs,
hashes and storage generations are in assets/neural/official-downloads.json.
The runtime file includes all 166700 class-annotated neurons and all 25582938
official connections between them, retaining original IDs and integer counts
totalling 124177617 synapses. Rows involving endpoints outside the annotated
neuron set are excluded explicitly; no connection-weight threshold is added.
CSR reindexing changes storage order only. Neuron annotations are retained in
assets/neural/neuron-annotations.json.gz. No community graph package is used.

Electrical signs, normalization, background currents, simplified LIF parameters,
sensory encoding and motor decoding are local model assumptions. They are not
measured physiology or a scientifically validated flight controller. See
assets/neural/provenance.json, src/neural_core.rs and README.md.

The earlier female FlyWire subset and upstream package license are no longer
bundled in this version. Prior distributions have their own notices.

## Rust dependencies

Bevy 0.19.1 and its dependencies retain their respective licenses. Bevy is dual
MIT/Apache-2.0. Exact versions are recorded in `Cargo.lock`. Geometry and synthesized
audio in `assets/models` and `assets/audio` were created for this project.
