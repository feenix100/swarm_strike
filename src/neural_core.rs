//! Official MaleCNS wiring in CSR form. Electrical dynamics and interfaces below
//! are game-model assumptions; the imported file retains original synapse counts.
use std::{fs, path::Path};

pub const STEP_SECONDS: f32 = 0.005;
#[derive(Clone, Copy)]
pub struct Node {
    pub body_id: u64,
    pub group: usize,
    pub side: i8,
    pub sign: i8,
}
#[derive(Clone, Copy)]
struct Edge {
    post: u32,
    weight: f32,
}
pub struct Graph {
    pub nodes: Vec<Node>,
    offsets: Vec<usize>,
    edges: Vec<Edge>,
    group_counts: [usize; 6],
    pub synapses: u64,
}
impl Graph {
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| {
            format!(
                "{}: {e}. Run tools/import_malecns.py to prepare the official dataset.",
                path.display()
            )
        })?;
        Self::from_bytes(&data)
    }
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        fn u32_at(d: &[u8], p: usize) -> u32 {
            u32::from_le_bytes(d[p..p + 4].try_into().unwrap())
        }
        fn u64_at(d: &[u8], p: usize) -> u64 {
            u64::from_le_bytes(d[p..p + 8].try_into().unwrap())
        }
        if data.len() < 24 || &data[..4] != b"SWMC" || u32_at(data, 4) != 1 {
            return Err("Invalid MaleCNS format/version".into());
        }
        let (n, e) = (u32_at(data, 8) as usize, u32_at(data, 12) as usize);
        let expected = 24usize
            .checked_add(n.checked_mul(12).ok_or("Size overflow")?)
            .and_then(|v| v.checked_add((n + 1).checked_mul(4)?))
            .and_then(|v| v.checked_add(e.checked_mul(8)?))
            .ok_or("Size overflow")?;
        if n == 0 || data.len() != expected {
            return Err("Invalid MaleCNS file length".into());
        }
        let mut nodes = Vec::with_capacity(n);
        let mut group_counts = [0; 6];
        for i in 0..n {
            let p = 24 + i * 12;
            let node = Node {
                body_id: u64_at(data, p),
                group: data[p + 8] as usize,
                side: data[p + 9] as i8,
                sign: data[p + 10] as i8,
            };
            if node.group >= 6
                || !(-1..=1).contains(&node.side)
                || !(-1..=1).contains(&node.sign)
                || nodes
                    .last()
                    .is_some_and(|last: &Node| last.body_id >= node.body_id)
            {
                return Err("Invalid MaleCNS neuron metadata".into());
            }
            group_counts[node.group] += 1;
            nodes.push(node);
        }
        let p = 24 + n * 12;
        let offsets: Vec<usize> = (0..=n).map(|i| u32_at(data, p + i * 4) as usize).collect();
        if offsets[0] != 0 || offsets[n] != e || offsets.windows(2).any(|w| w[0] > w[1]) {
            return Err("Invalid MaleCNS offsets".into());
        }
        let p = p + (n + 1) * 4;
        let mut incoming = vec![0.0f64; n];
        let mut synapses = 0u64;
        for i in 0..e {
            let (post, count) = (
                u32_at(data, p + i * 8) as usize,
                u32_at(data, p + i * 8 + 4),
            );
            if post >= n || count == 0 {
                return Err("Invalid MaleCNS connection".into());
            }
            incoming[post] += count as f64;
            synapses += count as u64;
        }
        if synapses != u64_at(data, 16) {
            return Err("MaleCNS synapse checksum mismatch".into());
        }
        let mut edges = Vec::with_capacity(e);
        for a in 0..n {
            for i in offsets[a]..offsets[a + 1] {
                let (post, count) = (
                    u32_at(data, p + i * 8) as usize,
                    u32_at(data, p + i * 8 + 4),
                );
                // The source file stores unsigned integer counts, never these
                // normalized/signed currents. Unknown/modulatory NT has no fast
                // current in this model. No connection is removed from storage.
                let weight =
                    count as f32 / incoming[post].max(1.0) as f32 * 5.0 * nodes[a].sign as f32;
                edges.push(Edge {
                    post: post as u32,
                    weight,
                });
            }
        }
        Ok(Self {
            nodes,
            offsets,
            edges,
            group_counts,
            synapses,
        })
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    pub fn memory_bytes(&self) -> usize {
        self.nodes.len() * std::mem::size_of::<Node>()
            + self.edges.len() * std::mem::size_of::<Edge>()
            + self.offsets.len() * std::mem::size_of::<usize>()
    }
}

