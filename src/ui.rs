use crate::{
    neural::FlyBrains,
    render::{AMBER, CYAN, INK},
    sim::{BodyKind, Config, Game, Mode, Team},
};
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub enum Action {
    PilotBody,
    Linked,
    Help,
    Drone,
    Observe,
    DroneCount,
    FlyCount,
    Resume,
    Restart,
    Menu,
}
#[derive(Component)]
pub struct ButtonStyle {
    primary: bool,
}
#[derive(Component, Clone, Copy)]
pub enum Panel {
    Menu,
    Hud,
    Roster,
    Pause,
    End,
    Telemetry,
    Help,
}
#[derive(Component, Clone, Copy)]
pub enum Label {
    Score,
    Strength,
    Timer,
    Health,
    Boost,
    Weapon,
    Ammo,
    Flight,
    Notice,
    EndTitle,
    EndStats,
    Neural,
    Telemetry,
    Enemy,
    Respawn,
    DroneCount,
    FlyCount,
    Rule,
    Roster(usize),
    Orders,
    Marker(usize),
    PilotBody,
    ControlMode,
    Trigger,
}
#[derive(Component)]
pub struct Meter(pub usize);
#[derive(Component)]
pub struct Crosshair;
#[derive(Component)]
pub struct DamageOverlay;
#[derive(Component)]
pub struct NeuralBar(pub usize);
#[derive(Component)]
pub struct TargetMarker(pub usize);
const MUTED: Color = Color::srgb(0.46, 0.61, 0.65);
const WHITE: Color = Color::srgb(0.88, 0.95, 0.93);

fn text(
    cmd: &mut Commands,
    parent: Entity,
    value: &str,
    size: f32,
    color: Color,
    node: Node,
) -> Entity {
    let id = cmd
        .spawn((
            Text::new(value),
            TextFont {
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(color),
            node,
        ))
        .id();
    cmd.entity(parent).add_child(id);
    id
}
fn label(
    cmd: &mut Commands,
    parent: Entity,
    tag: Label,
    size: f32,
    color: Color,
    node: Node,
) -> Entity {
    let id = text(cmd, parent, "", size, color, node);
    cmd.entity(id).insert(tag);
    id
}
fn button(
    cmd: &mut Commands,
    parent: Entity,
    value: &str,
    action: Action,
    primary: bool,
) -> (Entity, Entity) {
    let id = cmd
        .spawn((
            Button,
            action,
            ButtonStyle { primary },
            Node {
                width: percent(100),
                min_height: px(50),
                padding: UiRect::axes(px(17), px(12)),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(if primary {
                CYAN
            } else {
                Color::srgba(0.25, 0.42, 0.45, 0.7)
            }),
            BackgroundColor(if primary {
                CYAN
            } else {
                Color::srgba(0.05, 0.10, 0.12, 0.85)
            }),
        ))
        .id();
    cmd.entity(parent).add_child(id);
    let caption = text(
        cmd,
        id,
        value,
        15.0,
        if primary { INK } else { WHITE },
        Node::default(),
    );
    (id, caption)
}
fn full_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        ..default()
    }
}

