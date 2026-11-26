//! Independent full MaleCNS neuron states; immutable official wiring is shared.
//! Workers publish their latest output so neural cost cannot block fixed physics.
use crate::neural_core::{Circuit, Graph, STEP_SECONDS};
use bevy::prelude::*;
use std::sync::{Arc, OnceLock};
#[cfg(not(test))]
use std::{
    sync::{Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

fn shared_graph() -> Arc<Graph> {
    static GRAPH: OnceLock<Arc<Graph>> = OnceLock::new();
    Arc::clone(GRAPH.get_or_init(|| {
        #[cfg(test)]
        let graph = crate::neural_core::fixture();
        #[cfg(not(test))]
        let graph = {
            let path = std::env::var_os("SWARMSTRIKE_CONNECTOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("assets/neural/malecns-v1.0.bin"));
            let graph = Graph::load(&path).expect("Cannot load official MaleCNS connectome");
            eprintln!(
                "MaleCNS v1.0: {} neurons, {} connections, {} synapses; {:.1} MB shared wiring",
                graph.nodes.len(),
                graph.edge_count(),
                graph.synapses,
                graph.memory_bytes() as f64 / 1e6
            );
            graph
        };
        Arc::new(graph)
    }))
}

#[derive(Clone, Copy, Default)]
struct Snapshot {
    activity: [f32; 6],
    motor: [f32; 3],
    spikes: usize,
    steps: u64,
    hz: f32,
}
#[cfg(not(test))]
struct Mailbox {
    input: [f32; 3],
    snapshot: Snapshot,
    generation: u64,
    last_input: Instant,
    active: bool,
    stop: bool,
}
#[cfg(not(test))]
struct Worker {
    mailbox: Arc<(Mutex<Mailbox>, Condvar)>,
    handle: Option<JoinHandle<()>>,
}
#[cfg(not(test))]
impl Worker {
    fn new(graph: Arc<Graph>) -> Self {
        let mailbox = Arc::new((
            Mutex::new(Mailbox {
                input: [0.0; 3],
                snapshot: Snapshot::default(),
                generation: 0,
                last_input: Instant::now(),
                active: false,
                stop: false,
            }),
            Condvar::new(),
        ));
        let channel = Arc::clone(&mailbox);
        let handle = thread::Builder::new()
            .name("MaleCNS neural pilot".into())
            .spawn(move || {
                let mut circuit = Circuit::new(&graph);
                let mut generation = 0;
                let mut next_tick = Instant::now();
                let mut sample_start = Instant::now();
                let mut sample_steps = 0;
                let mut hz = 0.0;
                loop {
                    let (lock, wake) = &*channel;
                    let mut shared = lock.lock().expect("Neural mailbox");
                    while !shared.stop
                        && (!shared.active
                            || shared.last_input.elapsed() > Duration::from_millis(100))
                    {
                        shared = wake.wait(shared).expect("Neural wake");
                        next_tick = Instant::now();
                        sample_start = Instant::now();
                        sample_steps = circuit.steps;
                        hz = 0.0;
                    }
                    if shared.stop {
                        break;
                    }
                    if generation != shared.generation {
                        circuit.reset();
                        generation = shared.generation;
                        sample_start = Instant::now();
                        sample_steps = 0;
                        hz = 0.0;
                    }
                    let input = shared.input;
                    drop(shared);
                    circuit.step(&graph, input);
                    if sample_start.elapsed().as_secs_f32() >= 0.25 {
                        hz = (circuit.steps - sample_steps) as f32
                            / sample_start.elapsed().as_secs_f32();
                        sample_start = Instant::now();
                        sample_steps = circuit.steps;
                    }
                    let mut shared = lock.lock().expect("Neural result");
                    if shared.generation == generation {
                        shared.snapshot = Snapshot {
                            activity: circuit.activity,
                            motor: circuit.motor,
                            spikes: circuit.spikes,
                            steps: circuit.steps,
                            hz,
                        };
                    }
                    drop(shared);
                    next_tick += Duration::from_secs_f32(STEP_SECONDS);
                    let now = Instant::now();
                    if next_tick < now {
                        next_tick = now;
                    } else {
                        thread::sleep(next_tick - now);
                    }
                }
            })
            .expect("Create neural worker");
        Self {
            mailbox,
            handle: Some(handle),
        }
    }
    fn advance(&self, input: [f32; 3]) -> Snapshot {
        let (lock, wake) = &*self.mailbox;
        let mut shared = lock.lock().expect("Neural input");
        shared.input = input;
        shared.last_input = Instant::now();
        shared.active = true;
        let snapshot = shared.snapshot;
        drop(shared);
        wake.notify_one();
        snapshot
    }
    fn reset(&self) {
        let (lock, wake) = &*self.mailbox;
        let mut shared = lock.lock().expect("Reset neural state");
        shared.generation += 1;
        shared.snapshot = Snapshot::default();
        shared.active = false;
        drop(shared);
        wake.notify_one();
    }
}
#[cfg(not(test))]
impl Drop for Worker {
    fn drop(&mut self) {
        let (lock, wake) = &*self.mailbox;
        if let Ok(mut shared) = lock.lock() {
            shared.stop = true;
        }
        wake.notify_one();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

pub struct Brain {
    graph: Arc<Graph>,
    #[cfg(not(test))]
    worker: Option<Worker>,
    #[cfg(test)]
    circuit: Circuit,
    #[cfg(test)]
    clock: f32,
    pub edge_count: usize,
    pub synapses: u64,
    pub activity: [f32; 6],
    pub motor: Vec3,
    pub spikes: usize,
    pub steps: u64,
    pub effective_hz: f32,
    pub enabled: bool,
}
impl Brain {
    fn new(graph: Arc<Graph>) -> Self {
        Self {
            edge_count: graph.edge_count(),
            synapses: graph.synapses,
            #[cfg(not(test))]
            worker: None,
            #[cfg(test)]
            circuit: Circuit::new(&graph),
            #[cfg(test)]
            clock: 0.0,
            graph,
            activity: [0.0; 6],
            motor: Vec3::ZERO,
            spikes: 0,
            steps: 0,
            effective_hz: 0.0,
            enabled: true,
        }
    }
    pub fn neurons(&self) -> usize {
        self.graph.nodes.len()
    }
    pub fn reset(&mut self) {
        #[cfg(not(test))]
        if let Some(worker) = &self.worker {
            worker.reset();
        }
        #[cfg(test)]
        {
            self.circuit.reset();
            self.clock = 0.0;
        }
        self.activity = [0.0; 6];
        self.motor = Vec3::ZERO;
        self.spikes = 0;
        self.steps = 0;
        self.effective_hz = 0.0;
    }
    pub fn advance(&mut self, _dt: f32, bearing: f32, elevation: f32, looming: f32) {
        if !self.enabled {
            self.motor = Vec3::ZERO;
            return;
        }
        let input = [bearing, elevation, looming];
        #[cfg(not(test))]
        let state = self
            .worker
            .get_or_insert_with(|| Worker::new(Arc::clone(&self.graph)))
            .advance(input);
        // Small deterministic circuits isolate game rules from worker scheduling.
        // The full official graph is validated and benchmarked separately.
        #[cfg(test)]
        let state = {
            self.clock += _dt;
            while self.clock >= STEP_SECONDS {
                self.clock -= STEP_SECONDS;
                self.circuit.step(&self.graph, input);
            }
            Snapshot {
                activity: self.circuit.activity,
                motor: self.circuit.motor,
                spikes: self.circuit.spikes,
                steps: self.circuit.steps,
                hz: 200.0,
            }
        };
        self.activity = state.activity;
        self.motor = Vec3::from_array(state.motor);
        self.spikes = state.spikes;
        self.steps = state.steps;
        self.effective_hz = state.hz;
    }
}
impl Default for Brain {
    fn default() -> Self {
        Self::new(shared_graph())
    }
}

#[derive(Resource)]
pub struct FlyBrains {
    pub brains: Vec<Brain>,
}
impl Default for FlyBrains {
    fn default() -> Self {
        let graph = shared_graph();
        Self {
            brains: (0..4).map(|_| Brain::new(Arc::clone(&graph))).collect(),
        }
    }
}
impl FlyBrains {
    pub fn reset(&mut self) {
        for b in &mut self.brains {
            b.reset();
        }
    }
    pub fn configure(&mut self) {
        for b in &mut self.brains {
            b.enabled = true;
        }
    }
    pub fn get(&self, slot: usize) -> &Brain {
        &self.brains[slot.min(3)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_dataset_has_expected_dimensions_and_ids() {
        let graph = Graph::load(std::path::Path::new("assets/neural/malecns-v1.0.bin")).unwrap();
        assert_eq!(graph.nodes.len(), 166700);
        assert_eq!(graph.edge_count(), 25582938);
        assert_eq!(graph.synapses, 124177617);
        assert_eq!(graph.nodes[0].body_id, 10001);
        assert!(graph.nodes.iter().any(|n| n.sign < 0));
        assert!(graph.memory_bytes() < 230_000_000);
    }
    #[test]
    fn flies_share_wiring_and_keep_independent_activity() {
        let mut swarm = FlyBrains::default();
        assert!(Arc::ptr_eq(&swarm.brains[0].graph, &swarm.brains[1].graph));
        for _ in 0..50 {
            swarm.brains[0].advance(0.02, 0.9, 0.3, 0.3);
        }
        assert!(swarm.brains[0].steps > 0);
        assert_eq!(swarm.brains[1].steps, 0);
        assert_eq!(swarm.brains[1].motor, Vec3::ZERO);
        let before = swarm.brains[0].steps;
        swarm.brains[1].reset();
        assert_eq!(swarm.brains[0].steps, before);
    }
}