pub struct Circuit {
    v: Vec<f32>,
    next: Vec<f32>,
    refractory: Vec<u8>,
    fired: Vec<usize>,
    pub activity: [f32; 6],
    pub motor: [f32; 3],
    pub steps: u64,
    pub spikes: usize,
}
impl Circuit {
    pub fn new(graph: &Graph) -> Self {
        let n = graph.nodes.len();
        Self {
            v: vec![0.0; n],
            next: vec![0.0; n],
            refractory: vec![0; n],
            fired: vec![],
            activity: [0.0; 6],
            motor: [0.0; 3],
            steps: 0,
            spikes: 0,
        }
    }
    pub fn reset(&mut self) {
        self.v.fill(0.0);
        self.next.fill(0.0);
        self.refractory.fill(0);
        self.fired.clear();
        self.activity = [0.0; 6];
        self.motor = [0.0; 3];
        self.steps = 0;
        self.spikes = 0;
    }
    pub fn step(&mut self, graph: &Graph, input: [f32; 3]) {
        self.next.fill(0.0);
        for &a in &self.fired {
            for edge in &graph.edges[graph.offsets[a]..graph.offsets[a + 1]] {
                self.next[edge.post as usize] += edge.weight;
            }
        }
        self.fired.clear();
        let mut totals = [0.0; 6];
        let mut lateral = [0.0; 2];
        let mut lateral_n = [0; 2];
        let mut vertical = [0.0; 2];
        let mut vertical_n = [0; 2];
        let [bearing, elevation, looming] = input;
        for (i, node) in graph.nodes.iter().enumerate() {
            if self.refractory[i] > 0 {
                self.refractory[i] -= 1;
                self.v[i] = 0.0;
                continue;
            }
            // Visual currents go only to official ol_sensory cells. The vertical
            // split by index is an engineered encoder, not anatomical retinotopy.
            let upper = if i % 2 == 0 { 1.0 } else { -1.0 };
            let external = if node.group == 0 {
                (0.14
                    + bearing * node.side as f32 * 0.10
                    + elevation * upper * 0.04
                    + looming * 0.06)
                    .clamp(0.0, 0.36)
            } else {
                0.10
            };
            self.v[i] = (self.v[i] * 0.91 + self.next[i] + external).clamp(-2.0, 2.0);
            let activity = self.v[i].max(0.0);
            totals[node.group] += activity;
            if node.group == 3 || node.group == 4 {
                if node.side != 0 {
                    let side = usize::from(node.side > 0);
                    lateral[side] += activity;
                    lateral_n[side] += 1;
                }
                let half = i % 2;
                vertical[half] += activity;
                vertical_n[half] += 1;
            }
            if self.v[i] >= 1.0 {
                self.v[i] = 0.0;
                self.refractory[i] = 3;
                self.fired.push(i);
            }
        }
        for (g, total) in totals.iter().enumerate() {
            self.activity[g] =
                self.activity[g] * 0.96 + total / graph.group_counts[g].max(1) as f32 * 0.04;
        }
        for i in 0..2 {
            lateral[i] /= lateral_n[i].max(1) as f32;
            vertical[i] /= vertical_n[i].max(1) as f32;
        }
        let target = [
            ((lateral[1] - lateral[0]) * 9.0).tanh(),
            ((vertical[0] - vertical[1]) * 5.0).tanh(),
            ((lateral[0] + lateral[1]) * 0.65).clamp(0.0, 1.0),
        ];
        for (value, target) in self.motor.iter_mut().zip(target) {
            *value += (target - *value) * 0.08;
        }
        self.steps += 1;
        self.spikes = self.fired.len();
    }
}

#[cfg(test)]
pub fn fixture() -> Graph {
    let mut nodes = vec![];
    for i in 0..8 {
        nodes.push(Node {
            body_id: 100 + i as u64,
            group: if i < 4 { 0 } else { 3 },
            side: if i % 2 == 0 { -1 } else { 1 },
            sign: 1,
        });
    }
    let edges = (0..8)
        .map(|i| Edge {
            post: ((i + 4) % 8) as u32,
            weight: 0.9,
        })
        .collect();
    Graph {
        nodes,
        offsets: (0..=8).collect(),
        edges,
        group_counts: [4, 0, 0, 4, 0, 0],
        synapses: 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wiring_propagates_and_states_are_independent() {
        let graph = fixture();
        let mut a = Circuit::new(&graph);
        let b = Circuit::new(&graph);
        let mut cut = fixture();
        for edge in &mut cut.edges {
            edge.weight = 0.0;
        }
        let mut ablated = Circuit::new(&cut);
        for _ in 0..200 {
            a.step(&graph, [0.9, 0.3, 0.3]);
            ablated.step(&cut, [0.9, 0.3, 0.3]);
        }
        assert_eq!(b.steps, 0);
        assert!(a.motor.iter().all(|v| v.is_finite()));
        assert!((a.motor[0] - ablated.motor[0]).abs() > 0.001);
        a.reset();
        assert_eq!(a.steps, 0);
        assert_eq!(a.motor, [0.0; 3]);
    }
    #[test]
    fn invalid_graphs_are_rejected() {
        assert!(Graph::from_bytes(b"SWMC").is_err());
        assert!(Graph::from_bytes(&[0; 24]).is_err());
    }

    #[test]
    fn full_official_graph_reacts_to_sensors_and_connections() {
        let mut graph = Graph::load(Path::new("assets/neural/malecns-v1.0.bin")).unwrap();
        let mut circuit = Circuit::new(&graph);
        for _ in 0..400 {
            circuit.step(&graph, [0.9, 0.3, 0.3]);
        }
        let right = circuit.motor;
        assert!(circuit.spikes > 0);
        circuit.reset();
        for _ in 0..400 {
            circuit.step(&graph, [-0.9, -0.3, 0.3]);
        }
        let left = circuit.motor;
        assert!((right[0] - left[0]).abs() > 0.001, "{right:?} {left:?}");
        for edge in &mut graph.edges {
            edge.weight = 0.0;
        }
        circuit.reset();
        for _ in 0..400 {
            circuit.step(&graph, [0.9, 0.3, 0.3]);
        }
        assert!(
            right
                .iter()
                .zip(circuit.motor)
                .map(|(a, b)| (a - b).abs())
                .sum::<f32>()
                > 0.01
        );
    }
}
