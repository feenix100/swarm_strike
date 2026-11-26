use crate::sim::{BodyKind, Config, Game, Mode, Team};
use bevy::{
    asset::RenderAssetUsages, core_pipeline::tonemapping::Tonemapping, post_process::bloom::Bloom,
    prelude::*, render::render_resource::PrimitiveTopology,
};

pub const CYAN: Color = Color::srgb(0.15, 0.91, 0.9);
pub const AMBER: Color = Color::srgb(1.0, 0.58, 0.23);
pub const INK: Color = Color::srgb(0.018, 0.033, 0.044);
#[derive(Component)]
pub struct CombatModel {
    pub team: Team,
    pub kind: BodyKind,
    pub slot: usize,
}
#[derive(Component)]
pub struct Wing(pub f32, pub usize);
#[derive(Component)]
pub struct GameCamera;
#[derive(Component)]
pub struct BulletVisual(pub usize);
#[derive(Component)]
pub struct SparkVisual(pub usize);
#[derive(Component)]
pub struct Rotor;
#[derive(Component)]
pub struct Reactor;
#[derive(Component)]
pub struct StudioLight(pub f32);
#[derive(Component)]
pub struct FlyArmament;
#[derive(Resource)]
pub struct RenderAssets {
    pub bullet: [Handle<StandardMaterial>; 2],
    pub heavy: [Handle<StandardMaterial>; 2],
}

fn material(
    m: &mut Assets<StandardMaterial>,
    c: Color,
    metal: f32,
    emission: f32,
) -> Handle<StandardMaterial> {
    m.add(StandardMaterial {
        base_color: c,
        metallic: metal,
        perceptual_roughness: 0.5,
        emissive: c.to_linear() * emission,
        ..default()
    })
}
fn block(
    cmd: &mut Commands,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    pos: Vec3,
    scale: Vec3,
) -> Entity {
    cmd.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos).with_scale(scale),
    ))
    .id()
}
/// Flat upward-facing quads share a draw instead of spawning thousands of tiles.
#[derive(Default)]
struct GroundQuads {
    vertices: Vec<[f32; 3]>,
}
impl GroundQuads {
    fn rect(&mut self, x0: f32, z0: f32, x1: f32, z1: f32, y: f32) {
        self.vertices.extend([
            [x0, y, z0],
            [x0, y, z1],
            [x1, y, z1],
            [x0, y, z0],
            [x1, y, z1],
            [x1, y, z0],
        ]);
    }
    fn mesh(self) -> Mesh {
        let n = self.vertices.len();
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; n])
    }
}

fn stl_meshes(data: &[u8], meshes: &mut Assets<Mesh>) -> Vec<(usize, Handle<Mesh>)> {
    let count = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
    let mut vertices: Vec<Vec<[f32; 3]>> = vec![vec![]; 8];
    let mut normals: Vec<Vec<[f32; 3]>> = vec![vec![]; 8];
    for i in 0..count {
        let o = 84 + i * 50;
        let group = (u16::from_le_bytes(data[o + 48..o + 50].try_into().unwrap()) as usize).min(7);
        let f = |p| f32::from_le_bytes(data[p..p + 4].try_into().unwrap());
        let n = [f(o), f(o + 4), f(o + 8)];
        for k in 0..3 {
            let p = o + 12 + k * 12;
            vertices[group].push([f(p), f(p + 4), f(p + 8)]);
            normals[group].push(n);
        }
    }
    vertices
        .into_iter()
        .zip(normals)
        .enumerate()
        .filter_map(|(i, (v, n))| {
            if v.is_empty() {
                None
            } else {
                let mut n = n;
                if !matches!(i, 3 | 6 | 7) {
                    // Weld only shading normals. The STL geometry and fine lens
                    // facets stay intact, while broad cuticle surfaces are smooth.
                    let key = |p: &[f32; 3]| p.map(|x| (x * 100_000.0).round() as i32);
                    let mut smooth = std::collections::HashMap::<[i32; 3], Vec3>::new();
                    for (p, normal) in v.iter().zip(&n) {
                        *smooth.entry(key(p)).or_default() += Vec3::from_array(*normal);
                    }
                    for (p, normal) in v.iter().zip(&mut n) {
                        *normal = smooth[&key(p)].normalize_or_zero().to_array();
                    }
                }
                let len = v.len();
                Some((
                    i,
                    meshes.add(
                        Mesh::new(
                            PrimitiveTopology::TriangleList,
                            RenderAssetUsages::default(),
                        )
                        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, v)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, n)
                        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0]; len]),
                    ),
                ))
            }
        })
        .collect()
}