fn key_row(cmd: &mut Commands, parent: Entity, keys: &str, description: &str) {
    let row = cmd
        .spawn(Node {
            width: percent(100),
            min_height: px(40),
            column_gap: px(13),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    cmd.entity(parent).add_child(row);
    let cap = cmd
        .spawn((
            Node {
                min_width: px(112),
                padding: UiRect::axes(px(10), px(8)),
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.20, 0.42, 0.45)),
            BackgroundColor(Color::srgb(0.04, 0.10, 0.12)),
        ))
        .id();
    cmd.entity(row).add_child(cap);
    text(cmd, cap, keys, 14.0, CYAN, Node::default());
    text(
        cmd,
        row,
        description,
        15.0,
        WHITE,
        Node {
            flex_shrink: 1.0,
            ..default()
        },
    );
}
fn help_column(cmd: &mut Commands, parent: Entity, title: &str, rows: &[(&str, &str)]) -> Entity {
    let id = cmd
        .spawn((
            Node {
                width: percent(32),
                padding: UiRect::all(px(20)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                border: UiRect::top(px(2)),
                ..default()
            },
            BorderColor::all(CYAN),
            BackgroundColor(Color::srgba(0.025, 0.06, 0.075, 0.94)),
        ))
        .id();
    cmd.entity(parent).add_child(id);
    text(
        cmd,
        id,
        title,
        21.0,
        WHITE,
        Node {
            margin: UiRect::bottom(px(10)),
            ..default()
        },
    );
    for (keys, description) in rows {
        key_row(cmd, id, keys, description);
    }
    id
}

pub fn setup_ui(mut cmd: Commands) {
    let menu = cmd
        .spawn((
            Panel::Menu,
            Node {
                position_type: PositionType::Absolute,
                width: percent(45),
                height: percent(100),
                padding: UiRect::axes(vw(4.5), vh(3)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                row_gap: px(9),
                ..default()
            },
            BackgroundColor(Color::srgba(0.014, 0.029, 0.039, 0.95)),
        ))
        .id();
    text(
        &mut cmd,
        menu,
        "S W A R M S T R I K E   /   S Q U A D   L A B",
        13.0,
        CYAN,
        Node {
            margin: UiRect::bottom(px(6)),
            ..default()
        },
    );
    text(
        &mut cmd,
        menu,
        "INSTINCT\nVS. SILICON",
        46.0,
        WHITE,
        Node::default(),
    );
    text(
        &mut cmd,
        menu,
        "Pilot a drone or a fly. Fight a neural swarm.\nChoose your craft, squad size, and opponents.",
        16.0,
        MUTED,
        Node {
            margin: UiRect::bottom(px(7)),
            ..default()
        },
    );
    let (_, caption) = button(&mut cmd, menu, "", Action::PilotBody, false);
    cmd.entity(caption).insert(Label::PilotBody);
    let counts = cmd
        .spawn(Node {
            width: percent(100),
            column_gap: px(12),
            ..default()
        })
        .id();
    cmd.entity(menu).add_child(counts);
    let (drone, caption) = button(&mut cmd, counts, "", Action::DroneCount, false);
    cmd.entity(drone).insert(Node {
        width: percent(50),
        min_height: px(50),
        padding: UiRect::all(px(14)),
        border: UiRect::all(px(1)),
        align_items: AlignItems::Center,
        ..default()
    });
    cmd.entity(caption).insert(Label::DroneCount);
    let (fly, caption) = button(&mut cmd, counts, "", Action::FlyCount, false);
    cmd.entity(fly).insert(Node {
        width: percent(50),
        min_height: px(50),
        padding: UiRect::all(px(14)),
        border: UiRect::all(px(1)),
        align_items: AlignItems::Center,
        ..default()
    });
    cmd.entity(caption).insert(Label::FlyCount);
    let (_, caption) = button(&mut cmd, menu, "", Action::Linked, false);
    cmd.entity(caption).insert(Label::ControlMode);
    button(
        &mut cmd,
        menu,
        "DEPLOY YOUR SQUAD                    [ D ]",
        Action::Drone,
        true,
    );
    button(
        &mut cmd,
        menu,
        "WATCH SQUAD BATTLE                   [ O ]",
        Action::Observe,
        false,
    );
    button(
        &mut cmd,
        menu,
        "CONTROLS + FLIGHT BRIEFING          [ H ]",
        Action::Help,
        false,
    );
    label(
        &mut cmd,
        menu,
        Label::Neural,
        13.0,
        CYAN,
        Node {
            margin: UiRect::top(px(4)),
            ..default()
        },
    );
    key_row(&mut cmd, menu, "LMB", "3 rounds, then cooldown. Release.");
    key_row(
        &mut cmd,
        menu,
        "TAB / B",
        "Switch craft / link squad inputs",
    );
    label(
        &mut cmd,
        menu,
        Label::Rule,
        12.0,
        AMBER,
        Node {
            margin: UiRect::top(px(5)),
            ..default()
        },
    );

    let specimen = cmd
        .spawn((
            Panel::Menu,
            Node {
                position_type: PositionType::Absolute,
                right: vw(4),
                bottom: vh(6),
                width: vw(38),
                padding: UiRect::all(px(18)),
                border: UiRect::left(px(2)),
                flex_direction: FlexDirection::Column,
                row_gap: px(7),
                ..default()
            },
            BorderColor::all(AMBER),
            BackgroundColor(Color::srgba(0.015, 0.035, 0.045, 0.75)),
        ))
        .id();
    text(
        &mut cmd,
        specimen,
        "INDEPENDENT NEURAL FLIGHT PILOTS",
        17.0,
        WHITE,
        Node::default(),
    );
    text(
        &mut cmd,
        specimen,
        "133,696 TRIANGLES / PHOTO-INFORMED ANATOMY",
        12.0,
        AMBER,
        Node::default(),
    );
    text(
        &mut cmd,
        specimen,
        "Each fly has its own activity, sensory input and motor output.\nOfficial MaleCNS v1.0 neuron graph, engineered navigation and combat.",
        13.0,
        MUTED,
        Node::default(),
    );

    let hud = cmd.spawn((Panel::Hud, full_node())).id();
    for slot in 0..4 {
        let marker = cmd
            .spawn((
                TargetMarker(slot),
                Node {
                    position_type: PositionType::Absolute,
                    width: px(24),
                    height: px(24),
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BorderColor::all(AMBER),
            ))
            .id();
        cmd.entity(hud).add_child(marker);
        label(
            &mut cmd,
            marker,
            Label::Marker(slot),
            11.0,
            AMBER,
            Node {
                position_type: PositionType::Absolute,
                top: px(26),
                left: px(-12),
                width: px(180),
                ..default()
            },
        );
    }
    text(
        &mut cmd,
        hud,
        "S W A R M S T R I K E",
        19.0,
        WHITE,
        Node {
            position_type: PositionType::Absolute,
            left: px(30),
            top: px(24),
            ..default()
        },
    );
    text(
        &mut cmd,
        hud,
        "OPEN FIELD / SQUAD FLIGHT",
        12.0,
        CYAN,
        Node {
            position_type: PositionType::Absolute,
            left: px(31),
            top: px(51),
            ..default()
        },
    );
    label(
        &mut cmd,
        hud,
        Label::Score,
        23.0,
        WHITE,
        Node {
            position_type: PositionType::Absolute,
            left: percent(34),
            top: px(24),
            ..default()
        },
    );
    label(
        &mut cmd,
        hud,
        Label::Strength,
        12.0,
        MUTED,
        Node {
            position_type: PositionType::Absolute,
            left: percent(34),
            top: px(57),
            ..default()
        },
    );
    label(
        &mut cmd,
        hud,
        Label::Timer,
        25.0,
        CYAN,
        Node {
            position_type: PositionType::Absolute,
            right: px(32),
            top: px(24),
            ..default()
        },
    );
    label(
        &mut cmd,
        hud,
        Label::Enemy,
        13.0,
        AMBER,
        Node {
            position_type: PositionType::Absolute,
            right: px(32),
            top: px(63),
            ..default()
        },
    );
    label(
        &mut cmd,
        hud,
        Label::Notice,
        19.0,
        WHITE,
        Node {
            position_type: PositionType::Absolute,
            left: percent(34),
            top: percent(22),
            width: percent(62),
            ..default()
        },
    );
    label(
        &mut cmd,
        hud,
        Label::Respawn,
        23.0,
        AMBER,
        Node {
            position_type: PositionType::Absolute,
            left: percent(34),
            top: percent(42),
            ..default()
        },
    );
    for (i, pos) in [
        (0, Vec2::new(-13.0, -1.0)),
        (1, Vec2::new(6.0, -1.0)),
        (2, Vec2::new(-1.0, -13.0)),
        (3, Vec2::new(-1.0, 6.0)),
    ] {
        let cross = cmd
            .spawn((
                Crosshair,
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    top: percent(50),
                    margin: UiRect {
                        left: px(pos.x),
                        top: px(pos.y),
                        ..default()
                    },
                    width: px(if i < 2 { 7 } else { 2 }),
                    height: px(if i < 2 { 2 } else { 7 }),
                    ..default()
                },
                BackgroundColor(CYAN),
            ))
            .id();
        cmd.entity(hud).add_child(cross);
    }
    let roster = cmd
        .spawn((
            Panel::Roster,
            Node {
                position_type: PositionType::Absolute,
                left: px(30),
                top: px(92),
                width: px(292),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                border: UiRect::left(px(2)),
                ..default()
            },
            BorderColor::all(CYAN),
            BackgroundColor(Color::srgba(0.012, 0.026, 0.035, 0.9)),
        ))
        .id();
    cmd.entity(hud).add_child(roster);
    text(
        &mut cmd,
        roster,
        "YOUR SQUAD / TAB TO SWITCH",
        12.0,
        MUTED,
        Node::default(),
    );
    for slot in 0..4 {
        label(
            &mut cmd,
            roster,
            Label::Roster(slot),
            14.0,
            WHITE,
            Node::default(),
        );
    }
    label(
        &mut cmd,
        roster,
        Label::Orders,
        12.0,
        AMBER,
        Node {
            margin: UiRect::top(px(3)),
            ..default()
        },
    );
    let bottom = cmd
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(30),
                bottom: px(42),
                width: px(285),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                padding: UiRect::all(px(18)),
                border: UiRect::left(px(2)),
                ..default()
            },
            BorderColor::all(CYAN),
            BackgroundColor(Color::srgba(0.012, 0.026, 0.035, 0.85)),
        ))
        .id();
    cmd.entity(hud).add_child(bottom);
    label(
        &mut cmd,
        bottom,
        Label::Health,
        24.0,
        WHITE,
        Node::default(),
    );
    for (i, col) in [(0, CYAN), (1, AMBER)] {
        let bg = cmd
            .spawn((
                Node {
                    width: percent(100),
                    height: px(if i == 0 { 5 } else { 3 }),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.13, 0.21, 0.24)),
            ))
            .id();
        cmd.entity(bottom).add_child(bg);
        let fill = cmd
            .spawn((
                Meter(i),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(col),
            ))
            .id();
        cmd.entity(bg).add_child(fill);
    }
    label(&mut cmd, bottom, Label::Boost, 12.0, AMBER, Node::default());
    label(
        &mut cmd,
        hud,
        Label::Flight,
        13.0,
        MUTED,
        Node {
            position_type: PositionType::Absolute,
            left: percent(36),
            bottom: px(64),
            ..default()
        },
    );
    let weapon = cmd
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(30),
                bottom: px(42),
                width: px(300),
                padding: UiRect::all(px(18)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                border: UiRect::right(px(2)),
                ..default()
            },
            BorderColor::all(AMBER),
            BackgroundColor(Color::srgba(0.012, 0.026, 0.035, 0.85)),
        ))
        .id();
    cmd.entity(hud).add_child(weapon);
    label(
        &mut cmd,
        weapon,
        Label::Weapon,
        14.0,
        AMBER,
        Node::default(),
    );
    label(&mut cmd, weapon, Label::Ammo, 34.0, WHITE, Node::default());
    label(
        &mut cmd,
        weapon,
        Label::Trigger,
        13.0,
        CYAN,
        Node::default(),
    );
    text(
        &mut cmd,
        weapon,
        "1  NEEDLE   2  STINGER   R  RELOAD",
        12.0,
        MUTED,
        Node::default(),
    );
    text(
        &mut cmd,
        hud,
        "F1-F4 / TAB  SWITCH    B  LINK INPUT    T  TARGET    G  FOCUS    C  ORDER    H  CONTROLS    ESC  PAUSE",
        12.0,
        MUTED,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: percent(10),
            ..default()
        },
    );
    let damage = cmd
        .spawn((DamageOverlay, full_node(), BackgroundColor(Color::NONE)))
        .id();
    cmd.entity(hud).add_child(damage);

    let pause = cmd
        .spawn((
            Panel::Pause,
            full_node(),
            BackgroundColor(Color::srgba(0.009, 0.02, 0.03, 0.94)),
        ))
        .id();
    let inner = cmd
        .spawn(Node {
            width: px(490),
            margin: UiRect::all(Val::Auto),
            flex_direction: FlexDirection::Column,
            row_gap: px(16),
            ..default()
        })
        .id();
    cmd.entity(pause).add_child(inner);
    text(
        &mut cmd,
        inner,
        "FLIGHT SUSPENDED",
        34.0,
        WHITE,
        Node {
            margin: UiRect::bottom(px(12)),
            ..default()
        },
    );
    button(&mut cmd, inner, "RESUME FLIGHT / ESC", Action::Resume, true);
    button(&mut cmd, inner, "RESTART BATTLE", Action::Restart, false);
    button(&mut cmd, inner, "RETURN TO SQUAD LAB", Action::Menu, false);
    button(
        &mut cmd,
        inner,
        "CONTROLS + FLIGHT BRIEFING / H",
        Action::Help,
        false,
    );
    text(
        &mut cmd,
        inner,
        "Click once for three rounds. Move during the reset.\nLead moving targets and compensate for projectile drop.\nB links inputs; F1-F4 / Tab changes the viewed craft.",
        15.0,
        MUTED,
        Node {
            margin: UiRect::top(px(8)),
            ..default()
        },
    );

    let end = cmd
        .spawn((
            Panel::End,
            full_node(),
            BackgroundColor(Color::srgba(0.009, 0.02, 0.03, 0.94)),
        ))
        .id();
    let inner = cmd
        .spawn(Node {
            width: px(510),
            margin: UiRect::all(Val::Auto),
            flex_direction: FlexDirection::Column,
            row_gap: px(18),
            ..default()
        })
        .id();
    cmd.entity(end).add_child(inner);
    text(
        &mut cmd,
        inner,
        "E X P E R I M E N T   C O M P L E T E",
        13.0,
        CYAN,
        Node::default(),
    );
    label(
        &mut cmd,
        inner,
        Label::EndTitle,
        48.0,
        WHITE,
        Node::default(),
    );
    label(
        &mut cmd,
        inner,
        Label::EndStats,
        18.0,
        MUTED,
        Node {
            margin: UiRect::bottom(px(16)),
            ..default()
        },
    );
    button(
        &mut cmd,
        inner,
        "DEPLOY AGAIN / ENTER",
        Action::Restart,
        true,
    );
    button(&mut cmd, inner, "RETURN TO SQUAD LAB", Action::Menu, false);

    let telemetry = cmd
        .spawn((
            Panel::Telemetry,
            Node {
                position_type: PositionType::Absolute,
                right: px(30),
                top: px(105),
                width: px(305),
                padding: UiRect::all(px(16)),
                flex_direction: FlexDirection::Column,
                row_gap: px(9),
                border: UiRect::right(px(2)),
                ..default()
            },
            BorderColor::all(CYAN),
            BackgroundColor(Color::srgba(0.012, 0.026, 0.035, 0.94)),
        ))
        .id();
    text(
        &mut cmd,
        telemetry,
        "N E U R A L   T E L E M E T R Y",
        14.0,
        CYAN,
        Node::default(),
    );
    label(
        &mut cmd,
        telemetry,
        Label::Telemetry,
        12.0,
        WHITE,
        Node::default(),
    );
    for (i, name) in [
        "VISUAL SENSORY",
        "VISUAL PROCESSING",
        "CENTRAL COMPLEX",
        "DESCENDING",
        "MOTOR NEURONS",
        "OTHER NEURONS",
    ]
    .iter()
    .enumerate()
    {
        text(&mut cmd, telemetry, name, 11.0, MUTED, Node::default());
        let bg = cmd
            .spawn((
                Node {
                    width: percent(100),
                    height: px(4),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.18, 0.21)),
            ))
            .id();
        cmd.entity(telemetry).add_child(bg);
        let bar = cmd
            .spawn((
                NeuralBar(i),
                Node {
                    width: percent(0),
                    height: percent(100),
                    ..default()
                },
                BackgroundColor(if i == 5 { AMBER } else { CYAN }),
            ))
            .id();
        cmd.entity(bg).add_child(bar);
    }
    text(
        &mut cmd,
        telemetry,
        "Shared measured wiring. Independent activity.\nSimplified LIF + engineered motor readout.\nAim and combat use a game autopilot.",
        12.0,
        MUTED,
        Node::default(),
    );
    let help = cmd
        .spawn((
            Panel::Help,
            full_node(),
            BackgroundColor(Color::srgba(0.009, 0.02, 0.03, 0.98)),
        ))
        .id();
    let content = cmd
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            padding: UiRect::axes(vw(4), vh(5)),
            flex_direction: FlexDirection::Column,
            row_gap: px(20),
            ..default()
        })
        .id();
    cmd.entity(help).add_child(content);
    text(
        &mut cmd,
        content,
        "FLIGHT MANUAL",
        36.0,
        WHITE,
        Node::default(),
    );
    text(
        &mut cmd,
        content,
        "YOU command the cyan squad. Every red-eyed opponent is a neural-controlled fly.",
        17.0,
        CYAN,
        Node::default(),
    );
    let columns = cmd
        .spawn(Node {
            width: percent(100),
            column_gap: percent(2),
            ..default()
        })
        .id();
    cmd.entity(content).add_child(columns);
    help_column(
        &mut cmd,
        columns,
        "FLIGHT",
        &[
            ("W A S D", "Move through space"),
            ("MOUSE", "Aim and steer"),
            ("E", "Ascend / beside WASD"),
            ("Q", "Descend / beside WASD"),
            ("SHIFT", "Temporary boost"),
            ("RMB", "Precision aiming"),
            ("V", "First-person / chase camera"),
        ],
    );
    let combat = help_column(
        &mut cmd,
        columns,
        "BURST COMBAT",
        &[
            ("LMB CLICK", "One committed three-round burst"),
            ("RELEASE", "Re-arm after cooldown: 1.25 / 1.8s"),
            ("1 / 2", "Needle / heavy Stinger"),
            ("R", "Reload controlled craft"),
            ("T", "Cycle the marked opponent"),
        ],
    );
    text(
        &mut cmd,
        combat,
        "LEAD THE TARGET\nShots take time to travel and drop slightly. Aim ahead and above at range. Use cover and reposition during your reset.",
        16.0,
        AMBER,
        Node {
            margin: UiRect::top(px(16)),
            ..default()
        },
    );
    help_column(
        &mut cmd,
        columns,
        "SQUAD COMMAND",
        &[
            ("F1 - F4", "View a particular friendly craft"),
            ("TAB", "Next living craft"),
            ("SHIFT+TAB", "Previous living craft"),
            ("B", "Link all inputs / solo control"),
            ("C", "Solo wingmen: engage, regroup, hold"),
            ("G", "Solo wingmen: focus marked fly"),
            ("F9", "Inspect each fly's neural activity"),
        ],
    );
    text(
        &mut cmd,
        content,
        "LINKED: flight, aim, boost, burst fire and reload go to all living squad craft. Switching changes your viewpoint.\nSOLO: pilot the viewed craft while wingmen follow orders. C or G releases wingmen to autopilot; B links them again.",
        16.0,
        MUTED,
        Node::default(),
    );
    let (back, _) = button(&mut cmd, content, "BACK / ESC", Action::Help, true);
    cmd.entity(back).insert(Node {
        width: px(260),
        min_height: px(50),
        padding: UiRect::axes(px(18), px(12)),
        align_items: AlignItems::Center,
        ..default()
    });
}

