#![cfg_attr(all(target_os = "windows", not(test)), windows_subsystem = "windows")]
mod neural;
mod neural_core;
mod render;
mod sim;
mod ui;
use bevy::{
    audio::{PlaybackSettings, Volume},
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::{CursorGrabMode, CursorOptions, MonitorSelection, WindowMode},
};
use neural::FlyBrains;
use sim::{BodyKind, Config, Game, Mode, PilotInput, Sound, Team};

fn main() {
    let config = Config::load();
    let mut game = Game::new(&config);
    let args: Vec<String> = std::env::args().collect();
    let option = |name: &str| {
        args.iter()
            .position(|s| s == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if let Some(n) = option("--drones").and_then(|v| v.parse::<usize>().ok()) {
        game.setup.drones = n.clamp(1, 4);
    }
    if let Some(n) = option("--flies").and_then(|v| v.parse::<usize>().ok()) {
        game.setup.flies = n.clamp(1, 4);
    }
    if args.iter().any(|s| s == "--solo") {
        game.linked = false;
    }
    if args.iter().any(|s| s == "--duel") {
        game.setup.drones = 1;
        game.setup.flies = 1;
    }
    if let Some(pilot) = option("--pilot") {
        if pilot == "fly" {
            game.pilot_body = BodyKind::Fly;
            game.start_squad(true, &config);
        } else if pilot == "drone" {
            game.pilot_body = BodyKind::Drone;
            game.start_squad(true, &config);
        } else if pilot == "observe" {
            game.start_squad(false, &config);
        }
    }
    if args.iter().any(|s| s == "--telemetry") {
        game.debug = true;
    }
    if args.iter().any(|s| s == "--controls") {
        game.mode = Mode::Menu;
        game.help = true;
    }
    let capture = Capture {
        path: option("--capture"),
        frame: 0,
        at: option("--capture-frame")
            .and_then(|s| s.parse().ok())
            .unwrap_or(120),
    };
    let profile = PerformanceProbe {
        output: option("--profile-output"),
        capture: option("--profile-capture"),
        seconds: option("--profile-seconds")
            .and_then(|v| v.parse().ok())
            .unwrap_or(30.0),
        start: None,
        frames: Vec::new(),
        sample_steps: [0; 4],
        total_neural_steps: [0; 4],
        active_neural_seconds: [0.0; 4],
        captured: false,
    };
    App::new()
        .insert_resource(profile)
        .insert_resource(config)
        .insert_resource(game)
        .insert_resource(capture)
        .init_resource::<FlyBrains>()
        .init_resource::<PilotInput>()
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .insert_resource(ClearColor(Color::srgb(0.055, 0.10, 0.16)))
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: std::env::current_dir()
                        .unwrap_or_default()
                        .join("assets")
                        .to_string_lossy()
                        .into_owned(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "SWARMSTRIKE | Tactical Flight".into(),
                        resolution: (1440, 900).into(),
                        present_mode: bevy::window::PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(Startup, (render::setup_scene, ui::setup_ui, setup_audio))
        .add_systems(FixedUpdate, simulate)
        .add_systems(
            Update,
            (
                ui::interact,
                controls,
                cursor,
                render::animate_scene,
                render::sync_scene,
                render::ballistic_trails,
                ui::update_ui,
                play_sounds,
                screenshot,
                target_markers,
                capture_frame,
                profile_performance,
            )
                .chain(),
        )
        .run();
}
#[derive(Resource)]
struct Capture {
    path: Option<String>,
    frame: u32,
    at: u32,
}
fn capture_frame(
    mut commands: Commands,
    mut capture: ResMut<Capture>,
    mut exit: MessageWriter<AppExit>,
) {
    if capture.path.is_none() {
        return;
    }
    capture.frame += 1;
    if capture.frame == capture.at {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(capture.path.clone().unwrap()));
    }
    if capture.frame == capture.at + 120 {
        exit.write(AppExit::Success);
    }
}
fn screenshot(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::F12) {
        let _ = std::fs::create_dir_all("screenshots");
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("screenshots/swarmstrike-{t}.png")));
    }
}
fn target_markers(
    g: Res<Game>,
    camera: Single<(&Camera, &GlobalTransform), With<render::GameCamera>>,
    window: Single<&Window>,
    mut markers: Query<(&ui::TargetMarker, &mut Node, &mut BorderColor)>,
) {
    let i = g.viewed_actor();
    let a = &g.actors[i];
    let enemy_team = if a.team == Team::Drone {
        Team::Fly
    } else {
        Team::Drone
    };
    for (marker, mut node, mut color) in &mut markers {
        let Some(id) = g.actor_id(enemy_team, marker.0) else {
            node.display = Display::None;
            continue;
        };
        let other = &g.actors[id];
        let selected = g.selected_target == Some(id);
        node.display = if g.mode == Mode::Playing && other.alive() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display == Display::None {
            continue;
        }
        let size = if selected { 24.0 } else { 14.0 };
        node.width = px(size);
        node.height = px(size);
        *color = BorderColor::all(if selected {
            render::AMBER
        } else {
            Color::srgba(1.0, 0.58, 0.23, 0.45)
        });
        if let Ok(p) = camera.0.world_to_viewport(camera.1, other.pos) {
            let outside = p.x < 25.0
                || p.x > window.width() - 140.0
                || p.y < 95.0
                || p.y > window.height() - 190.0;
            if outside && !selected {
                node.display = Display::None;
                continue;
            }
            node.left = px(p.x.clamp(25.0, window.width() - 140.0) - size * 0.5);
            node.top = px(p.y.clamp(95.0, window.height() - 190.0) - size * 0.5);
        } else if selected {
            let local = a.rotation().inverse() * (other.pos - a.pos);
            node.left = px(if local.x < 0.0 {
                25.0
            } else {
                window.width() - 140.0
            });
            node.top = percent(50);
        } else {
            node.display = Display::None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    mut input: ResMut<PilotInput>,
    mut g: ResMut<Game>,
    mut brains: ResMut<FlyBrains>,
    c: Res<Config>,
    mut window: Single<&mut Window>,
) {
    if keys.just_pressed(KeyCode::F11) {
        window.mode = if window.mode == WindowMode::Windowed {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        } else {
            WindowMode::Windowed
        };
    }
    if keys.just_pressed(KeyCode::KeyM) {
        g.muted = !g.muted;
    }
    if keys.just_pressed(KeyCode::F9) {
        g.debug = !g.debug;
    }
    if keys.just_pressed(KeyCode::KeyH) {
        if g.mode == Mode::Playing {
            g.mode = Mode::Paused;
        }
        g.help = !g.help;
    }
    if g.help {
        if keys.just_pressed(KeyCode::Escape) {
            g.help = false;
        }
        *input = PilotInput::default();
        return;
    }
    if g.mode == Mode::Menu {
        if keys.just_pressed(KeyCode::KeyJ) {
            g.setup.drones = g.setup.drones % 4 + 1;
        }
        if keys.just_pressed(KeyCode::KeyK) {
            g.setup.flies = g.setup.flies % 4 + 1;
        }
        if keys.just_pressed(KeyCode::KeyP) {
            g.pilot_body = if g.pilot_body == BodyKind::Drone {
                BodyKind::Fly
            } else {
                BodyKind::Drone
            };
        }
        if keys.just_pressed(KeyCode::KeyB) {
            g.linked = !g.linked;
        }
        let deploy = keys.just_pressed(KeyCode::KeyD);
        let observe = keys.just_pressed(KeyCode::KeyO);
        if deploy || observe {
            g.start_squad(deploy, &c);
            brains.reset();
            brains.configure();
        }
        *input = PilotInput::default();
        return;
    }
    if g.mode == Mode::Finished {
        if keys.just_pressed(KeyCode::Enter) {
            g.restart(&c);
            brains.reset();
        }
        if keys.just_pressed(KeyCode::Escape) {
            g.mode = Mode::Menu;
        }
        *input = PilotInput::default();
        return;
    }
    if keys.just_pressed(KeyCode::Escape) {
        g.mode = if g.mode == Mode::Paused {
            Mode::Playing
        } else {
            Mode::Paused
        };
        g.input_grace = 0.10;
    }
    if !window.focused && g.mode == Mode::Playing && g.human.is_some() {
        g.mode = Mode::Paused;
    }
    if g.mode != Mode::Playing {
        *input = PilotInput::default();
        return;
    }
    if keys.just_pressed(KeyCode::KeyB) {
        g.toggle_linked();
    }
    if keys.just_pressed(KeyCode::KeyV) {
        g.chase = !g.chase;
    }
    if keys.just_pressed(KeyCode::Tab) {
        g.cycle_craft(keys.pressed(KeyCode::ShiftLeft));
    }
    if g.squad {
        for (slot, key) in [KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4]
            .into_iter()
            .enumerate()
        {
            if keys.just_pressed(key) {
                g.select_craft(slot);
            }
        }
    }
    if keys.just_pressed(KeyCode::KeyT) {
        g.cycle_target();
    }
    if keys.just_pressed(KeyCode::KeyC) {
        g.cycle_order();
    }
    if keys.just_pressed(KeyCode::KeyG) {
        g.focus_fire();
    }
    let axis = |positive, negative| {
        keys.pressed(positive) as u8 as f32 - keys.pressed(negative) as u8 as f32
    };
    input.movement = Vec3::new(
        axis(KeyCode::KeyD, KeyCode::KeyA),
        axis(KeyCode::KeyE, KeyCode::KeyQ),
        axis(KeyCode::KeyS, KeyCode::KeyW),
    );
    input.fire = mouse.pressed(MouseButton::Left) && g.input_grace <= 0.0;
    input.boost = keys.pressed(KeyCode::ShiftLeft);
    input.reload = keys.pressed(KeyCode::KeyR);
    input.stabilize = mouse.pressed(MouseButton::Right);
    if g.human.is_some() {
        let sensitivity = c.mouse_sensitivity * if input.stabilize { 0.38 } else { 1.0 };
        if g.input_grace <= 0.0 {
            g.apply_look(-motion.delta.x * sensitivity, -motion.delta.y * sensitivity);
        }
        if keys.just_pressed(KeyCode::Digit1) {
            g.set_weapon(0);
        }
        if keys.just_pressed(KeyCode::Digit2) {
            g.set_weapon(1);
        }
    }
}
fn cursor(g: Res<Game>, window: Single<&Window>, mut cursor: Single<&mut CursorOptions>) {
    let locked = g.mode == Mode::Playing && g.human.is_some() && window.focused;
    cursor.visible = !locked;
    cursor.grab_mode = if locked {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
}
fn simulate(
    time: Res<Time<Fixed>>,
    mut g: ResMut<Game>,
    input: Res<PilotInput>,
    c: Res<Config>,
    mut brains: ResMut<FlyBrains>,
) {
    g.step(time.delta_secs(), &input, &c, &mut brains);
}
#[derive(Resource)]
struct AudioClips([Handle<AudioSource>; 6]);
fn setup_audio(mut cmd: Commands, assets: Res<AssetServer>) {
    cmd.insert_resource(AudioClips([
        assets.load("audio/rifle.wav"),
        assets.load("audio/cannon.wav"),
        assets.load("audio/hit.wav"),
        assets.load("audio/kill.wav"),
        assets.load("audio/boost.wav"),
        assets.load("audio/reload.wav"),
    ]));
}
fn play_sounds(mut cmd: Commands, clips: Res<AudioClips>, mut g: ResMut<Game>) {
    let muted = g.muted;
    for sound in g.sounds.drain(..) {
        if !muted {
            let i = match sound {
                Sound::Rifle => 0,
                Sound::Cannon => 1,
                Sound::Hit => 2,
                Sound::Kill => 3,
                Sound::Boost => 4,
                Sound::Reload => 5,
            };
            cmd.spawn((
                AudioPlayer::new(clips.0[i].clone()),
                PlaybackSettings {
                    volume: Volume::Linear(0.45),
                    ..PlaybackSettings::DESPAWN
                },
            ));
        }
    }
}

#[derive(Resource)]
struct PerformanceProbe {
    output: Option<String>,
    capture: Option<String>,
    seconds: f64,
    start: Option<std::time::Instant>,
    frames: Vec<f64>,
    sample_steps: [u64; 4],
    total_neural_steps: [u64; 4],
    active_neural_seconds: [f64; 4],
    captured: bool,
}
fn profile_performance(
    mut commands: Commands,
    mut probe: ResMut<PerformanceProbe>,
    time: Res<Time<Real>>,
    brains: Res<FlyBrains>,
    game: Res<Game>,
    mut exit: MessageWriter<AppExit>,
) {
    if probe.output.is_none() {
        return;
    }
    let elapsed = probe
        .start
        .get_or_insert_with(std::time::Instant::now)
        .elapsed()
        .as_secs_f64();
    if elapsed < 3.0 {
        for slot in 0..4 {
            probe.sample_steps[slot] = brains.get(slot).steps;
        }
        return;
    }
    for slot in 0..game.setup.flies {
        let steps = brains.get(slot).steps;
        probe.total_neural_steps[slot] += if steps >= probe.sample_steps[slot] {
            steps - probe.sample_steps[slot]
        } else {
            steps
        };
        probe.sample_steps[slot] = steps;
        if game
            .actors
            .iter()
            .any(|actor| actor.team == Team::Fly && actor.slot == slot && actor.alive())
        {
            probe.active_neural_seconds[slot] += time.delta_secs_f64();
        }
    }
    probe.frames.push(time.delta_secs_f64() * 1000.0);
    if elapsed >= 8.0 && !probe.captured {
        if let Some(path) = &probe.capture {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
        }
        probe.captured = true;
    }
    if elapsed >= probe.seconds + 3.0 {
        let frame_count = probe.frames.len();
        let mean = probe.frames.iter().sum::<f64>() / frame_count as f64;
        probe.frames.sort_by(f64::total_cmp);
        let neural:Vec<_>=(0..game.setup.flies).map(|slot| {
            let b=brains.get(slot);
            serde_json::json!({"slot":slot+1,"steps":b.steps,"last_step_spikes":b.spikes,
                "total_measured_neural_steps":probe.total_neural_steps[slot],
                "measured_steps_per_active_second":probe.total_neural_steps[slot] as f64/probe.active_neural_seconds[slot].max(0.001),
                "motor":[b.motor.x,b.motor.y,b.motor.z]})
        }).collect();
        let stats = serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"seconds":elapsed-3.0,"frames":frame_count,
            "average_fps":1000.0/mean,"mean_frame_ms":mean,"p95_frame_ms":probe.frames[frame_count*95/100],
            "p99_frame_ms":probe.frames[frame_count*99/100],"max_frame_ms":probe.frames[frame_count-1],
            "neurons_per_enemy":brains.get(0).neurons(),"connections":brains.get(0).edge_count,
            "opponents":game.setup.flies,"neural":neural,"game_elapsed":game.elapsed,
            "scores":game.scores,"shots":game.actors.iter().map(|a|a.shots).sum::<u32>()});
        std::fs::write(
            probe.output.take().unwrap(),
            serde_json::to_string_pretty(&stats).unwrap(),
        )
        .expect("Write performance profile");
        exit.write(AppExit::Success);
    }
}
