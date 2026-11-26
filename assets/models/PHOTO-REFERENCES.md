# Fly model photo references

The second-generation original mesh was revised by studying these photographs of
**Drosophila melanogaster** by **André Karwath (Aka)**, dated 16 July 2005:

- [Side view](https://commons.wikimedia.org/wiki/File:Drosophila_melanogaster_-_side_(aka).jpg):
  domed thorax, descending oval abdomen, thin wings, shorter antennae and slender legs.
- [Dorsal view](https://commons.wikimedia.org/wiki/File:Drosophila_melanogaster_-_top_(aka).jpg):
  head/thorax proportions, paired eyes, thoracic bristles, folded overlapping wings
  and dark abdominal bands.
- [Front view](https://commons.wikimedia.org/wiki/File:Drosophila_melanogaster_-_front_(aka).jpg):
  eye placement, central face, compact antennae, palps and mouthparts.

The photos are licensed [CC BY-SA 2.5](https://creativecommons.org/licenses/by-sa/2.5/).
They were viewed as anatomical references; photo files, crops and textures are not
distributed in this package. This is independently generated, artist-built geometry,
not photogrammetry, a measured specimen reconstruction or a tracing of image pixels.
Venation, lens tiling and setal positions are simplified artistic interpretations.

## Files

- `fly_detailed.stl`: complete resting fly, wings swept back along the abdomen.
- `fly_flight.stl`: complete flight pose with spread wings.
- `fly_body.stl`: shared body used in the native game.
- `fly_wing_left.stl`, `fly_wing_right.stl`: local closed membranes and veins for
  animation; hinge position and rest angle are recorded in `model-info.json`.

Each complete pose has **133,696 triangles**. Geometry includes a continuous banded
abdomen, shallow hexagonal corneal lenses, scutum and scutellum, six articulated legs,
five tarsomeres per leg, claws/pulvilli, halteres, plumose aristae, three ocelli,
retracted mouthparts, curved macrosetae and fine microsetae. Wings have thin closed
membranes and longitudinal veins connected by crossveins.

Units are millimetres, artistically scaled for visibility. Separate anatomical parts
overlap as closed surfaces. A physical print needs a union and thickening of small
features; this is a detailed game/display mesh rather than a ready-to-print solid.
