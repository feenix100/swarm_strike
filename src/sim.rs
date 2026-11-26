use crate::neural::FlyBrains;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Weapon {
    pub name: String,
    pub damage: f32,
    pub interval: f32,
    pub speed: f32,
    pub magazine: u32,
    pub reload: f32,
    #[serde(default = "three")]
    pub burst_size: u32,
    #[serde(default = "burst_pause")]
    pub burst_cooldown: f32,
    #[serde(default = "bullet_gravity")]
    pub gravity: f32,
}
#[derive(Clone, Deserialize)]
pub struct Obstacle {
    pub center: [f32; 3],
    pub size: [f32; 3],
}
impl Obstacle {
    pub fn pos(&self) -> Vec3 {
        Vec3::from_array(self.center)
    }
    pub fn half(&self) -> Vec3 {
        Vec3::from_array(self.size) * 0.5
    }
}
#[derive(Resource, Clone, Deserialize)]
pub struct Config {
    pub arena: [f32; 3],
    #[serde(default = "four")]
    pub drone_count: usize,
    #[serde(default = "four")]
    pub fly_count: usize,
    pub health: f32,
    pub speed: f32,
    pub boost_speed: f32,
    pub acceleration: f32,
    pub boost_duration: f32,
    pub boost_cooldown: f32,
    pub respawn_seconds: f32,
    pub match_seconds: f32,
    pub score_to_win: u32,
    pub mouse_sensitivity: f32,
    pub weapons: [Weapon; 2],
    pub obstacles: Vec<Obstacle>,
}
impl Default for Config {
    fn default() -> Self {
        serde_json::from_str(include_str!("../assets/config.json")).unwrap()
    }
}
impl Config {
    pub fn load() -> Self {
        let config = std::fs::read_to_string("assets/config.json")
            .ok()
            .and_then(|s| serde_json::from_str::<Self>(&s).ok())
            .unwrap_or_default();
        assert!(
            config.arena.iter().all(|x| x.is_finite() && *x > 10.0)
                && config.health > 0.0
                && config.match_seconds > 0.0
                && config.score_to_win > 0,
            "Invalid arena or match config"
        );
        assert!(
            config.weapons.iter().all(|w| w.magazine > 0
                && w.reload > 0.0
                && w.interval > 0.0
                && w.speed > 0.0
                && w.damage > 0.0
                && w.burst_size == 3
                && w.burst_cooldown > 0.0
                && w.gravity.is_finite()
                && w.gravity >= 0.0),
            "Invalid weapon config"
        );
        config
    }
}
fn three() -> u32 {
    3
}
fn burst_pause() -> f32 {
    1.25
}
fn bullet_gravity() -> f32 {
    5.5
}
fn four() -> usize {
    4
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Drone,
    Fly,
}
impl BodyKind {
    pub fn name(self) -> &'static str {
        if self == Self::Drone { "DRONE" } else { "FLY" }
    }
    pub fn plural(self) -> &'static str {
        if self == Self::Drone {
            "DRONES"
        } else {
            "FLIES"
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Team {
    Drone,
    Fly,
}
impl Team {
    pub fn index(self) -> usize {
        usize::from(self == Self::Fly)
    }
    pub fn name(self) -> &'static str {
        if self == Self::Drone { "DRONE" } else { "FLY" }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquadOrder {
    Engage,
    Regroup,
    Hold,
}
impl SquadOrder {
    pub fn name(self) -> &'static str {
        match self {
            Self::Engage => "ENGAGE",
            Self::Regroup => "REGROUP",
            Self::Hold => "HOLD POSITION",
        }
    }
}
#[derive(Clone, Copy)]
pub struct SquadSetup {
    pub drones: usize,
    pub flies: usize,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Menu,
    Playing,
    Paused,
    Finished,
}
#[derive(Resource, Default)]
pub struct PilotInput {
    pub movement: Vec3,
    pub fire: bool,
    pub boost: bool,
    pub reload: bool,
    pub stabilize: bool,
}
#[derive(Clone)]
pub struct Actor {
    pub team: Team,
    pub kind: BodyKind,
    pub slot: usize,
    pub hold_pos: Vec3,
    pub target: Option<usize>,
    pub retarget: f32,
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub hp: f32,
    pub score: u32,
    pub weapon: usize,
    pub ammo: [u32; 2],
    pub reload: f32,
    pub reload_weapon: usize,
    pub cooldown: f32,
    pub boost: f32,
    pub boost_lock: f32,
    pub respawn: f32,
    pub shield: f32,
    pub shots: u32,
    pub hits: u32,
    pub burst_remaining: u32,
    pub burst_timer: f32,
    pub burst_weapon: usize,
    pub burst_direction: Vec3,
    pub trigger_down: bool,
    pub aim_lock: f32,
}
impl Actor {
    fn new(team: Team, slot: usize, count: usize, c: &Config) -> Self {
        Self {
            team,
            kind: if team == Team::Drone {
                BodyKind::Drone
            } else {
                BodyKind::Fly
            },
            slot,
            hold_pos: spawn(team, slot, count),
            target: None,
            retarget: 0.0,
            pos: spawn(team, slot, count),
            vel: Vec3::ZERO,
            yaw: if team == Team::Drone {
                0.0
            } else {
                std::f32::consts::PI
            },
            pitch: 0.0,
            hp: c.health,
            score: 0,
            weapon: 0,
            ammo: [c.weapons[0].magazine, c.weapons[1].magazine],
            reload: 0.0,
            reload_weapon: 0,
            cooldown: 0.0,
            boost: 0.0,
            boost_lock: 0.0,
            respawn: 0.0,
            shield: 1.5,
            shots: 0,
            hits: 0,
            burst_remaining: 0,
            burst_timer: 0.0,
            burst_weapon: 0,
            burst_direction: Vec3::NEG_Z,
            trigger_down: false,
            aim_lock: 0.0,
        }
    }
    pub fn rotation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(self.pitch)
    }
    pub fn forward(&self) -> Vec3 {
        self.rotation() * Vec3::NEG_Z
    }
    pub fn alive(&self) -> bool {
        self.respawn <= 0.0 && self.hp > 0.0
    }
}
pub fn spawn(team: Team, slot: usize, count: usize) -> Vec3 {
    if count == 1 {
        return if team == Team::Drone {
            Vec3::new(-28.0, 21.0, 35.0)
        } else {
            Vec3::new(28.0, 25.0, -35.0)
        };
    }
    let sign = if team == Team::Drone { 1.0 } else { -1.0 };
    Vec3::new(
        sign * (-31.0 + slot as f32 * 7.0),
        20.0 + (slot / 2) as f32 * 9.0,
        sign * (36.0 + (slot % 2) as f32 * 4.0),
    )
}
#[derive(Clone)]
pub struct Bullet {
    pub pos: Vec3,
    pub vel: Vec3,
    pub owner: usize,
    pub heavy: bool,
    pub ttl: f32,
    pub gravity: f32,
    pub trail: Vec<Vec3>,
}
#[derive(Clone)]
pub struct Spark {
    pub pos: Vec3,
    pub vel: Vec3,
    pub ttl: f32,
    pub max: f32,
    pub color: usize,
}
#[derive(Clone, Copy)]
pub enum Sound {
    Rifle,
    Cannon,
    Hit,
    Kill,
    Boost,
    Reload,
}
#[derive(Resource)]
pub struct Game {
    pub mode: Mode,
    pub human: Option<usize>,
    pub actors: Vec<Actor>,
    pub setup: SquadSetup,
    pub squad: bool,
    pub pilot_body: BodyKind,
    pub linked: bool,
    pub help: bool,
    pub order: SquadOrder,
    pub focus_target: Option<usize>,
    pub selected_target: Option<usize>,
    pub scores: [u32; 2],
    pub bullets: Vec<Bullet>,
    pub sparks: Vec<Spark>,
    pub remaining: f32,
    pub elapsed: f32,
    pub sudden_death: bool,
    pub winner: Option<Team>,
    pub chase: bool,
    pub debug: bool,
    pub muted: bool,
    pub hit_marker: f32,
    pub damage_flash: f32,
    pub announcement: String,
    pub announce_time: f32,
    pub sounds: Vec<Sound>,
    pub seed: u64,
    pub eliminations: u32,
    pub camera_shake: f32,
    pub input_grace: f32,
}
impl Game {
    pub fn new(c: &Config) -> Self {
        Self {
            mode: Mode::Menu,
            human: Some(0),
            actors: Self::make_actors(1, 1, c),
            setup: SquadSetup {
                drones: c.drone_count.clamp(1, 4),
                flies: c.fly_count.clamp(1, 4),
            },
            squad: false,
            pilot_body: BodyKind::Fly,
            linked: true,
            help: false,
            order: SquadOrder::Engage,
            focus_target: None,
            selected_target: Some(1),
            scores: [0; 2],
            bullets: vec![],
            sparks: vec![],
            remaining: c.match_seconds,
            elapsed: 0.0,
            sudden_death: false,
            winner: None,
            chase: true,
            debug: false,
            muted: false,
            hit_marker: 0.0,
            damage_flash: 0.0,
            announcement: String::new(),
            announce_time: 0.0,
            sounds: vec![],
            seed: 57012,
            eliminations: 0,
            camera_shake: 0.0,
            input_grace: 0.0,
        }
    }
    fn make_actors(drones: usize, flies: usize, c: &Config) -> Vec<Actor> {
        (0..drones)
            .map(|slot| Actor::new(Team::Drone, slot, drones, c))
            .chain((0..flies).map(|slot| Actor::new(Team::Fly, slot, flies, c)))
            .collect()
    }
    fn initialize(&mut self, c: &Config) {
        let setup = self.setup;
        let pilot_body = self.pilot_body;
        let linked = self.linked;
        let muted = self.muted;
        let debug = self.debug;
        *self = Self::new(c);
        self.setup = setup;
        self.pilot_body = pilot_body;
        self.linked = linked;
        self.muted = muted;
        self.debug = debug;
        self.mode = Mode::Playing;
        self.input_grace = 0.20;
    }
    /// A one-on-one round. The player is always on the friendly side.
    pub fn start(&mut self, human: Option<usize>, c: &Config) {
        self.initialize(c);
        self.human = human.map(|_| 0);
        self.debug |= human.is_none();
        self.actors[0].kind = self.pilot_body;
        self.chase = self.pilot_body == BodyKind::Fly;
        self.selected_target = Some(1);
        self.announce("BURST SYSTEMS ONLINE / LEAD YOUR TARGET");
    }
    pub fn start_squad(&mut self, manual: bool, c: &Config) {
        self.initialize(c);
        self.squad = true;
        self.setup.drones = self.setup.drones.clamp(1, 4);
        self.setup.flies = self.setup.flies.clamp(1, 4);
        self.actors = Self::make_actors(self.setup.drones, self.setup.flies, c);
        for a in &mut self.actors {
            if a.team == Team::Drone {
                a.kind = self.pilot_body;
            }
        }
        self.human = manual.then_some(0);
        self.selected_target = Some(self.setup.drones);
        self.chase = !manual || self.pilot_body == BodyKind::Fly;
        self.debug |= !manual;
        self.announce(if manual {
            "SQUAD ONLINE / B TO LINK OR UNLINK"
        } else {
            "NEURAL SWARM / AUTONOMOUS BATTLE"
        });
    }
    pub fn restart(&mut self, c: &Config) {
        let squad = self.squad;
        let human = self.human;
        if squad {
            self.start_squad(human.is_some(), c);
        } else {
            self.start(human, c);
        }
    }
    pub fn viewed_actor(&self) -> usize {
        self.human.unwrap_or_else(|| {
            self.actors
                .iter()
                .position(|a| a.team == Team::Fly && a.alive())
                .unwrap_or(self.team_count(Team::Drone))
        })
    }
    pub fn team_count(&self, team: Team) -> usize {
        self.actors.iter().filter(|a| a.team == team).count()
    }
    pub fn actor_id(&self, team: Team, slot: usize) -> Option<usize> {
        self.actors
            .iter()
            .position(|a| a.team == team && a.slot == slot)
    }
    pub fn score_limit(&self, c: &Config) -> u32 {
        c.score_to_win
            * if self.squad {
                self.setup.drones.max(self.setup.flies) as u32
            } else {
                1
            }
    }
    pub fn announce(&mut self, message: &str) {
        self.announcement = message.into();
        self.announce_time = 2.5;
    }
    pub fn select_craft(&mut self, slot: usize) -> bool {
        if !self.squad || self.human.is_none() {
            return false;
        }
        let Some(id) = self.actor_id(Team::Drone, slot) else {
            return false;
        };
        if !self.actors[id].alive() {
            self.announce(&format!(
                "{} {} RECONSTRUCTING",
                self.pilot_body.name(),
                slot + 1
            ));
            return false;
        }
        self.human = Some(id);
        self.announce(&format!(
            "CONTROL TRANSFER / {} {}",
            self.pilot_body.name(),
            slot + 1
        ));
        self.refresh_selected_target();
        true
    }
    pub fn cycle_craft(&mut self, backward: bool) {
        let Some(id) = self.human else {
            return;
        };
        if !self.squad {
            return;
        }
        let count = self.team_count(Team::Drone);
        let slot = self.actors[id].slot;
        for offset in 1..=count {
            let next = if backward {
                (slot + count - offset) % count
            } else {
                (slot + offset) % count
            };
            if let Some(candidate) = self.actor_id(Team::Drone, next)
                && self.actors[candidate].alive()
            {
                self.select_craft(next);
                return;
            }
        }
    }
    pub fn player_controls(&self, id: usize) -> bool {
        self.human == Some(id)
            || (self.squad
                && self.linked
                && self.human.is_some()
                && self.actors[id].team == Team::Drone)
    }
    pub fn toggle_linked(&mut self) {
        self.linked = !self.linked;
        self.announce(if self.linked {
            "LINKED CONTROL / ALL LIVING CRAFT FOLLOW INPUT"
        } else {
            "SOLO CONTROL / WINGMEN ON AUTOPILOT"
        });
    }
    pub fn set_weapon(&mut self, weapon: usize) {
        for id in 0..self.actors.len() {
            if self.player_controls(id) && self.actors[id].burst_remaining == 0 {
                self.actors[id].weapon = weapon;
            }
        }
    }
    pub fn apply_look(&mut self, yaw_delta: f32, pitch_delta: f32) {
        let Some(lead) = self.human else {
            return;
        };
        let yaw = (self.actors[lead].yaw + yaw_delta).rem_euclid(std::f32::consts::TAU);
        let pitch = (self.actors[lead].pitch + pitch_delta).clamp(-1.50, 1.50);
        for id in 0..self.actors.len() {
            if self.player_controls(id) {
                self.actors[id].yaw = yaw;
                self.actors[id].pitch = pitch;
            }
        }
    }
    pub fn cycle_order(&mut self) {
        if !self.squad || self.human.is_none() {
            return;
        }
        self.linked = false;
        self.order = match self.order {
            SquadOrder::Engage => SquadOrder::Regroup,
            SquadOrder::Regroup => SquadOrder::Hold,
            SquadOrder::Hold => SquadOrder::Engage,
        };
        self.focus_target = None;
        if self.order == SquadOrder::Hold {
            for a in &mut self.actors {
                a.hold_pos = a.pos;
            }
        }
        self.announce(&format!("SQUAD ORDER / {}", self.order.name()));
    }
    pub fn cycle_target(&mut self) {
        let team = self.actors[self.viewed_actor()].team;
        let candidates: Vec<_> = self
            .actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.team != team && a.alive())
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            self.selected_target = None;
            return;
        }
        let index = self
            .selected_target
            .and_then(|t| candidates.iter().position(|id| *id == t))
            .map_or(0, |i| (i + 1) % candidates.len());
        self.selected_target = Some(candidates[index]);
    }
    pub fn focus_fire(&mut self) {
        if !self.squad || self.human.is_none() {
            return;
        }
        self.refresh_selected_target();
        self.linked = false;
        self.focus_target = self.selected_target;
        self.order = SquadOrder::Engage;
        if let Some(t) = self.focus_target {
            let a = &self.actors[t];
            self.announce(&format!("FOCUS FIRE / {} {}", a.team.name(), a.slot + 1));
        }
    }
    fn refresh_selected_target(&mut self) {
        let i = self.viewed_actor();
        if self.selected_target.is_none_or(|t| {
            t >= self.actors.len()
                || !self.actors[t].alive()
                || self.actors[t].team == self.actors[i].team
        }) {
            self.selected_target = self.choose_target(i);
        }
        if self.focus_target.is_some_and(|t| !self.actors[t].alive()) {
            self.focus_target = None;
        }
    }
    fn choose_target(&self, i: usize) -> Option<usize> {
        let a = &self.actors[i];
        if a.team == Team::Drone
            && self.squad
            && self.order == SquadOrder::Engage
            && let Some(t) = self.focus_target
            && self.actors[t].alive()
            && self.actors[t].team != a.team
        {
            return Some(t);
        }
        self.actors
            .iter()
            .enumerate()
            .filter(|(_, b)| b.team != a.team && b.alive())
            .min_by(|(ia, b), (ib, d)| {
                let cost = |id, other: &Actor| {
                    a.pos.distance_squared(other.pos)
                        + if other.shield > 0.0 { 80.0 } else { 0.0 }
                        + self
                            .actors
                            .iter()
                            .filter(|ally| {
                                ally.team == a.team && ally.alive() && ally.target == Some(id)
                            })
                            .count() as f32
                            * 100.0
                };
                cost(*ia, b).total_cmp(&cost(*ib, d))
            })
            .map(|(id, _)| id)
    }
    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed % 100000) as f32 / 100000.0
    }
    pub fn reload(&mut self, i: usize, c: &Config) {
        let a = &mut self.actors[i];
        let w = &c.weapons[a.weapon];
        if a.reload <= 0.0 && a.burst_remaining == 0 && a.ammo[a.weapon] < w.magazine && a.alive() {
            a.reload = w.reload;
            a.reload_weapon = a.weapon;
            if self.human == Some(i) {
                self.sounds.push(Sound::Reload);
            }
        }
    }
    pub fn step(&mut self, dt: f32, input: &PilotInput, c: &Config, brains: &mut FlyBrains) {
        if self.mode != Mode::Playing {
            return;
        }
        self.elapsed += dt;
        self.remaining = (self.remaining - dt).max(0.0);
        self.input_grace = (self.input_grace - dt).max(0.0);
        self.hit_marker = (self.hit_marker - dt).max(0.0);
        self.damage_flash = (self.damage_flash - dt * 1.5).max(0.0);
        self.announce_time -= dt;
        self.camera_shake *= (-dt * 12.0).exp();
        brains.configure();
        // Select independently before stimulation: every fly encodes its own view.
        for i in 0..self.actors.len() {
            self.actors[i].retarget -= dt;
            let invalid = self.actors[i]
                .target
                .is_none_or(|t| !self.actors[t].alive());
            if invalid || self.actors[i].retarget <= 0.0 {
                self.actors[i].target = self.choose_target(i);
                self.actors[i].retarget = 0.30;
            }
            let a = &self.actors[i];
            if a.team == Team::Fly && a.alive() && self.human != Some(i) {
                let (bearing, elevation, looming) = if let Some(target) = a.target {
                    let d = self.actors[target].pos - a.pos;
                    let local = a.rotation().inverse() * d.normalize_or_zero();
                    // No target visual stimulus through solid cover.
                    if clear_line(a.pos, self.actors[target].pos, c, 0.0) {
                        (local.x, local.y, (1.0 - d.length() / 25.0).max(0.0))
                    } else {
                        (0.0, 0.0, 0.0)
                    }
                } else {
                    (0.0, 0.0, 0.0)
                };
                brains.brains[a.slot].advance(dt, bearing, elevation, looming);
            }
        }
        for i in 0..self.actors.len() {
            if self.actors[i].respawn > 0.0 {
                self.actors[i].respawn -= dt;
                if self.actors[i].respawn <= 0.0 {
                    let old = &self.actors[i];
                    let team = old.team;
                    let slot = old.slot;
                    let kind = old.kind;
                    let count = self.team_count(team);
                    let (score, shots, hits) = (old.score, old.shots, old.hits);
                    self.actors[i] = Actor::new(team, slot, count, c);
                    let a = &mut self.actors[i];
                    a.kind = kind;
                    a.trigger_down = input.fire;
                    a.score = score;
                    a.shots = shots;
                    a.hits = hits;
                    a.shield = 2.0;
                    if team == Team::Fly {
                        brains.brains[slot].reset();
                    }
                }
                continue;
            }
            let is_human = self.player_controls(i);
            let motor = if self.actors[i].team == Team::Fly {
                brains.get(self.actors[i].slot).motor
            } else {
                Vec3::ZERO
            };
            let (movement, fire, boost, reload) = if is_human {
                let a = &self.actors[self.human.unwrap_or(i)];
                (
                    a.rotation() * Vec3::new(input.movement.x, 0.0, input.movement.z)
                        + Vec3::Y * input.movement.y,
                    input.fire,
                    input.boost,
                    input.reload,
                )
            } else {
                self.bot(i, dt, c, motor)
            };
            if reload {
                self.reload(i, c);
            }
            let a = &mut self.actors[i];
            a.cooldown = (a.cooldown - dt).max(0.0);
            a.shield = (a.shield - dt).max(0.0);
            a.boost_lock = (a.boost_lock - dt).max(0.0);
            if a.reload > 0.0 {
                a.reload -= dt;
                if a.reload <= 0.0 {
                    a.ammo[a.reload_weapon] = c.weapons[a.reload_weapon].magazine;
                }
            }
            if boost && a.boost_lock <= 0.0 && a.boost <= 0.0 {
                a.boost = c.boost_duration;
                a.boost_lock = c.boost_duration + c.boost_cooldown;
                if is_human {
                    self.sounds.push(Sound::Boost);
                }
            }
            a.boost = (a.boost - dt).max(0.0);
            let speed = if a.boost > 0.0 {
                c.boost_speed
            } else {
                c.speed
            };
            let desired = movement.clamp_length_max(1.0) * speed;
            a.vel = a.vel.move_towards(desired, c.acceleration * dt);
            a.pos += a.vel * dt;
            resolve_collision(a, c, 1.8);
            let new_press = if is_human {
                fire && !a.trigger_down
            } else {
                fire
            };
            a.trigger_down = if is_human { fire } else { false };
            if new_press {
                let direction = if is_human && self.linked {
                    let lead = &self.actors[self.human.unwrap_or(i)];
                    let depth = self
                        .selected_target
                        .map(|t| lead.pos.distance(self.actors[t].pos))
                        .unwrap_or(60.0)
                        .clamp(12.0, 85.0);
                    (lead.pos + lead.forward() * depth - self.actors[i].pos).normalize_or_zero()
                } else {
                    self.actors[i].forward()
                };
                self.begin_burst(i, c, direction);
            }
            self.tick_burst(i, dt, c);
        }
        self.separate_actors(c);
        let mut bullets = std::mem::take(&mut self.bullets);
        for mut b in bullets.drain(..) {
            let end = b.pos + b.vel * dt + Vec3::NEG_Y * (0.5 * b.gravity * dt * dt);
            b.vel.y -= b.gravity * dt;
            b.trail.insert(0, b.pos);
            b.trail.truncate(9);
            let team = self.actors[b.owner].team;
            let ground = if b.pos.y <= 0.0 {
                0.0
            } else if end.y <= 0.0 {
                b.pos.y / (b.pos.y - end.y)
            } else {
                1.01
            };
            let obstruction = c
                .obstacles
                .iter()
                .filter_map(|o| segment_box(b.pos, end, o.pos(), o.half()))
                .fold(ground, f32::min);
            let hit = self
                .actors
                .iter()
                .enumerate()
                .filter(|(_, a)| a.team != team && a.alive() && a.shield <= 0.0)
                .filter_map(|(id, a)| {
                    segment_sphere(b.pos, end, a.pos, 2.25 + if b.heavy { 0.25 } else { 0.0 })
                        .filter(|t| *t < obstruction)
                        .map(|t| (id, t))
                })
                .min_by(|(_, a), (_, b)| a.total_cmp(b));
            if let Some((target, t)) = hit {
                let point = b.pos.lerp(end, t);
                let damage = c.weapons[usize::from(b.heavy)].damage;
                self.actors[target].hp = (self.actors[target].hp - damage).max(0.0);
                self.actors[b.owner].hits += 1;
                if self.human == Some(b.owner) {
                    self.hit_marker = 0.20;
                    self.sounds.push(Sound::Hit);
                }
                if self.human == Some(target) {
                    self.damage_flash = 0.7;
                    self.camera_shake = 0.18;
                }
                self.burst(point, if b.heavy { 24 } else { 9 }, team.index());
                if self.actors[target].hp <= 0.0 {
                    self.actors[target].respawn = c.respawn_seconds;
                    self.actors[target].burst_remaining = 0;
                    self.actors[b.owner].score += 1;
                    self.scores[team.index()] += 1;
                    self.eliminations += 1;
                    self.burst(point, 42, team.index());
                    self.sounds.push(Sound::Kill);
                    let victim = &self.actors[target];
                    let text = format!("{} {} ELIMINATED", victim.kind.name(), victim.slot + 1);
                    self.announce(&text);
                }
            } else if obstruction <= 1.0 {
                self.burst(b.pos.lerp(end, obstruction), 5, team.index());
            } else {
                b.pos = end;
                b.ttl -= dt;
                if b.ttl > 0.0 {
                    self.bullets.push(b);
                }
            }
        }
        for s in &mut self.sparks {
            s.pos += s.vel * dt;
            s.vel *= 0.96;
            s.ttl -= dt;
        }
        self.sparks.retain(|s| s.ttl > 0.0);
        if self.sparks.len() > 512 {
            let excess = self.sparks.len() - 512;
            self.sparks.drain(..excess);
        }
        // Resolve the entire physics step once. Simultaneous final kills tie
        // into overtime; projectile iteration order can never choose a winner.
        self.resolve_match(c);
        if self.squad
            && let Some(i) = self.human
            && !self.actors[i].alive()
        {
            self.cycle_craft(false);
        }
        self.refresh_selected_target();
    }
    fn resolve_match(&mut self, c: &Config) {
        let reached = self
            .scores
            .iter()
            .any(|score| *score >= self.score_limit(c));
        if reached || self.remaining <= 0.0 || self.sudden_death {
            if self.scores[0] == self.scores[1] {
                if !self.sudden_death {
                    self.announce("OVERTIME / NEXT TEAM LEAD WINS");
                }
                self.sudden_death = true;
            } else {
                self.winner = Some(if self.scores[0] > self.scores[1] {
                    Team::Drone
                } else {
                    Team::Fly
                });
                self.mode = Mode::Finished;
            }
        }
    }
    fn separate_actors(&mut self, c: &Config) {
        for i in 0..self.actors.len() {
            for j in i + 1..self.actors.len() {
                if !self.actors[i].alive() || !self.actors[j].alive() {
                    continue;
                }
                let delta = self.actors[j].pos - self.actors[i].pos;
                let distance = delta.length();
                if distance < 3.6 {
                    let n = if distance > 0.001 {
                        delta / distance
                    } else {
                        Vec3::X
                    };
                    let push = n * (3.6 - distance) * 0.5;
                    self.actors[i].pos -= push;
                    self.actors[j].pos += push;
                }
            }
        }
        for a in &mut self.actors {
            if a.alive() {
                resolve_collision(a, c, 1.8);
            }
        }
    }
    fn bot(&mut self, i: usize, dt: f32, c: &Config, neural: Vec3) -> (Vec3, bool, bool, bool) {
        let a = &self.actors[i];
        let Some(target_id) = a.target else {
            return (Vec3::ZERO, false, false, false);
        };
        let target = &self.actors[target_id];
        let delta = target.pos - a.pos;
        let distance = delta.length();
        let visible = clear_line(a.pos, target.pos, c, 0.0);
        let travel = distance / c.weapons[a.weapon].speed;
        let lead = target.pos
            + target.vel * travel * 0.72
            + Vec3::Y * (0.5 * c.weapons[a.weapon].gravity * travel * travel * 0.65);
        let aim = (lead - a.pos).normalize_or_zero();
        let desired_yaw = (-aim.x).atan2(-aim.z) + (self.elapsed * 1.9 + i as f32).sin() * 0.022;
        let desired_pitch = aim.y.asin() + (self.elapsed * 1.3 + i as f32).cos() * 0.015;
        let yaw_delta = angle_delta(a.yaw, desired_yaw);
        let pitch_delta = desired_pitch - a.pitch;
        let tangent = Vec3::new(-delta.z, 0.0, delta.x).normalize_or_zero();
        let phase = self.elapsed * 1.05 + i as f32 * 2.8;
        let mut move_dir = if visible {
            delta.normalize_or_zero() * ((distance - 27.0) / 15.0).clamp(-1.0, 1.0)
                + tangent * (0.65 + phase.sin() * 0.25)
                + Vec3::Y * (phase * 0.75).sin() * 0.42
        } else {
            self.route_towards(a.pos, target.pos, c)
        };
        if a.team == Team::Fly {
            move_dir += tangent * neural.x * 0.9 + Vec3::Y * neural.y * 0.7;
            move_dir *= 0.72 + neural.z * 0.5;
        }
        if self.squad
            && a.team == Team::Drone
            && let Some(pilot) = self.human
        {
            let anchor = match self.order {
                SquadOrder::Regroup => {
                    Some(self.actors[pilot].pos + Vec3::new((a.slot as f32 - 1.5) * 5.0, 3.0, -7.0))
                }
                SquadOrder::Hold => Some(a.hold_pos),
                SquadOrder::Engage => None,
            };
            if let Some(anchor) = anchor {
                move_dir = if anchor.distance(a.pos) > 3.0 {
                    self.route_towards(a.pos, anchor, c)
                } else {
                    (anchor - a.pos) * 0.25
                };
            }
        }
        for o in &c.obstacles {
            let near = a.pos.clamp(o.pos() - o.half(), o.pos() + o.half());
            let away = a.pos - near;
            let d = away.length();
            if d < 7.0 && d > 0.01 {
                move_dir += away / d * ((7.0 - d) / 3.0);
            }
        }
        for (other_id, other) in self.actors.iter().enumerate() {
            if other_id != i && other.alive() {
                let d = a.pos - other.pos;
                let length = d.length();
                if length < 6.0 && length > 0.01 {
                    move_dir += d / length * ((6.0 - length) / 4.0);
                }
            }
        }
        let danger = self.bullets.iter().any(|b| {
            self.actors[b.owner].team != a.team
                && b.pos.distance(a.pos) < 12.0
                && b.vel.dot(a.pos - b.pos) > 0.0
        });
        if danger && (a.team == Team::Fly || self.order != SquadOrder::Hold) {
            move_dir += Vec3::Y * (if a.pos.y < 32.0 { 1.3 } else { -1.3 }) + tangent * 0.8;
        }
        let aligned = visible
            && target.alive()
            && distance < 85.0
            && yaw_delta.abs() < 0.16
            && pitch_delta.abs() < 0.16;
        let reload = a.ammo[a.weapon] == 0;
        let aim_lock = if aligned { a.aim_lock + dt } else { 0.0 };
        let fire = aligned && aim_lock >= 0.25 && a.burst_remaining == 0 && a.cooldown <= 0.0;
        let is_fly = a.team == Team::Fly;
        let a = &mut self.actors[i];
        a.aim_lock = aim_lock;
        a.yaw += yaw_delta.clamp(-2.8 * dt, 2.8 * dt);
        a.pitch += pitch_delta.clamp(-2.2 * dt, 2.2 * dt);
        if a.reload <= 0.0 && a.burst_remaining == 0 {
            a.weapon = usize::from(distance > 40.0 && phase.sin() > 0.2);
        }
        let can_boost = is_fly || self.order != SquadOrder::Hold;
        (
            move_dir,
            fire,
            can_boost && (danger || distance > 60.0),
            reload,
        )
    }
    fn route_towards(&self, from: Vec3, target: Vec3, c: &Config) -> Vec3 {
        if clear_line(from, target, c, 2.3) {
            return (target - from).normalize_or_zero();
        }
        let mut goal = target;
        let mut best = f32::INFINITY;
        for o in &c.obstacles {
            let p = o.pos();
            let h = o.half() + Vec3::splat(5.0);
            for sx in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    for y in [from.y, p.y + h.y, p.y - h.y] {
                        let v = Vec3::new(p.x + sx * h.x, y, p.z + sz * h.z);
                        if v.y < 4.0 {
                            continue;
                        }
                        let cost = from.distance(v)
                            + v.distance(target)
                            + if clear_line(v, target, c, 2.3) {
                                0.0
                            } else {
                                35.0
                            };
                        if cost < best && clear_line(from, v, c, 2.3) {
                            best = cost;
                            goal = v;
                        }
                    }
                }
            }
        }
        (goal - from).normalize_or_zero()
    }
    fn begin_burst(&mut self, i: usize, c: &Config, direction: Vec3) {
        let a = &mut self.actors[i];
        if !a.alive() || a.cooldown > 0.0 || a.reload > 0.0 || a.burst_remaining > 0 {
            return;
        }
        if a.ammo[a.weapon] == 0 {
            self.reload(i, c);
            return;
        }
        a.burst_weapon = a.weapon;
        a.burst_remaining = c.weapons[a.weapon].burst_size.min(a.ammo[a.weapon]);
        a.burst_direction = direction;
        a.burst_timer = 0.0;
        a.aim_lock = 0.0;
    }
    fn tick_burst(&mut self, i: usize, dt: f32, c: &Config) {
        let a = &mut self.actors[i];
        if !a.alive() {
            a.burst_remaining = 0;
            return;
        }
        if a.burst_remaining == 0 {
            return;
        }
        a.burst_timer -= dt;
        while a.burst_remaining > 0 && a.burst_timer <= 0.0 {
            let w = &c.weapons[a.burst_weapon];
            let heavy = a.burst_weapon == 1;
            let dir = a.burst_direction;
            let pos = a.pos + dir * 2.7;
            a.ammo[a.burst_weapon] -= 1;
            a.shots += 1;
            a.burst_remaining -= 1;
            self.bullets.push(Bullet {
                pos,
                vel: dir * w.speed + a.vel * 0.25,
                owner: i,
                heavy,
                ttl: 3.0,
                gravity: w.gravity,
                trail: vec![pos],
            });
            a.burst_timer += w.interval;
            if self.human == Some(i) {
                self.camera_shake = if heavy { 0.10 } else { 0.025 };
                self.sounds
                    .push(if heavy { Sound::Cannon } else { Sound::Rifle });
            }
            if a.burst_remaining == 0 {
                a.cooldown = w.burst_cooldown;
            }
        }
    }
    fn burst(&mut self, p: Vec3, n: usize, color: usize) {
        for _ in 0..n {
            let v = Vec3::new(
                self.random() - 0.5,
                self.random() - 0.5,
                self.random() - 0.5,
            )
            .normalize_or_zero()
                * (3.0 + self.random() * 12.0);
            let ttl = 0.18 + self.random() * 0.55;
            self.sparks.push(Spark {
                pos: p,
                vel: v,
                ttl,
                max: ttl,
                color,
            });
        }
    }
}

