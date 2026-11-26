"""Original, photo-informed Drosophila surfaces; no photograph textures or scans.

Front is -Z, dorsal is +Y. Wing files are local to WING_HINGE. All pieces are
closed triangle surfaces; intersections are intentional, not a printable union.
"""
import json
import math
import random

TAU = math.tau
WING_HINGE = (.49, .42, -.23)
REST_SWEEP = 1.62
REST_PITCH = .06


def add(a, b): return tuple(x + y for x, y in zip(a, b))
def sub(a, b): return tuple(x - y for x, y in zip(a, b))
def mul(a, s): return tuple(x * s for x in a)
def cross(a, b): return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
def unit(a): return mul(a, 1 / max(1e-12, math.sqrt(sum(x*x for x in a))))
def mix(a, b, t): return add(mul(a, 1-t), mul(b, t))


def bristle(solid, root, direction, length, radius=.004, bend=(0, 0, .12), mat=6):
    """A recurved, tapering seta rather than a thick straight spike."""
    direction = unit(direction)
    p = root
    for j in range(3):
        t = (j + 1) / 3
        q = add(root, add(mul(direction, length*t), mul(bend, length*t*t)))
        solid.tube(p, q, radius*(1-j/3), max(.00065, radius*(1-t)), mat, 6)
        p = q


def loft(solid, profile, material, n=72):
    """Continuous abdomen with elliptical cross-sections and actual band geometry."""
    rings = [[(rx*math.cos(TAU*i/n), cy+ry*math.sin(TAU*i/n), z)
              for i in range(n)] for z, cy, rx, ry in profile]
    for i in range(n):
        k = (i+1) % n
        solid.tri((0, profile[0][1], profile[0][0]), rings[0][k], rings[0][i], 1)
        solid.tri((0, profile[-1][1], profile[-1][0]), rings[-1][i], rings[-1][k], 5)
        for j in range(len(rings)-1):
            a, b = rings[j][i], rings[j][k]
            c, d = rings[j+1][i], rings[j+1][k]
            z = (profile[j][0] + profile[j+1][0]) * .5
            dorsal = math.sin(TAU*(i+.5)/n)
            mat = material(z, dorsal)
            solid.tri(a, b, c, mat)
            solid.tri(b, d, c, mat)


def eye(solid, sign):
    p, r = (sign*.48, .13, -1.13), (.255, .365, .285)
    solid.sphere(p, r, 2, 64, 36)
    # Tiny, shallow hexagonal corneal lenses. No raised spherical beads.
    for j in range(1, 25):
        latitude = math.pi * j / 26
        count = max(8, round(64*math.sin(latitude)))
        for i in range(count):
            longitude = TAU * (i + (j % 2)*.5) / count
            d = (math.sin(latitude)*math.cos(longitude), math.cos(latitude),
                 math.sin(latitude)*math.sin(longitude))
            if d[0]*sign < -.06 or d[2] > .72:
                continue
            center = add(p, tuple(r[k]*d[k] for k in range(3)))
            normal = unit(tuple(d[k]/r[k] for k in range(3)))
            u = unit(cross(normal, (0, 1, 0)))
            v = cross(normal, u)
            radius = .0175
            front, back = add(center, mul(normal, .0022)), add(center, mul(normal, -.002))
            ring = [add(front, add(mul(u, radius*math.cos(TAU*k/6)),
                                   mul(v, radius*math.sin(TAU*k/6)))) for k in range(6)]
            rear = [add(a, mul(normal, -.0042)) for a in ring]
            mat = 7 if (i + 2*j) % 7 else 2
            for k in range(6):
                nxt = (k+1) % 6
                solid.tri(front, ring[k], ring[nxt], mat)
                solid.tri(back, rear[nxt], rear[k], mat)
                solid.tri(ring[k], rear[k], rear[nxt], mat)
                solid.tri(ring[k], rear[nxt], ring[nxt], mat)
            if (i+j*3) % 23 == 0:
                bristle(solid, front, normal, .023, .0009, (0, 0, .02))


def catmull(points, subdivisions=6, closed=False):
    out = []
    count = len(points) if closed else len(points)-1
    for i in range(count):
        a = points[(i-1) % len(points)] if closed else points[max(0, i-1)]
        b, c = points[i], points[(i+1) % len(points)]
        d = points[(i+2) % len(points)] if closed else points[min(len(points)-1, i+2)]
        for step in range(subdivisions):
            t = step/subdivisions
            out.append(tuple(.5*((2*b[k])+(-a[k]+c[k])*t +
                                (2*a[k]-5*b[k]+4*c[k]-d[k])*t*t +
                                (-a[k]+3*b[k]-3*c[k]+d[k])*t*t*t)
                             for k in range(len(b))))
    if not closed:
        out.append(points[-1])
    return out