#[allow(clippy::type_complexity)]
pub fn interact(
    mut buttons: Query<
        (
            &Interaction,
            &Action,
            &ButtonStyle,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Changed<Interaction>,
    >,
    mut g: ResMut<Game>,
    c: Res<Config>,
    mut brains: ResMut<FlyBrains>,
) {
    for (interaction, action, style, mut color, mut border) in &mut buttons {
        color.0 = match *interaction {
            Interaction::Pressed => AMBER,
            Interaction::Hovered => {
                if style.primary {
                    Color::srgb(0.5, 1.0, 0.94)
                } else {
                    Color::srgb(0.10, 0.24, 0.26)
                }
            }
            Interaction::None => {
                if style.primary {
                    CYAN
                } else {
                    Color::srgba(0.05, 0.10, 0.12, 0.85)
                }
            }
        };
        *border = BorderColor::all(if *interaction == Interaction::Hovered || style.primary {
            CYAN
        } else {
            Color::srgba(0.25, 0.42, 0.45, 0.7)
        });
        if *interaction == Interaction::Pressed {
            match action {
                Action::PilotBody => {
                    g.pilot_body = if g.pilot_body == BodyKind::Drone {
                        BodyKind::Fly
                    } else {
                        BodyKind::Drone
                    }
                }
                Action::Linked => g.linked = !g.linked,
                Action::Help => g.help = !g.help,
                Action::Drone => {
                    g.start_squad(true, &c);
                    brains.reset();
                }
                Action::Observe => {
                    g.start_squad(false, &c);
                    brains.reset();
                }
                Action::DroneCount => g.setup.drones = g.setup.drones % 4 + 1,
                Action::FlyCount => g.setup.flies = g.setup.flies % 4 + 1,
                Action::Resume => g.mode = Mode::Playing,
                Action::Restart => {
                    g.restart(&c);
                    brains.reset();
                }
                Action::Menu => g.mode = Mode::Menu,
            }
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_ui(
    time: Res<Time>,
    g: Res<Game>,
    c: Res<Config>,
    brains: Res<FlyBrains>,
    mut panels: Query<(&Panel, &mut Node)>,
    mut labels: Query<(&Label, &mut Text, &mut TextColor)>,
    mut meters: Query<(&Meter, &mut Node), Without<Panel>>,
    mut bars: Query<(&NeuralBar, &mut Node), (Without<Panel>, Without<Meter>)>,
    mut cross: Query<&mut BackgroundColor, (With<Crosshair>, Without<DamageOverlay>)>,
    mut damage: Query<&mut BackgroundColor, (With<DamageOverlay>, Without<Crosshair>)>,
    mut fps: Local<f32>,
) {
    *fps = *fps * 0.96 + (1.0 / time.delta_secs().max(0.001)) * 0.04;
    for (panel, mut node) in &mut panels {
        let on = match panel {
            Panel::Menu => g.mode == Mode::Menu && !g.help,
            Panel::Help => g.help,
            Panel::Hud => g.mode == Mode::Playing,
            Panel::Roster => g.squad && g.mode == Mode::Playing,
            Panel::Pause => g.mode == Mode::Paused && !g.help,
            Panel::End => g.mode == Mode::Finished,
            Panel::Telemetry => g.debug && g.mode == Mode::Playing,
        };
        node.display = if on { Display::Flex } else { Display::None };
    }
    let viewed = g.viewed_actor();
    let a = &g.actors[viewed];
    let neural_slot = g
        .selected_target
        .filter(|id| g.actors[*id].team == Team::Fly)
        .map(|id| g.actors[id].slot)
        .unwrap_or(if a.team == Team::Fly { a.slot } else { 0 });
    let brain = brains.get(neural_slot);
    for (tag, mut text, mut color) in &mut labels {
        let value = match *tag {
            Label::DroneCount => format!("YOUR {} {} [J]", g.pilot_body.plural(), g.setup.drones),
            Label::FlyCount => format!("NEURAL FLIES {} [K]", g.setup.flies),
            Label::Rule => format!(
                "FIRST TO {} TEAM KILLS / 3 MIN / NO FRIENDLY FIRE",
                c.score_to_win * g.setup.drones.max(g.setup.flies) as u32
            ),
            Label::Neural => format!(
                "{} OPPONENTS / {} INDEPENDENT NEURAL PILOTS\nEnemy craft are always flies.",
                g.setup.flies, g.setup.flies
            ),
            Label::PilotBody => format!("YOUR CRAFT: {} / SWITCH [P]", g.pilot_body.name()),
            Label::ControlMode => {
                if g.linked {
                    "LINKED INPUT: ALL CRAFT / SWITCH [B]".into()
                } else {
                    "SOLO PILOT + WINGMEN / SWITCH [B]".into()
                }
            }
            Label::Trigger => {
                if a.burst_remaining > 0 {
                    format!("BURST COMMITTED / {} LEFT", a.burst_remaining)
                } else if a.cooldown > 0.0 {
                    format!("BURST COOLDOWN {:.1}s", a.cooldown)
                } else if a.trigger_down {
                    "RELEASE TRIGGER TO RE-ARM".into()
                } else {
                    "READY / CLICK FOR THREE ROUNDS".into()
                }
            }
            Label::Score => format!("SQUAD {:02}  /  {:02} SWARM", g.scores[0], g.scores[1]),
            Label::Strength => format!(
                "AIRBORNE {}/{} : {}/{}   /   GOAL {}",
                g.actors
                    .iter()
                    .filter(|a| a.team == Team::Drone && a.alive())
                    .count(),
                g.team_count(Team::Drone),
                g.actors
                    .iter()
                    .filter(|a| a.team == Team::Fly && a.alive())
                    .count(),
                g.team_count(Team::Fly),
                g.score_limit(&c)
            ),
            Label::Timer => {
                if g.sudden_death {
                    "OVERTIME".into()
                } else {
                    format!(
                        "{:02}:{:02}",
                        g.remaining.ceil() as u32 / 60,
                        g.remaining.ceil() as u32 % 60
                    )
                }
            }
            Label::Health => format!("{:03.0}  INTEGRITY", a.hp),
            Label::Boost => {
                if a.boost > 0.0 {
                    "BOOST ENGAGED".into()
                } else if a.boost_lock > 0.0 {
                    format!("BOOST RECHARGING {:.1}s", a.boost_lock)
                } else {
                    "BOOST READY / SHIFT".into()
                }
            }
            Label::Weapon => c.weapons[a.weapon].name.clone(),
            Label::Ammo => {
                if a.reload > 0.0 {
                    format!("RELOAD {:.1}", a.reload)
                } else {
                    format!(
                        "{:02} / {:02}",
                        a.ammo[a.weapon], c.weapons[a.weapon].magazine
                    )
                }
            }
            Label::Flight => format!(
                "{} {} / {:.1}m/s / {:.0}m / {}",
                a.kind.name(),
                a.slot + 1,
                a.vel.length(),
                a.pos.y,
                if g.human.is_none() {
                    "OBSERVE"
                } else if g.chase {
                    "CHASE"
                } else {
                    "FPV"
                }
            ),
            Label::Notice => {
                if g.announce_time > 0.0 {
                    g.announcement.clone()
                } else {
                    String::new()
                }
            }
            Label::Enemy => {
                if let Some(t) = g.selected_target {
                    let enemy = &g.actors[t];
                    format!(
                        "LOCK {} {} / {:.0}m / {:.0}HP",
                        enemy.kind.name(),
                        enemy.slot + 1,
                        a.pos.distance(enemy.pos),
                        enemy.hp
                    )
                } else {
                    "NO VISIBLE COMBATANTS / T TO CYCLE".into()
                }
            }
            Label::Respawn => {
                if !a.alive() {
                    format!("RECONSTRUCTING {:.1}s", a.respawn)
                } else if a.shield > 0.0 {
                    "SPAWN SHIELD ACTIVE".into()
                } else {
                    String::new()
                }
            }
            Label::Roster(slot) => {
                if let Some(id) = g.actor_id(Team::Drone, slot) {
                    let unit = &g.actors[id];
                    color.0 = if g.human == Some(id) {
                        CYAN
                    } else if !unit.alive() {
                        MUTED
                    } else {
                        WHITE
                    };
                    if !unit.alive() {
                        format!(
                            "[F{}] {}{} REBUILD {:.1}s",
                            slot + 1,
                            unit.kind.name(),
                            slot + 1,
                            unit.respawn
                        )
                    } else {
                        format!(
                            "[F{}] {}{} {:03.0}HP {}",
                            slot + 1,
                            unit.kind.name(),
                            slot + 1,
                            unit.hp,
                            if g.human == Some(id) {
                                "VIEW"
                            } else if g.player_controls(id) {
                                "LINK"
                            } else {
                                "AUTO"
                            }
                        )
                    }
                } else {
                    String::new()
                }
            }
            Label::Orders => {
                if g.human.is_none() {
                    "OBSERVER / AUTONOMOUS SQUAD".into()
                } else if g.linked {
                    format!(
                        "LINKED INPUT / {} CRAFT\nB SOLO / C WINGMAN ORDERS",
                        g.team_count(Team::Drone)
                    )
                } else if let Some(id) = g.focus_target {
                    format!("FOCUS FLY {} / C CHANGE ORDER", g.actors[id].slot + 1)
                } else {
                    format!("ORDER {} / C", g.order.name())
                }
            }
            Label::Marker(slot) => {
                let enemy_team = if a.team == Team::Drone {
                    Team::Fly
                } else {
                    Team::Drone
                };
                if let Some(id) = g.actor_id(enemy_team, slot) {
                    let unit = &g.actors[id];
                    let local = a.rotation().inverse() * (unit.pos - a.pos);
                    format!(
                        "{} {}{}",
                        unit.kind.name(),
                        slot + 1,
                        if g.selected_target == Some(id) {
                            if local.z > 0.0 {
                                " / BEHIND"
                            } else {
                                " / LOCK"
                            }
                        } else {
                            ""
                        }
                    )
                } else {
                    String::new()
                }
            }
            Label::EndTitle => {
                if g.human.is_none() {
                    format!(
                        "{} VICTORY",
                        if g.winner == Some(Team::Drone) {
                            "SQUAD"
                        } else {
                            "SWARM"
                        }
                    )
                } else if g.winner == Some(a.team) {
                    "AIR SUPERIORITY".into()
                } else {
                    "SQUAD LOST".into()
                }
            }
            Label::EndStats => {
                let shots: u32 = g
                    .actors
                    .iter()
                    .filter(|b| b.team == a.team)
                    .map(|b| b.shots)
                    .sum();
                let hits: u32 = g
                    .actors
                    .iter()
                    .filter(|b| b.team == a.team)
                    .map(|b| b.hits)
                    .sum();
                format!(
                    "SQUAD {} : {} SWARM\n{} friendly craft / {} neural flies / {}m {:02}s\nTeam accuracy {:.0}% / {} confirmed hits",
                    g.scores[0],
                    g.scores[1],
                    g.team_count(Team::Drone),
                    g.team_count(Team::Fly),
                    g.elapsed as u32 / 60,
                    g.elapsed as u32 % 60,
                    hits as f32 / shots.max(1) as f32 * 100.0,
                    hits
                )
            }
            Label::Telemetry => {
                let mut status = format!(
                    "{:.0} FPS / 60Hz PHYSICS\nMALE CNS v1.0 / FLY {}\n{} CELLS / {} LINKS\n{} SYNAPSES / 5ms LIF\n{} INDEPENDENT PILOTS\n",
                    *fps,
                    neural_slot + 1,
                    brain.neurons(),
                    brain.edge_count,
                    brain.synapses,
                    g.team_count(Team::Fly)
                );
                for slot in 0..g.team_count(Team::Fly) {
                    status.push_str(&format!(
                        "NN{} {} SPIKES / {} STEPS / {:.0}Hz\n",
                        slot + 1,
                        brains.get(slot).spikes,
                        brains.get(slot).steps,
                        brains.get(slot).effective_hz
                    ));
                }
                status
            }
        };
        if text.0 != value {
            text.0 = value;
        }
    }
    for (meter, mut node) in &mut meters {
        node.width = percent(if meter.0 == 0 {
            a.hp / c.health * 100.0
        } else {
            (1.0 - a.boost_lock / (c.boost_cooldown + c.boost_duration)) * 100.0
        });
    }
    for (bar, mut node) in &mut bars {
        node.width = percent((brain.activity[bar.0] * 100.0).clamp(0.0, 100.0));
    }
    for mut color in &mut cross {
        color.0 = if g.hit_marker > 0.0 { AMBER } else { CYAN };
    }
    for mut color in &mut damage {
        color.0 = Color::srgba(0.85, 0.05, 0.02, g.damage_flash * 0.16);
    }
}