pub fn angle_delta(a: f32, b: f32) -> f32 {
    (b - a + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
pub fn segment_box(a: Vec3, b: Vec3, center: Vec3, half: Vec3) -> Option<f32> {
    let d = b - a;
    let min = center - half;
    let max = center + half;
    let mut lo = 0.0f32;
    let mut hi = 1.0f32;
    for k in 0..3 {
        if d[k].abs() < 1e-6 {
            if a[k] < min[k] || a[k] > max[k] {
                return None;
            }
        } else {
            let p = (min[k] - a[k]) / d[k];
            let q = (max[k] - a[k]) / d[k];
            lo = lo.max(p.min(q));
            hi = hi.min(p.max(q));
            if lo > hi {
                return None;
            }
        }
    }
    Some(lo)
}
pub fn segment_sphere(a: Vec3, b: Vec3, p: Vec3, r: f32) -> Option<f32> {
    let d = b - a;
    let m = a - p;
    let aa = d.length_squared();
    let cc = m.length_squared() - r * r;
    if cc <= 0.0 {
        return Some(0.0);
    }
    if aa < 1e-9 {
        return None;
    }
    let bb = m.dot(d);
    let disc = bb * bb - aa * cc;
    if disc < 0.0 {
        return None;
    }
    let t = (-bb - disc.sqrt()) / aa;
    if (0.0..=1.0).contains(&t) {
        Some(t)
    } else {
        None
    }
}
pub fn clear_line(a: Vec3, b: Vec3, c: &Config, inflate: f32) -> bool {
    !c.obstacles
        .iter()
        .any(|o| segment_box(a, b, o.pos(), o.half() + Vec3::splat(inflate)).is_some())
}
fn resolve_collision(a: &mut Actor, c: &Config, r: f32) {
    // Open air: only the ground and freestanding cover are solid.
    if a.pos.y < r {
        a.pos.y = r;
        a.vel.y = a.vel.y.max(0.0);
    }
    for o in &c.obstacles {
        let near = a.pos.clamp(o.pos() - o.half(), o.pos() + o.half());
        let delta = a.pos - near;
        let distance = delta.length();
        if distance < r {
            let normal = if distance > 1e-5 {
                delta / distance
            } else {
                let local = a.pos - o.pos();
                let pen = o.half() - local.abs();
                let k = if pen.x < pen.y && pen.x < pen.z {
                    0
                } else if pen.y < pen.z {
                    1
                } else {
                    2
                };
                let mut v = Vec3::ZERO;
                v[k] = if local[k] >= 0.0 { 1.0 } else { -1.0 };
                a.pos[k] = o.pos()[k] + v[k] * (o.half()[k] + r);
                v
            };
            a.pos += normal * (r - distance);
            let dot = a.vel.dot(normal);
            if dot < 0.0 {
                a.vel -= normal * dot;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn human_flight_boost_reload_and_pause() {
        let mut c = Config::default();
        c.obstacles.clear();
        let mut g = Game::new(&c);
        g.start(Some(0), &c);
        let mut brain = FlyBrains::default();
        let start = g.actors[0].pos;
        let input = PilotInput {
            movement: Vec3::new(0.0, 1.0, -1.0),
            boost: true,
            ..default()
        };
        for _ in 0..60 {
            g.step(1.0 / 60.0, &input, &c, &mut brain);
        }
        assert!(g.actors[0].pos.distance(start) > 15.0);
        assert!(g.actors[0].vel.length() > c.speed);
        g.actors[0].ammo[0] = 0;
        g.reload(0, &c);
        for _ in 0..100 {
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brain);
        }
        assert_eq!(g.actors[0].ammo[0], c.weapons[0].magazine);
        g.mode = Mode::Paused;
        let previous = g.remaining;
        g.step(1.0, &input, &c, &mut brain);
        assert_eq!(g.remaining, previous);
    }
    #[test]
    fn swept_projectile_cannot_tunnel() {
        assert!(
            segment_sphere(
                Vec3::new(-50.0, 0.0, 0.0),
                Vec3::new(50.0, 0.0, 0.0),
                Vec3::ZERO,
                2.0
            )
            .is_some()
        );
        assert!(segment_box(Vec3::NEG_X * 50.0, Vec3::X * 50.0, Vec3::ZERO, Vec3::ONE).is_some());
    }
    #[test]
    fn cover_blocks_line_of_sight() {
        let c = Config::default();
        assert!(!clear_line(
            Vec3::new(-20.0, 15.0, 0.0),
            Vec3::new(20.0, 15.0, 0.0),
            &c,
            0.0
        ));
    }
    #[test]
    fn weapons_obey_magazine_and_cooldown() {
        let c = Config::default();
        let mut g = Game::new(&c);
        g.start(Some(0), &c);
        g.begin_burst(0, &c, Vec3::NEG_Z);
        g.tick_burst(0, 0.01, &c);
        assert_eq!(g.actors[0].ammo[0], 29);
        g.begin_burst(0, &c, Vec3::NEG_Z);
        assert_eq!(g.actors[0].burst_remaining, 2);
        g.tick_burst(0, c.weapons[0].interval * 2.0, &c);
        assert_eq!(g.actors[0].ammo[0], 27);
        assert!(g.actors[0].cooldown > 0.0);
        g.actors[0].cooldown = 0.0;
        g.actors[0].ammo[0] = 0;
        g.begin_burst(0, &c, Vec3::NEG_Z);
        assert_eq!(g.actors[0].reload, c.weapons[0].reload);
    }
    #[test]
    fn timeout_tie_enters_sudden_death() {
        let c = Config::default();
        let mut g = Game::new(&c);
        g.start(Some(0), &c);
        g.remaining = 0.001;
        g.step(0.02, &PilotInput::default(), &c, &mut FlyBrains::default());
        assert!(g.sudden_death);
        assert!(g.mode == Mode::Playing);
    }
    #[test]
    fn death_awards_score_and_respawn() {
        let mut c = Config::default();
        c.obstacles.clear();
        let mut g = Game::new(&c);
        g.start(Some(0), &c);
        g.actors[1].shield = 0.0;
        g.actors[1].hp = 10.0;
        g.actors[1].pos = Vec3::new(0.0, 20.0, 0.0);
        g.bullets.push(Bullet {
            pos: Vec3::new(0.0, 20.0, 3.0),
            vel: Vec3::NEG_Z * 120.0,
            owner: 0,
            heavy: false,
            ttl: 1.0,
            gravity: 0.0,
            trail: vec![],
        });
        let mut b = FlyBrains::default();
        g.step(0.02, &PilotInput::default(), &c, &mut b);
        assert_eq!(g.actors[0].score, 1);
        assert!(g.actors[1].respawn > 0.0);
        for _ in 0..160 {
            g.step(0.02, &PilotInput::default(), &c, &mut b);
        }
        assert!(g.actors[1].alive());
        assert!(g.actors[1].shield > 0.0);
    }
    #[test]
    fn bot_duel_makes_progress() {
        let c = Config::default();
        let mut g = Game::new(&c);
        g.start(None, &c);
        let mut b = FlyBrains::default();
        for _ in 0..7200 {
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut b);
        }
        assert!(g.actors.iter().all(|a| a.pos.is_finite()));
        assert!(g.eliminations > 0, "bots never eliminated one another");
    }
}

#[cfg(test)]
mod squad_tests {
    use super::*;
    fn squad(drones: usize, flies: usize) -> (Game, Config, FlyBrains) {
        let c = Config::default();
        let mut g = Game::new(&c);
        g.setup = SquadSetup { drones, flies };
        g.start_squad(true, &c);
        (g, c, FlyBrains::default())
    }
    #[test]
    fn every_one_to_four_combination_has_safe_distinct_spawns() {
        for drones in 1..=4 {
            for flies in 1..=4 {
                let (g, c, _) = squad(drones, flies);
                assert_eq!(g.actors.len(), drones + flies);
                assert_eq!(g.team_count(Team::Drone), drones);
                assert_eq!(g.team_count(Team::Fly), flies);
                assert_eq!(g.score_limit(&c), c.score_to_win * drones.max(flies) as u32);
                for (i, a) in g.actors.iter().enumerate() {
                    assert!(a.pos.y > 1.8 && a.pos.y < c.arena[1] - 1.8);
                    for o in &c.obstacles {
                        assert!(
                            segment_box(a.pos, a.pos, o.pos(), o.half() + Vec3::splat(1.8))
                                .is_none()
                        );
                    }
                    for other in &g.actors[i + 1..] {
                        assert!(a.pos.distance(other.pos) > 3.6);
                    }
                }
            }
        }
    }
    #[test]
    fn switching_preserves_each_drones_health_ammo_and_orientation() {
        let (mut g, _, _) = squad(4, 4);
        g.actors[0].ammo = [7, 2];
        g.actors[0].hp = 42.0;
        g.actors[0].yaw = 1.1;
        assert!(g.select_craft(3));
        assert_eq!(g.human, Some(3));
        assert!(g.select_craft(0));
        assert_eq!(g.actors[0].ammo, [7, 2]);
        assert_eq!(g.actors[0].hp, 42.0);
        assert_eq!(g.actors[0].yaw, 1.1);
        g.actors[1].hp = 0.0;
        g.actors[1].respawn = 3.0;
        g.cycle_craft(false);
        assert_eq!(g.human, Some(2));
        assert!(!g.select_craft(1));
        g.cycle_craft(true);
        assert_eq!(g.human, Some(0));
    }
    #[test]
    fn four_flies_advance_four_independent_required_neural_states() {
        let (mut g, c, mut brains) = squad(4, 4);
        for b in &mut brains.brains {
            b.enabled = false;
        }
        for _ in 0..8 {
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
        }
        assert!(brains.brains.iter().all(|b| b.enabled && b.steps > 0));
        for a in &g.actors[4..] {
            assert!(a.target.is_some_and(|t| g.actors[t].team == Team::Drone));
        }
        let other = brains.brains[1].steps;
        brains.brains[0].reset();
        assert_eq!(brains.brains[1].steps, other);
    }
    #[test]
    fn projectile_ignores_allies_and_hits_nearest_enemy() {
        let (mut g, mut c, mut brains) = squad(2, 2);
        c.obstacles.clear();
        for (i, a) in g.actors.iter_mut().enumerate() {
            a.shield = 0.0;
            a.pos = Vec3::new(-30.0 + i as f32 * 10.0, 20.0, 0.0);
        }
        g.actors[1].pos = Vec3::new(-3.0, 20.0, 0.0);
        g.actors[2].pos = Vec3::new(1.0, 20.0, 0.0);
        g.actors[3].pos = Vec3::new(9.0, 20.0, 0.0);
        g.bullets.push(Bullet {
            pos: Vec3::new(-9.0, 20.0, 0.0),
            vel: Vec3::X * 120.0,
            owner: 0,
            heavy: false,
            ttl: 2.0,
            gravity: 0.0,
            trail: vec![],
        });
        g.step(0.10, &PilotInput::default(), &c, &mut brains);
        assert_eq!(g.actors[1].hp, 100.0);
        assert_eq!(g.actors[2].hp, 88.0);
        assert_eq!(g.actors[3].hp, 100.0);
    }
    #[test]
    fn losing_active_drone_transfers_control_to_living_teammate() {
        let (mut g, mut c, mut brains) = squad(4, 4);
        c.obstacles.clear();
        g.actors[0].hp = 10.0;
        g.actors[0].shield = 0.0;
        g.bullets.push(Bullet {
            pos: g.actors[0].pos + Vec3::Z * 3.0,
            vel: Vec3::NEG_Z * 120.0,
            owner: 4,
            heavy: false,
            ttl: 1.0,
            gravity: 0.0,
            trail: vec![],
        });
        g.step(0.02, &PilotInput::default(), &c, &mut brains);
        assert_eq!(g.human, Some(1));
        assert_eq!(g.scores, [0, 1]);
        assert!(g.actors[0].respawn > 0.0);
    }
    #[test]
    fn focus_and_regroup_orders_apply_to_other_drones() {
        let (mut g, mut c, mut brains) = squad(4, 4);
        c.obstacles.clear();
        g.selected_target = Some(6);
        g.focus_fire();
        g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
        assert!(g.actors[1..4].iter().all(|a| a.target == Some(6)));
        g.cycle_order();
        assert_eq!(g.order, SquadOrder::Regroup);
        assert_eq!(g.focus_target, None);
        let anchor = g.actors[0].pos + Vec3::new(7.5, 3.0, -7.0);
        let before = g.actors[3].pos.distance(anchor);
        for _ in 0..30 {
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
        }
        assert!(g.actors[3].pos.distance(anchor) < before);
        g.cycle_order();
        assert_eq!(g.order, SquadOrder::Hold);
        assert_eq!(g.actors[2].hold_pos, g.actors[2].pos);
    }
    #[test]
    fn simultaneous_final_kills_enter_overtime_without_order_based_winner() {
        let mut c = Config::default();
        c.obstacles.clear();
        let mut g = Game::new(&c);
        g.start(None, &c);
        g.scores = [6, 6];
        for (i, a) in g.actors.iter_mut().enumerate() {
            a.pos = Vec3::new(if i == 0 { -10.0 } else { 10.0 }, 20.0, 0.0);
            a.hp = 10.0;
            a.shield = 0.0;
        }
        for owner in 0..2 {
            g.bullets.push(Bullet {
                owner,
                heavy: false,
                ttl: 1.0,
                pos: g.actors[1 - owner].pos + Vec3::Z * 3.0,
                vel: Vec3::NEG_Z * 120.0,
                gravity: 0.0,
                trail: vec![],
            });
        }
        let mut brains = FlyBrains::default();
        g.step(0.02, &PilotInput::default(), &c, &mut brains);
        assert_eq!(g.scores, [7, 7]);
        assert_eq!(g.winner, None);
        assert!(g.sudden_death);
        assert!(g.mode == Mode::Playing);
        for a in &mut g.actors {
            a.hp = 10.0;
            a.shield = 0.0;
            a.respawn = 0.0;
        }
        g.bullets.push(Bullet {
            owner: 0,
            heavy: false,
            ttl: 1.0,
            pos: g.actors[1].pos + Vec3::Z * 3.0,
            vel: Vec3::NEG_Z * 120.0,
            gravity: 0.0,
            trail: vec![],
        });
        g.step(0.02, &PilotInput::default(), &c, &mut brains);
        assert_eq!(g.scores, [8, 7]);
        assert_eq!(g.winner, Some(Team::Drone));
        assert!(g.mode == Mode::Finished);
    }
    #[test]
    fn maximum_squad_battle_progresses_and_team_scores_match_personal_kills() {
        let (mut g, mut c, mut brains) = squad(4, 4);
        c.match_seconds = 30.0;
        g.start_squad(false, &c);
        for _ in 0..60 * 35 {
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
            g.sounds.clear();
            if g.mode == Mode::Finished {
                break;
            }
        }
        assert!(g.eliminations > 4);
        assert!(g.actors.iter().all(|a| a.pos.is_finite()));
        for team in [Team::Drone, Team::Fly] {
            assert_eq!(
                g.scores[team.index()],
                g.actors
                    .iter()
                    .filter(|a| a.team == team)
                    .map(|a| a.score)
                    .sum::<u32>()
            );
        }
        assert!(brains.brains.iter().all(|b| b.steps > 0));
        assert!(g.bullets.iter().all(|b| b.owner < g.actors.len()));
    }
}

#[cfg(test)]
mod tactical_tests {
    use super::*;
    #[test]
    fn fly_is_default_and_the_pre_match_body_choice_survives_restart() {
        let c = Config::default();
        let mut g = Game::new(&c);
        assert_eq!(g.pilot_body, BodyKind::Fly);
        g.pilot_body = BodyKind::Drone;
        g.start_squad(true, &c);
        assert!(
            g.actors
                .iter()
                .filter(|a| a.team == Team::Drone)
                .all(|a| a.kind == BodyKind::Drone)
        );
        assert!(
            g.actors
                .iter()
                .filter(|a| a.team == Team::Fly)
                .all(|a| a.kind == BodyKind::Fly)
        );
        g.restart(&c);
        assert_eq!(g.pilot_body, BodyKind::Drone);
        g.pilot_body = BodyKind::Fly;
        g.start_squad(true, &c);
        assert!(g.actors.iter().all(|a| a.kind == BodyKind::Fly));
    }
    fn fixture(craft: usize, foes: usize, kind: BodyKind) -> (Game, Config, FlyBrains) {
        let mut c = Config::default();
        c.obstacles.clear();
        let mut g = Game::new(&c);
        g.setup = SquadSetup {
            drones: craft,
            flies: foes,
        };
        g.pilot_body = kind;
        g.start_squad(true, &c);
        for a in &mut g.actors {
            a.shield = 100.0;
        }
        (g, c, FlyBrains::default())
    }
    #[test]
    fn both_weapons_fire_three_rounds_per_click_and_never_repeat_while_held() {
        for weapon in 0..2 {
            let (mut g, c, mut brains) = fixture(1, 1, BodyKind::Drone);
            g.set_weapon(weapon);
            let held = PilotInput {
                fire: true,
                ..default()
            };
            for _ in 0..240 {
                g.step(1.0 / 60.0, &held, &c, &mut brains);
            }
            assert_eq!(g.actors[0].shots, 3);
            assert_eq!(g.actors[0].ammo[weapon], c.weapons[weapon].magazine - 3);
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
            for _ in 0..35 {
                g.step(1.0 / 60.0, &held, &c, &mut brains);
            }
            assert_eq!(g.actors[0].shots, 6);
        }
    }
    #[test]
    fn trigger_reset_cannot_be_bypassed_by_reclick_or_weapon_switch() {
        let (mut g, c, mut brains) = fixture(1, 1, BodyKind::Drone);
        let held = PilotInput {
            fire: true,
            ..default()
        };
        let burst_frames = (c.weapons[0].interval * 2.0 * 60.0).ceil() as usize + 1;
        for _ in 0..burst_frames {
            g.step(1.0 / 60.0, &held, &c, &mut brains);
        }
        assert_eq!(g.actors[0].shots, 3);
        assert!(g.actors[0].cooldown > 0.0);
        g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
        g.set_weapon(1);
        g.step(1.0 / 60.0, &held, &c, &mut brains);
        assert_eq!(g.actors[0].shots, 3);
        for _ in 0..90 {
            g.step(1.0 / 60.0, &held, &c, &mut brains);
        }
        assert_eq!(
            g.actors[0].shots, 3,
            "holding an early click must not queue another volley"
        );
    }
    #[test]
    fn linked_reload_refills_every_controlled_magazine() {
        let (mut g, c, mut brains) = fixture(4, 1, BodyKind::Drone);
        for _ in 0..30 {
            g.step(
                1.0 / 60.0,
                &PilotInput {
                    fire: true,
                    ..default()
                },
                &c,
                &mut brains,
            );
        }
        assert!(g.actors[..4].iter().all(|a| a.ammo[0] == 27));
        for _ in 0..100 {
            g.step(
                1.0 / 60.0,
                &PilotInput {
                    reload: true,
                    ..default()
                },
                &c,
                &mut brains,
            );
        }
        assert!(
            g.actors[..4]
                .iter()
                .all(|a| a.ammo[0] == c.weapons[0].magazine)
        );
    }
    #[test]
    fn linked_control_moves_and_fires_all_four_units_and_switches_only_the_view() {
        let (mut g, c, mut brains) = fixture(4, 4, BodyKind::Drone);
        let starts: Vec<_> = g.actors[..4].iter().map(|a| a.pos).collect();
        assert!(g.linked);
        g.apply_look(0.2, 0.1);
        let input = PilotInput {
            movement: Vec3::NEG_Z,
            fire: true,
            boost: true,
            ..default()
        };
        for _ in 0..50 {
            g.step(1.0 / 60.0, &input, &c, &mut brains);
        }
        for (id, start) in starts.iter().enumerate() {
            assert!(g.actors[id].pos.distance(*start) > 5.0);
            assert_eq!(g.actors[id].shots, 3);
            assert_eq!(g.actors[id].ammo[0], 27);
        }
        assert!(g.select_craft(3));
        assert_eq!(g.human, Some(3));
        assert!((0..4).all(|i| g.player_controls(i)));
        g.toggle_linked();
        assert_eq!((0..4).filter(|i| g.player_controls(*i)).count(), 1);
    }
    #[test]
    fn player_can_pilot_a_fly_but_every_opponent_is_still_an_independent_neural_fly() {
        let (mut g, c, mut brains) = fixture(1, 4, BodyKind::Fly);
        assert_eq!(g.human, Some(0));
        assert_eq!(g.actors[0].kind, BodyKind::Fly);
        assert!(
            g.actors[1..]
                .iter()
                .all(|a| a.team == Team::Fly && a.kind == BodyKind::Fly)
        );
        let start = g.actors[0].pos;
        let input = PilotInput {
            movement: Vec3::Y,
            fire: true,
            ..default()
        };
        for _ in 0..30 {
            g.step(1.0 / 60.0, &input, &c, &mut brains);
        }
        assert!(g.actors[0].pos.y > start.y + 2.0);
        assert_eq!(g.actors[0].shots, 3);
        assert!(brains.brains.iter().all(|b| b.steps > 0 && b.enabled));
    }
    #[test]
    fn ballistic_motion_matches_a_slight_downward_parabola() {
        let (mut g, c, mut brains) = fixture(1, 1, BodyKind::Drone);
        g.bullets.push(Bullet {
            pos: Vec3::new(0.0, 40.0, 0.0),
            vel: Vec3::X * 30.0,
            owner: 0,
            heavy: false,
            ttl: 3.0,
            gravity: 6.0,
            trail: vec![],
        });
        for _ in 0..30 {
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
        }
        let b = g.bullets.iter().find(|b| b.owner == 0).unwrap();
        assert!((b.pos.x - 15.0).abs() < 0.001);
        assert!((b.pos.y - 39.25).abs() < 0.002);
        assert!((b.vel.y + 3.0).abs() < 0.001);
        assert_eq!(b.trail.len(), 9);
        assert!(b.trail.windows(2).any(|p| p[0].y < p[1].y));
    }
    #[test]
    fn curved_projectiles_still_collide_with_cover() {
        let (mut g, mut c, mut brains) = fixture(1, 1, BodyKind::Drone);
        c.obstacles.push(Obstacle {
            center: [10.0, 39.7, 0.0],
            size: [1.0, 0.5, 4.0],
        });
        g.bullets.push(Bullet {
            pos: Vec3::new(0.0, 40.0, 0.0),
            vel: Vec3::X * 30.0,
            owner: 0,
            heavy: false,
            ttl: 3.0,
            gravity: 6.0,
            trail: vec![],
        });
        for _ in 0..30 {
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
        }
        assert!(!g.bullets.iter().any(|b| b.owner == 0));
    }
    #[test]
    fn rapid_clicking_still_leaves_a_full_cooldown_after_the_third_round() {
        let dt = 1.0 / 60.0;
        for weapon in 0..2 {
            let (mut g, c, mut brains) = fixture(1, 1, BodyKind::Drone);
            g.set_weapon(weapon);
            let mut shot_frames = vec![];
            for frame in 0..360 {
                let before = g.actors[0].shots;
                g.step(
                    dt,
                    &PilotInput {
                        fire: frame % 2 == 0,
                        ..default()
                    },
                    &c,
                    &mut brains,
                );
                if g.actors[0].shots > before {
                    shot_frames.push(frame);
                }
            }
            assert!(shot_frames.len() >= 6);
            for burst in shot_frames.chunks(3) {
                for gap in burst.windows(2) {
                    assert!((gap[1] - gap[0]) as f32 * dt >= c.weapons[weapon].interval - dt);
                }
            }
            for start in (3..shot_frames.len()).step_by(3) {
                let pause = (shot_frames[start] - shot_frames[start - 1]) as f32 * dt;
                assert!(pause >= c.weapons[weapon].burst_cooldown - dt);
            }
        }
    }
    #[test]
    fn both_player_craft_can_fly_past_the_old_edges_and_ceiling() {
        for kind in [BodyKind::Fly, BodyKind::Drone] {
            let (mut g, c, mut brains) = fixture(1, 1, kind);
            g.actors[0].pos = Vec3::new(48.0, 58.0, 48.0);
            for _ in 0..180 {
                g.step(
                    1.0 / 60.0,
                    &PilotInput {
                        movement: Vec3::ONE,
                        ..default()
                    },
                    &c,
                    &mut brains,
                );
            }
            let pos = g.actors[0].pos;
            assert!(pos.x > c.arena[0] * 0.5 + 20.0);
            assert!(pos.z > c.arena[2] * 0.5 + 20.0);
            assert!(pos.y > c.arena[1] + 20.0);
        }
    }
    #[test]
    fn projectiles_keep_flying_and_can_hit_beyond_the_old_arena() {
        let (mut g, c, mut brains) = fixture(1, 1, BodyKind::Fly);
        g.actors[0].pos = Vec3::new(45.0, 80.0, 0.0);
        g.actors[1].pos = Vec3::new(85.0, 80.0, 0.0);
        g.actors[1].shield = 0.0;
        g.bullets.push(Bullet {
            pos: Vec3::new(49.0, 80.0, 0.0),
            vel: Vec3::X * 60.0,
            owner: 0,
            heavy: false,
            ttl: 3.0,
            gravity: 0.0,
            trail: vec![],
        });
        for _ in 0..15 {
            g.step(1.0 / 60.0, &PilotInput::default(), &c, &mut brains);
        }
        assert!(g.bullets.iter().any(|b| b.owner == 0 && b.pos.x > 60.0));
        let round = g.bullets.iter().find(|b| b.owner == 0).unwrap();
        let target_pos = round.pos + Vec3::X * 3.0;
        g.actors[1].pos = target_pos;
        g.actors[1].vel = Vec3::ZERO;
        g.step(0.05, &PilotInput::default(), &c, &mut brains);
        assert_eq!(g.actors[1].hp, c.health - c.weapons[0].damage);
        g.bullets.push(Bullet {
            pos: Vec3::new(120.0, 0.2, 120.0),
            vel: Vec3::NEG_Y * 20.0,
            owner: 0,
            heavy: false,
            ttl: 3.0,
            gravity: 0.0,
            trail: vec![],
        });
        g.step(0.02, &PilotInput::default(), &c, &mut brains);
        assert!(!g.bullets.iter().any(|b| b.owner == 0 && b.pos.x > 100.0));
    }
}