pub fn setup_scene(
    mut cmd: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    c: Res<Config>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let dark = material(&mut mats, Color::srgb(0.028, 0.048, 0.062), 0.7, 0.0);
    let floor_tile = material(&mut mats, Color::srgb(0.041, 0.065, 0.079), 0.65, 0.0);
    let panel = material(&mut mats, Color::srgb(0.075, 0.105, 0.119), 0.75, 0.0);
    let seam = material(&mut mats, Color::srgb(0.10, 0.24, 0.26), 0.4, 0.5);
    let cyan = material(&mut mats, CYAN, 0.35, 3.5);
    let amber = material(&mut mats, AMBER, 0.35, 3.5);
    block(
        &mut cmd,
        &cube,
        &dark,
        Vec3::new(0.0, -0.6, 0.0),
        Vec3::new(20_000.0, 1.0, 20_000.0),
    );
    // Batch the extended grid into three meshes. Extending its reach should not
    // multiply floor entities or per-frame draw preparation by its tile count.
    let mut tiles = GroundQuads::default();
    for x in -12..12 {
        for z in -12..12 {
            if (x + z) % 2 == 0 {
                tiles.rect(
                    x as f32 * 10.0 + 0.06,
                    z as f32 * 10.0 + 0.06,
                    x as f32 * 10.0 + 9.94,
                    z as f32 * 10.0 + 9.94,
                    0.01,
                );
            }
        }
    }
    let mut minor = GroundQuads::default();
    let mut major = GroundQuads::default();
    for line in -100..=100 {
        let coordinate = line as f32 * 10.0;
        let half_width = if line % 5 == 0 { 0.045 } else { 0.018 };
        let mesh = if line % 5 == 0 {
            &mut major
        } else {
            &mut minor
        };
        mesh.rect(
            coordinate - half_width,
            -1000.0,
            coordinate + half_width,
            1000.0,
            0.055,
        );
        mesh.rect(
            -1000.0,
            coordinate - half_width,
            1000.0,
            coordinate + half_width,
            0.055,
        );
    }
    for (quads, mat) in [
        (tiles, floor_tile),
        (minor, seam),
        (
            major,
            material(&mut mats, Color::srgb(0.12, 0.34, 0.36), 0.4, 0.18),
        ),
    ] {
        cmd.spawn((
            Mesh3d(meshes.add(quads.mesh())),
            MeshMaterial3d(mat),
            Transform::IDENTITY,
        ));
    }
    // Freestanding cover in an open-air training field; no enclosure or ceiling.
    for (i, o) in c.obstacles.iter().enumerate() {
        let p = o.pos();
        let half = o.half();
        let glow = if i % 3 == 0 { &amber } else { &cyan };
        block(&mut cmd, &cube, &dark, p, half * 2.0);
        block(
            &mut cmd,
            &cube,
            &panel,
            p + Vec3::Y * (half.y + 0.08),
            Vec3::new(half.x * 2.0 + 0.5, 0.22, half.z * 2.0 + 0.5),
        );
        for sx in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                block(
                    &mut cmd,
                    &cube,
                    &panel,
                    p + Vec3::new(sx * (half.x - 0.25), 0.0, sz * (half.z - 0.25)),
                    Vec3::new(0.75, half.y * 2.0, 0.75),
                );
                block(
                    &mut cmd,
                    &cube,
                    glow,
                    p + Vec3::new(sx * (half.x + 0.03), 0.0, sz * (half.z + 0.03)),
                    Vec3::new(0.14, half.y * 1.85, 0.14),
                );
            }
        }
        for sy in [-1.0, 1.0] {
            block(
                &mut cmd,
                &cube,
                glow,
                p + Vec3::new(0.0, sy * (half.y - 0.25), half.z + 0.04),
                Vec3::new(half.x * 1.9, 0.10, 0.10),
            );
            block(
                &mut cmd,
                &cube,
                glow,
                p + Vec3::new(0.0, sy * (half.y - 0.25), -half.z - 0.04),
                Vec3::new(half.x * 1.9, 0.10, 0.10),
            );
        }
        for j in 0..4 {
            block(
                &mut cmd,
                &cube,
                &panel,
                p + Vec3::new(0.0, -half.y + 1.8 + j as f32 * 1.15, half.z + 0.04),
                Vec3::new(half.x * 1.6, 0.28, 0.12),
            );
        }
    }
    // Reactor crown and opposite deployment circles.
    let ring = meshes.add(Torus::new(8.8, 9.05));
    cmd.spawn((
        Mesh3d(ring),
        MeshMaterial3d(cyan.clone()),
        Transform::from_xyz(0.0, 38.0, 0.0),
        Reactor,
    ));
    let ring = meshes.add(Torus::new(7.2, 7.3));
    for (p, mat) in [
        (Vec3::new(-28.0, 0.2, 35.0), cyan.clone()),
        (Vec3::new(28.0, 0.2, -35.0), amber.clone()),
    ] {
        cmd.spawn((
            Mesh3d(ring.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(p),
        ));
    }
    cmd.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.49, 0.68, 0.78),
        brightness: 220.0,
        ..default()
    });
    cmd.spawn((
        DirectionalLight {
            illuminance: 7000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.05, -0.65, 0.0)),
    ));
    for (p, color) in [
        (Vec3::new(-24.0, 45.0, 20.0), CYAN),
        (Vec3::new(24.0, 45.0, -20.0), AMBER),
        (Vec3::new(0.0, 54.0, 0.0), Color::WHITE),
    ] {
        cmd.spawn((
            PointLight {
                color,
                intensity: 9_000_000.0,
                range: 85.0,
                radius: 3.0,
                ..default()
            },
            Transform::from_translation(p),
        ));
    }
    cmd.spawn((
        Camera3d::default(),
        DistanceFog {
            color: Color::srgb(0.055, 0.10, 0.16),
            falloff: FogFalloff::Linear {
                start: 100.0,
                end: 400.0,
            },
            ..default()
        },
        GameCamera,
        Transform::from_xyz(-15.0, 25.0, 37.0).looking_at(Vec3::new(0.0, 21.0, 10.0), Vec3::Y),
        Projection::Perspective(PerspectiveProjection {
            fov: 75.0_f32.to_radians(),
            ..default()
        }),
        Tonemapping::TonyMcMapface,
        Bloom {
            intensity: 0.18,
            ..Bloom::NATURAL
        },
        Msaa::Sample4,
    ));
    for (position, color, power) in [
        (
            Vec3::new(-5.0, 29.0, 23.0),
            Color::srgb(1.0, 0.85, 0.65),
            2_800_000.0,
        ),
        (
            Vec3::new(13.0, 28.0, 10.0),
            Color::srgb(0.55, 0.85, 1.0),
            1_400_000.0,
        ),
    ] {
        cmd.spawn((
            PointLight {
                color,
                intensity: power,
                range: 40.0,
                radius: 3.0,
                ..default()
            },
            Transform::from_translation(position),
            StudioLight(power),
        ));
    }

    // STL meshes are embedded so the executable remains portable.
    let colors = [
        Color::srgb(0.47, 0.245, 0.075),
        Color::srgb(0.70, 0.43, 0.145),
        Color::srgb(0.39, 0.012, 0.006),
        Color::srgba(0.79, 0.86, 0.77, 0.16),
        Color::srgb(0.32, 0.25, 0.15),
        Color::srgb(0.065, 0.040, 0.022),
        Color::srgb(0.085, 0.047, 0.022),
        Color::srgb(0.53, 0.022, 0.012),
    ];
    let palette: Vec<_> = colors
        .into_iter()
        .enumerate()
        .map(|(i, color)| {
            mats.add(StandardMaterial {
                base_color: color,
                metallic: 0.02,
                perceptual_roughness: if i == 2 || i == 7 { 0.38 } else { 0.48 },
                alpha_mode: if i == 3 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                cull_mode: None,
                ..default()
            })
        })
        .collect();
    let body = stl_meshes(include_bytes!("../assets/models/fly_body.stl"), &mut meshes);
    let left = stl_meshes(
        include_bytes!("../assets/models/fly_wing_left.stl"),
        &mut meshes,
    );
    let right = stl_meshes(
        include_bytes!("../assets/models/fly_wing_right.stl"),
        &mut meshes,
    );
    let gunmesh = meshes.add(Cylinder::new(0.085, 0.95));
    let mut friendly_palette = palette.clone();
    friendly_palette[2] = material(&mut mats, Color::srgb(0.04, 0.4, 0.45), 0.25, 0.4);
    friendly_palette[7] = material(&mut mats, Color::srgb(0.12, 0.9, 0.95), 0.25, 0.4);
    for team in [Team::Drone, Team::Fly] {
        let fly_palette = if team == Team::Drone {
            &friendly_palette
        } else {
            &palette
        };
        let muzzle = if team == Team::Drone { &cyan } else { &amber };
        for slot in 0..4 {
            cmd.spawn((
                Transform::default(),
                Visibility::default(),
                CombatModel {
                    team,
                    kind: BodyKind::Fly,
                    slot,
                },
            ))
            .with_children(|parent| {
                for (i, mesh) in &body {
                    parent.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(fly_palette[*i].clone()),
                    ));
                }
                for (sign, wingmesh) in [(-1.0, &left), (1.0, &right)] {
                    parent
                        .spawn((
                            Transform::from_xyz(sign * 0.49, 0.42, -0.23),
                            Visibility::default(),
                            Wing(sign, slot),
                        ))
                        .with_children(|wing| {
                            for (i, mesh) in wingmesh {
                                wing.spawn((
                                    Mesh3d(mesh.clone()),
                                    MeshMaterial3d(fly_palette[*i].clone()),
                                ));
                            }
                        });
                    parent.spawn((
                        FlyArmament,
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_xyz(sign * 0.57, 0.14, -0.5)
                            .with_scale(Vec3::new(0.22, 0.23, 0.6)),
                    ));
                    parent.spawn((
                        FlyArmament,
                        Mesh3d(gunmesh.clone()),
                        MeshMaterial3d(panel.clone()),
                        Transform::from_xyz(sign * 0.57, 0.14, -1.0)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    ));
                    parent.spawn((
                        FlyArmament,
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(muzzle.clone()),
                        Transform::from_xyz(sign * 0.57, 0.14, -1.46)
                            .with_scale(Vec3::new(0.10, 0.10, 0.05)),
                    ));
                }
            });
        }
    }
    let rotor = meshes.add(Torus::new(0.80, 1.02));
    let rotorcore = meshes.add(Cylinder::new(0.17, 0.24));
    let accents = [
        cyan.clone(),
        material(&mut mats, Color::srgb(0.25, 0.55, 1.0), 0.2, 3.5),
        material(&mut mats, Color::srgb(0.65, 1.0, 0.25), 0.2, 3.5),
        material(&mut mats, Color::srgb(0.8, 0.35, 1.0), 0.2, 3.5),
    ];
    for (slot, accent) in accents.iter().enumerate() {
        cmd.spawn((
            Transform::default(),
            Visibility::default(),
            CombatModel {
                team: Team::Drone,
                kind: BodyKind::Drone,
                slot,
            },
        ))
        .with_children(|p| {
            p.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(panel.clone()),
                Transform::from_scale(Vec3::new(1.25, 0.56, 1.8)),
            ));
            p.spawn((
                Mesh3d(cube.clone()),
                MeshMaterial3d(accent.clone()),
                Transform::from_xyz(0.0, 0.08, -0.92).with_scale(Vec3::new(0.8, 0.18, 0.07)),
            ));
            for x in [-1.25, 1.25] {
                for z in [-1.0, 1.0] {
                    p.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_xyz(x * 0.5, 0.0, z * 0.5)
                            .with_scale(Vec3::new(1.5, 0.15, 0.24))
                            .with_rotation(Quat::from_rotation_y(if x * z > 0.0 {
                                -0.65
                            } else {
                                0.65
                            })),
                    ));
                    p.spawn((
                        Mesh3d(rotor.clone()),
                        MeshMaterial3d(dark.clone()),
                        Transform::from_xyz(x, 0.0, z),
                    ));
                    p.spawn((
                        Mesh3d(rotorcore.clone()),
                        MeshMaterial3d(accent.clone()),
                        Transform::from_xyz(x, 0.0, z),
                    ));
                    p.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(panel.clone()),
                        Transform::from_xyz(x, 0.0, z).with_scale(Vec3::new(1.5, 0.035, 0.13)),
                        Rotor,
                    ));
                }
            }
            for x in [-0.43, 0.43] {
                p.spawn((
                    Mesh3d(gunmesh.clone()),
                    MeshMaterial3d(dark.clone()),
                    Transform::from_xyz(x, -0.2, -1.23)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                ));
            }
        });
    }
    let bullet = [cyan.clone(), amber.clone()];
    let heavy = [
        material(&mut mats, Color::srgb(0.6, 1.0, 1.0), 0.0, 14.0),
        material(&mut mats, Color::srgb(1.0, 0.72, 0.3), 0.0, 14.0),
    ];
    let ball = meshes.add(Sphere::new(1.0).mesh().ico(1).unwrap());
    for i in 0..256 {
        cmd.spawn((
            Mesh3d(ball.clone()),
            MeshMaterial3d(cyan.clone()),
            Transform::default(),
            Visibility::Hidden,
            BulletVisual(i),
        ));
    }
    for i in 0..512 {
        cmd.spawn((
            Mesh3d(ball.clone()),
            MeshMaterial3d(amber.clone()),
            Transform::default(),
            Visibility::Hidden,
            SparkVisual(i),
        ));
    }
    cmd.insert_resource(RenderAssets { bullet, heavy });
}