def wing_height(x): return .015*math.sin(math.pi*max(0, min(1, x/3)))


def wing(solid_type, sign):
    s = solid_type()
    # Rounded distal tip, narrow root and broad posterior anal field.
    boundary = catmull([(0, 0), (.16, -.19), (.62, -.33), (1.28, -.37),
                        (2.02, -.30), (2.59, -.12), (2.91, .13), (3.0, .39),
                        (2.91, .64), (2.61, .83), (2.12, .93), (1.48, .84),
                        (.91, .62), (.49, .41), (.18, .29), (.035, .16)], 7, True)
    outline = [(sign*x, wing_height(x), z) for x, z in boundary]
    center = (sign*1.48, .027, .26)
    bottom = add(center, (0, -.008, 0))
    for i, a in enumerate(outline):
        b = outline[(i+1) % len(outline)]
        a2, b2 = add(a, (0, -.008, 0)), add(b, (0, -.008, 0))
        if sign > 0:
            s.tri(center, b, a, 3); s.tri(bottom, a2, b2, 3)
            s.tri(a, b, b2, 3); s.tri(a, b2, a2, 3)
        else:
            s.tri(center, a, b, 3); s.tri(bottom, b2, a2, 3)
            s.tri(a, b2, b, 3); s.tri(a, a2, b2, 3)
        s.tube(a, b, .004, .004, 4, 6)

    # Costa and five longitudinal fields, joined by r-m and dm-cu crossveins.
    # This is an artist's venation layout, not a measured specimen tracing.
    veins = [
        [(0, 0), (.32, -.17), (1.1, -.29), (2.04, -.25), (2.7, -.04)],
        [(.15, -.03), (.6, -.12), (1.3, -.12), (2.14, -.02), (2.91, .22)],
        [(.11, .04), (.57, .06), (1.26, .16), (2.06, .32), (2.94, .53)],
        [(.10, .10), (.56, .24), (1.15, .43), (1.84, .63), (2.55, .85)],
        [(.12, .16), (.43, .31), (.77, .48), (1.21, .72), (1.5, .84)],
        [(.91, .09), (.94, .20), (.95, .36)],
        [(1.94, .30), (1.93, .44), (1.88, .64)],
    ]
    for points in veins:
        curve = catmull(points, 5)
        for i in range(len(curve)-1):
            a, b = curve[i], curve[i+1]
            ra = .009 - .004*i/max(1, len(curve)-1)
            s.tube((sign*a[0], wing_height(a[0])+.007, a[1]),
                   (sign*b[0], wing_height(b[0])+.007, b[1]), ra, ra*.96, 4, 7)
    # Sparse marginal setae remain real geometry in a plain STL viewer.
    for i in range(49, len(outline)-5, 3):
        p = outline[i]
        bristle(s, p, (sign*.35, .1, 1), .026, .00085, (0, 0, .04))
    return s


def append_pose(target, source, origin, yaw=0, pitch=0):
    c, s = math.cos(yaw), math.sin(yaw)
    cp, sp = math.cos(pitch), math.sin(pitch)
    def transform(p):
        x, y, z = c*p[0]+s*p[2], p[1], -s*p[0]+c*p[2]
        return add(origin, (x, cp*y-sp*z, sp*y+cp*z))
    for a, b, c0, mat in source.triangles:
        target.tri(transform(a), transform(b), transform(c0), mat)


