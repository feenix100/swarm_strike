#[path = "../src/neural_core.rs"] mod neural_core;
use neural_core::{Graph,Circuit};
use std::{path::Path,sync::Arc,time::Instant};
fn main() {
    let args:Vec<String>=std::env::args().collect();
    let graph=Arc::new(Graph::load(Path::new(&args[1])).expect("Official MaleCNS file"));
    let steps=args.get(2).and_then(|v|v.parse::<usize>().ok()).unwrap_or(400);
    println!("neurons={} edges={} synapses={} shared_graph_mb={:.1}",graph.nodes.len(),graph.edge_count(),graph.synapses,graph.memory_bytes() as f64/1e6);
    for opponents in [1,4] {
        let start=Instant::now();
        let results=std::thread::scope(|s| {
            let jobs:Vec<_>=(0..opponents).map(|slot| {
                let g=Arc::clone(&graph);
                s.spawn(move || {
                    let mut c=Circuit::new(&g);
                    for _ in 0..200 {c.step(&g,[0.7,0.3,0.3]);}
                    let mut ms=Vec::with_capacity(steps);let mut total_spikes=0;let mut max_spikes=0;
                    for step in 0..steps {
                        let phase=(step as f32*0.025+slot as f32).sin();
                        let now=Instant::now();c.step(&g,[phase,0.3,0.3]);ms.push(now.elapsed().as_secs_f64()*1000.0);
                        total_spikes+=c.spikes;max_spikes=max_spikes.max(c.spikes);
                    }
                    let mean=ms.iter().sum::<f64>()/ms.len() as f64;
                    ms.sort_by(f64::total_cmp);
                    (mean,ms[ms.len()*95/100],total_spikes/steps,max_spikes,c.motor)
                })
            }).collect();jobs.into_iter().map(|j|j.join().unwrap()).collect::<Vec<_>>()
        });
        println!("opponents={opponents} wall_s={:.3} results={results:?}",start.elapsed().as_secs_f64());
    }
}