// Bevy ECS system parameters express disjoint component access in their types.
#[allow(clippy::type_complexity)]
pub fn animate_scene(
    time: Res<Time>,
    g: Res<Game>,
    mut wings: Query<(&Wing, &mut Transform)>,
    mut rotors: Query<&mut Transform, (With<Rotor>, Without<Wing>)>,
    mut reactors: Query<&mut Transform, (With<Reactor>, Without<Wing>, Without<Rotor>)>,
    mut studio: Query<(&StudioLight, &mut PointLight)>,
    mut armaments: Query<&mut Visibility, With<FlyArmament>>,
) {
    let t = time.elapsed_secs();
    for mut visibility in &mut armaments {
        *visibility = if g.mode == Mode::Menu {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    for (studio, mut light) in &mut studio {
        light.intensity = if g.mode == Mode::Menu { studio.0 } else { 0.0 };
    }
    for (wing, mut transform) in &mut wings {
        let flap = if g.mode == Mode::Menu {
            (t * 2.0).sin() * 0.015
        } else {
            (t * 75.0 + wing.1 as f32 * 0.7).sin() * 0.62
        };
        let sweep = if g.mode == Mode::Menu {
            Quat::from_rotation_x(0.06) * Quat::from_rotation_y(-wing.0 * 1.62)
        } else {
            Quat::IDENTITY
        };
        transform.rotation = sweep * Quat::from_rotation_z(flap * wing.0);
        transform.translation.y = 0.42
            + if g.mode == Mode::Menu && wing.0 < 0.0 {
                0.014
            } else {
                0.0
            };
    }
    for mut t in &mut rotors {
        t.rotate_y(time.delta_secs() * 70.0);
    }
    for mut t in &mut reactors {
        t.rotate_y(time.delta_secs() * 0.15);
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn sync_scene(
    time: Res<Time>,
    g: Res<Game>,
    c: Res<Config>,
    assets: Res<RenderAssets>,
    mut previous_actor: Local<Option<usize>>,
    mut models: Query<(&CombatModel, &mut Transform, &mut Visibility), Without<GameCamera>>,
    mut cam: Query<(&mut Transform, &mut Projection), (With<GameCamera>, Without<CombatModel>)>,
    mut bullets: Query<
        (
            &BulletVisual,
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        (
            Without<CombatModel>,
            Without<GameCamera>,
            Without<SparkVisual>,
        ),
    >,
    mut sparks: Query<
        (
            &SparkVisual,
            &mut Transform,
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        (
            Without<CombatModel>,
            Without<GameCamera>,
            Without<BulletVisual>,
        ),
    >,
) {
    let t = time.elapsed_secs();
    for (model, mut transform, mut visible) in &mut models {
        if g.mode == Mode::Menu {
            *visible = if model.team == Team::Fly && model.kind == BodyKind::Fly && model.slot == 0
            {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
            transform.translation = Vec3::new(3.2, 23.5 + (t * 0.7).sin() * 0.18, 11.0);
            transform.rotation =
                Quat::from_euler(EulerRot::YXZ, 1.70 + (t * 0.15).sin() * 0.20, -0.08, 0.12);
            transform.scale = Vec3::splat(4.0);
        } else {
            let Some(id) = g.actor_id(model.team, model.slot) else {
                *visible = Visibility::Hidden;
                continue;
            };
            let a = &g.actors[id];
            if a.kind != model.kind {
                *visible = Visibility::Hidden;
                continue;
            }
            *visible = if !a.alive() || (g.human == Some(id) && !g.chase) {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            transform.translation = a.pos;
            transform.rotation =
                a.rotation() * Quat::from_rotation_z((-a.vel.x * 0.012).clamp(-0.22, 0.22));
            transform.scale = Vec3::splat(if model.kind == BodyKind::Fly {
                1.25
            } else {
                1.0
            });
        }
    }
    let Ok((mut camera, mut projection)) = cam.single_mut() else {
        return;
    };
    if g.mode == Mode::Menu {
        *previous_actor = None;
        camera.translation = Vec3::new(-11.2, 25.0, 28.0);
        camera.look_at(Vec3::new(-4.1, 21.6, 10.0), Vec3::Y);
    } else {
        let i = g.viewed_actor();
        let a = &g.actors[i];
        let forward = a.forward();
        let (desired, focus) = if g.human.is_none() {
            let orbit = Quat::from_rotation_y((t * 0.16).sin() * 0.7);
            let mut desired = a.pos + orbit * Vec3::new(12.0, 7.0, 16.0);
            let mut hit = 1.0f32;
            for o in &c.obstacles {
                if let Some(f) =
                    crate::sim::segment_box(a.pos, desired, o.pos(), o.half() + Vec3::splat(0.45))
                {
                    hit = hit.min((f - 0.06).max(0.08));
                }
            }
            desired = a.pos.lerp(desired, hit);
            (desired, a.pos + forward * 5.0)
        } else if g.chase {
            let mut desired = a.pos - forward * 11.0 + Vec3::Y * 4.0;
            // Pull chase camera in before cover to prevent seeing through it.
            let mut hit = 1.0f32;
            for o in &c.obstacles {
                if let Some(f) =
                    crate::sim::segment_box(a.pos, desired, o.pos(), o.half() + Vec3::splat(0.45))
                {
                    hit = hit.min((f - 0.06).max(0.08));
                }
            }
            desired = a.pos.lerp(desired, hit);
            (desired, a.pos + forward * 35.0)
        } else {
            (a.pos + Vec3::Y * 0.28, a.pos + forward * 50.0)
        };
        let follow = if *previous_actor != Some(i) {
            1.0
        } else {
            1.0 - (-time.delta_secs() * 14.0).exp()
        };
        *previous_actor = Some(i);
        camera.translation = camera.translation.lerp(desired, follow);
        let shake = Vec3::new((t * 133.0).sin(), (t * 171.0).cos(), 0.0) * g.camera_shake;
        camera.translation += shake;
        camera.look_at(focus, Vec3::Y);
        if let Projection::Perspective(p) = &mut *projection {
            let fov: f32 = if a.boost > 0.0 { 88.0 } else { 75.0 };
            p.fov += (fov.to_radians() - p.fov) * follow;
        }
    }
    for (id, mut transform, mut visible, mut mat) in &mut bullets {
        if let Some(b) = g.bullets.get(id.0) {
            *visible = Visibility::Visible;
            transform.translation = b.pos;
            transform.rotation = Quat::from_rotation_arc(Vec3::Z, b.vel.normalize_or_zero());
            transform.scale = Vec3::new(
                if b.heavy { 0.22 } else { 0.075 },
                if b.heavy { 0.22 } else { 0.075 },
                if b.heavy { 0.7 } else { 1.3 },
            );
            mat.0 = if b.heavy {
                assets.heavy[g.actors[b.owner].team.index()].clone()
            } else {
                assets.bullet[g.actors[b.owner].team.index()].clone()
            };
        } else {
            *visible = Visibility::Hidden;
        }
    }
    for (id, mut transform, mut visible, mut mat) in &mut sparks {
        if let Some(s) = g.sparks.get(id.0) {
            *visible = Visibility::Visible;
            transform.translation = s.pos;
            transform.scale = Vec3::splat(0.11 * (s.ttl / s.max));
            mat.0 = assets.heavy[s.color].clone();
        } else {
            *visible = Visibility::Hidden;
        }
    }
}

/// A fading history makes the slight ballistic drop visible in flight.
pub fn ballistic_trails(mut gizmos: Gizmos, g: Res<Game>) {
    if g.mode != Mode::Playing {
        return;
    }
    for b in &g.bullets {
        let mut previous = b.pos;
        for (i, p) in b.trail.iter().enumerate() {
            let alpha = 0.6 * (1.0 - i as f32 / b.trail.len().max(1) as f32);
            let color = if g.actors[b.owner].team == Team::Drone {
                Color::srgba(0.15, 0.91, 0.9, alpha)
            } else {
                Color::srgba(1.0, 0.58, 0.23, alpha)
            };
            gizmos.line(previous, *p, color);
            previous = *p;
        }
    }
}