def build(solid_type, root):
    rng = random.Random(57012)
    body = solid_type()
    # Convex mesonotal dome, narrower neck, small posterior scutellum.
    body.sphere((0, .19, -.03), (.58, .56, .70), 0, 72, 40)
    body.sphere((0, -.13, -.03), (.47, .38, .68), 1, 48, 28)
    body.sphere((0, .34, .62), (.35, .22, .28), 0, 48, 24)
    body.sphere((0, .07, -.79), (.33, .29, .17), 5, 36, 20)
    body.sphere((0, .075, -1.08), (.51, .43, .32), 1, 64, 36)
    body.sphere((0, .04, -1.337), (.245, .30, .085), 1, 40, 24)

    # Continuous abdominal cuticle, with shallow physical intersegment furrows.
    profile = []
    anchors = [(.61, -.02, .29, .27), (.82, -.055, .43, .35),
               (1.10, -.12, .51, .405), (1.40, -.19, .52, .41),
               (1.70, -.245, .45, .36), (1.96, -.28, .33, .28),
               (2.18, -.29, .19, .18), (2.31, -.29, .065, .08),
               (2.34, -.29, .012, .02)]
    for z, cy, rx, ry in catmull(anchors, 7):
        phase = (z-.74)/.275
        furrow = 1 - .018*math.exp(-((phase-round(phase))/.09)**2)
        profile.append((z, cy, max(.01, rx*furrow), max(.015, ry*furrow)))

    def bands(z, dorsal):
        if z > 1.91 and dorsal > -.45:
            return 5
        phase = ((z-.73)/.275) % 1
        return 5 if .67 < phase < .96 and dorsal > -.24 else 1
    loft(body, profile, bands)
    for sign in (-1, 1):
        eye(body, sign)
        # Short antennae: scape, pedicel, hanging funiculus, plumose arista.
        body.sphere((sign*.15, .18, -1.405), (.075, .085, .068), 0, 20, 12)
        body.sphere((sign*.17, .10, -1.46), (.058, .078, .055), 1, 20, 12)
        body.sphere((sign*.18, -.018, -1.49), (.074, .128, .045), 0, 24, 16)
        root_arista = (sign*.20, .04, -1.52)
        end_arista = (sign*.41, .44, -1.54)
        body.tube(root_arista, end_arista, .008, .0015, 6, 7)
        for j in range(1, 9):
            p = mix(root_arista, end_arista, j/10)
            size = .11*(1-j/13)
            bristle(body, p, (sign*.8, .22, -.22), size, .0022, (sign*.1, .2, .05))
            bristle(body, p, (-sign*.65, .12, .20), size*.8, .0018, (0, .1, .05))
        # Maxillary palps flank a compact, retracted proboscis.
        body.sphere((sign*.15, -.22, -1.37), (.045, .075, .10), 1, 20, 12)
        body.sphere((sign*.049, -.31, -1.39), (.054, .044, .072), 0, 20, 12)
        bristle(body, (sign*.20, -.19, -1.43), (sign*.4, -.7, -.4), .15, .004)
        # Halteres sit behind and below the wing roots, with cream knobs.
        body.tube((sign*.34, .12, .48), (sign*.64, .11, .72), .022, .014, 1, 12)
        body.sphere((sign*.67, .12, .75), (.073, .063, .085), 1, 24, 16)

        legs = [
            [(.32, -.22, -.46), (.45, -.40, -.68), (.65, -.61, -1.04), (.73, -1.12, -1.37)],
            [(.43, -.24, -.04), (.62, -.42, .04), (1.02, -.64, .16), (1.10, -1.12, .29)],
            [(.37, -.22, .44), (.48, -.42, .68), (.73, -.65, 1.17), (.85, -1.12, 1.72)],
        ]
        for leg, coords in enumerate(legs):
            points = [(sign*x, y, z) for x, y, z in coords]
            for segment in range(3):
                ra, rb = [(.072, .055), (.056, .036), (.028, .017)][segment]
                body.tube(points[segment], points[segment+1], ra, rb, 1, 14)
                body.sphere(points[segment+1], (rb*1.12,)*3, 0, 16, 10)
                for j in range(1, 9):
                    p = mix(points[segment], points[segment+1], j/10)
                    bristle(body, p, (sign*.7, -.25, .2), .045, .0023)
            p = points[-1]
            dz = [-.057, .022, .063][leg]
            for t in range(5):
                q = add(p, (sign*.018, -.024, dz*(1-t*.08)))
                body.tube(p, q, .015-t*.0018, .013-t*.0017, 0, 10)
                p = q
            for side in (-1, 1):
                q = add(p, (sign*.019, .004, dz*.48+side*.015))
                body.tube(p, q, .006, .001, 5, 8)
                body.sphere(add(p, (sign*.01, -.006, side*.011)), (.011, .006, .013), 1, 12, 8)

    # Ocellar triangle and named-looking rows of orbital and thoracic setae.
    body.sphere((0, .47, -1.08), (.13, .018, .13), 5, 28, 12)
    for x, z in [(-.065, -1.03), (.065, -1.03), (0, -1.15)]:
        body.sphere((x, .49, z), (.028, .017, .028), 2, 16, 10)
    for sign in (-1, 1):
        for z in [-1.25, -1.08, -.92]:
            y = .075 + .43*math.sqrt(1-(.27/.51)**2-((z+1.08)/.32)**2)
            bristle(body, (sign*.27, y, z), (sign*.4, .8, .38), .22, .005)
        for z in [-.30, .08, .38]:
            y = .19 + .56*math.sqrt(1-(.20/.58)**2-((z+.03)/.70)**2)
            bristle(body, (sign*.20, y, z), (sign*.25, .65, .75), .34, .0065)
        for z in [-.23, .04, .22]:
            y = .19 + .56*math.sqrt(1-(.50/.58)**2-((z+.03)/.70)**2)
            bristle(body, (sign*.50, y, z), (sign*.65, .3, .7), .25, .005)
        y = .34 + .22*math.sqrt(1-(.20/.35)**2-((.75-.62)/.28)**2)
        bristle(body, (sign*.20, y, .75), (sign*.35, .25, .93), .40, .0065)

    # Fine acrostichal microsetae follow the thoracic surface instead of random spikes.
    for row in range(-5, 6):
        x = row*.072
        for j in range(17):
            z = -.58 + j*.064 + rng.uniform(-.01, .01)
            inside = 1-(x/.58)**2-((z+.03)/.70)**2
            if inside <= 0:
                continue
            y = .19 + .56*math.sqrt(inside)
            bristle(body, (x, y, z), (x*.4, .65, .8), rng.uniform(.045, .085), .0019)
    # Abdominal setae originate on the surface of the smooth cuticle.
    for j in range(4, len(profile)-4, 4):
        z, cy, rx, ry = profile[j]
        for k in range(12):
            angle = TAU*k/12
            p = (rx*math.cos(angle), cy+ry*math.sin(angle), z)
            bristle(body, p, (math.cos(angle)*.4, math.sin(angle)*.4, .85), .055, .002)

    wings = [wing(solid_type, sign) for sign in (-1, 1)]
    path = root/'assets/models'
    body.write(path/'fly_body.stl')
    wings[0].write(path/'fly_wing_left.stl')
    wings[1].write(path/'fly_wing_right.stl')
    counts = {}
    for name, sweep in [('fly_detailed.stl', REST_SWEEP), ('fly_flight.stl', 0.0)]:
        complete = solid_type()
        complete.triangles = list(body.triangles)
        for sign, surface in zip((-1, 1), wings):
            y = WING_HINGE[1] + (.014 if sweep and sign < 0 else 0)
            append_pose(complete, surface, (sign*WING_HINGE[0], y, WING_HINGE[2]),
                        -sign*sweep, REST_PITCH if sweep else 0)
        complete.write(path/name)
        counts[name] = len(complete.triangles)
    info = {
        'version': 2, 'species_reference': 'Drosophila melanogaster, male habitus',
        'triangles': counts['fly_detailed.stl'], 'files': counts, 'units': 'millimetres',
        'pose': 'folded resting wings in fly_detailed.stl; spread flight wings in fly_flight.stl',
        'wing_hinge': list(WING_HINGE), 'rest_sweep_radians': REST_SWEEP,
        'rest_pitch_radians': REST_PITCH, 'rest_left_wing_height_offset': .014,
        'components': 'overlapping closed surfaces; not boolean-unioned',
        'interpretation': 'Original artist-built mesh informed by reference photographs; not a scan or measured reconstruction.',
        'features': ['continuous tapered banded abdomen', 'arched scutum and scutellum',
                     'shallow hexagonal corneal lenses', 'short plumose antennae',
                     'retracted mouthparts and maxillary palps', 'six naturally posed slender legs',
                     'five tarsomeres with claws and pulvilli', 'halteres',
                     'recurved macrosetae and surface microsetae', 'thin swept wings with longitudinal veins and crossveins'],
        'palette': ['ochre chitin', 'golden cuticle', 'red compound eye', 'translucent membrane',
                    'fine wing vein', 'dark tergite', 'seta', 'corneal lens'],
        'reference_photographer': 'André Karwath (Aka), CC BY-SA 2.5; viewed as anatomical references only, photos not bundled',
        'reference_urls': [f'https://commons.wikimedia.org/wiki/File:Drosophila_melanogaster_-_{view}_(aka).jpg'
                           for view in ['front', 'side', 'top']],
    }
    (path/'model-info.json').write_text(json.dumps(info, indent=2, ensure_ascii=False), encoding='utf-8')
    print('Photo-informed fly STL:', counts['fly_detailed.stl'], 'triangles; resting and flight poses')
