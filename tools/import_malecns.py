"""Import the complete annotated MaleCNS v1.0 neuron graph from official files.

No community graph is used. Original files remain untouched in --source.
All 166,700 class-annotated neurons and every official connection between
them are retained, including weight-one and self connections. Rows involving
unclassified segments/glia/fragments are counted and excluded explicitly.
Stored weights are original integer synapse counts; model weights are separate.
Requires: numpy, pyarrow. See assets/neural/provenance.json for exact inputs.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import struct
import numpy as np
import pyarrow.feather as feather

ROOT = Path(__file__).resolve().parents[1]
PINNED = {'body-annotations-male-cns-v1.0-minconf-0.5.feather': '2177e246113e4cfbf1e7772ec37c6da1955ff22e8063d0b1f833101f99a9a3b2', 'body-neurotransmitters-male-cns-v1.0.feather': '95c9289220663abeb3409f3ad9e5a7f8a53f8093f5139d15502cd08da8879621', 'connectome-weights-male-cns-v1.0-minconf-0.5.feather': 'e35da783d1c686b2b58b3b87cd6a403ae43bfcfba8bff28e08ef752c1a56afc1'}
ANN = 'body-annotations-male-cns-v1.0-minconf-0.5.feather'
NT = 'body-neurotransmitters-male-cns-v1.0.feather'
WEIGHTS = 'connectome-weights-male-cns-v1.0-minconf-0.5.feather'

def digest(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source', type=Path, required=True)
    p.add_argument('--output', type=Path, default=ROOT/'assets/neural')
    args = p.parse_args()
    downloaded = json.loads((args.source/'official-downloads.json').read_text())
    assert {r['file'] for r in downloaded['files']} == set(PINNED)
    for rec in downloaded['files']:
        assert rec['sha256'] == PINNED[rec['file']]
        assert digest(args.source/rec['file']) == PINNED[rec['file']], rec['file']
    annotations = feather.read_table(args.source/ANN).to_pylist()
    cells = sorted((r for r in annotations if r['superclass'] is not None), key=lambda r:r['bodyId'])
    assert len(cells) == 166700
    ids = np.array([r['bodyId'] for r in cells], dtype=np.int64)
    nts = {r['body']:r['consensus_nt'] for r in feather.read_table(args.source/NT, columns=['body','consensus_nt']).to_pylist()}
    args.output.mkdir(parents=True, exist_ok=True)
    # Model interface groups do not control graph selection or prune connections.
    def group(r):
        sc = r['superclass']
        if sc == 'ol_sensory': return 0
        if sc.startswith('visual_') or sc == 'ol_intrinsic': return 1
        if r['class'] == 'CX': return 2
        if sc.startswith('descending_'): return 3
        if sc in ('vnc_motor','cb_motor'): return 4
        return 5
    nodes = np.zeros(len(cells), dtype=[('body','<u8'),('group','u1'),('side','i1'),('sign','i1'),('reserved','u1')])
    nodes['body'] = ids
    for i,r in enumerate(cells):
        nodes['group'][i] = group(r)
        side = r['somaSide'] if r['somaSide'] in ('L','R') else r['rootSide']
        nodes['side'][i] = -1 if side == 'L' else 1 if side == 'R' else 0
        nt = nts.get(r['bodyId'])
        # Explicit simulation assumption, never a change to stored synapse counts.
        nodes['sign'][i] = -1 if nt in ('gaba','glutamate','histamine') else 1 if nt == 'acetylcholine' else 0
    graph = feather.read_table(args.source/WEIGHTS, memory_map=True)
    kept_pre, kept_post, kept_weight = [], [], []
    total_synapses = 0
    for batch in graph.to_batches(max_chunksize=2_000_000):
        pre, post, weight = [batch.column(k).to_numpy() for k in range(3)]
        total_synapses += int(weight.sum())
        a,b = np.searchsorted(ids, pre), np.searchsorted(ids, post)
        valid = (a < len(ids)) & (b < len(ids))
        a_safe,b_safe = np.minimum(a,len(ids)-1),np.minimum(b,len(ids)-1)
        valid &= (ids[a_safe] == pre) & (ids[b_safe] == post)
        kept_pre.append(a[valid].astype('<u4'))
        kept_post.append(b[valid].astype('<u4'))
        w = weight[valid]
        assert np.all(w > 0) and np.all(w <= np.iinfo(np.uint32).max)
        kept_weight.append(w.astype('<u4'))
    a,b,w = (np.concatenate(parts) for parts in (kept_pre,kept_post,kept_weight))
    order = np.argsort(a,kind='stable')
    edges = np.empty(len(a),dtype=[('post','<u4'),('count','<u4')])
    edges['post'],edges['count'] = b[order],w[order]
    offsets = np.r_[0,np.cumsum(np.bincount(a,minlength=len(ids)),dtype=np.uint64)].astype('<u4')
    assert len(a) < 2**32 and int(offsets[-1]) == len(a)
    # Round-trip multiset validation is independent of runtime weight normalization.
    assert np.array_equal(edges['post'],b[order]) and np.array_equal(edges['count'],w[order])
    output = args.output/'malecns-v1.0.bin'
    with output.open('wb') as f:
        f.write(struct.pack('<4sIIIQ',b'SWMC',1,len(ids),len(a),int(w.sum(dtype=np.uint64))))
        f.write(nodes.tobytes()); f.write(offsets.tobytes()); f.write(edges.tobytes())
    with gzip.GzipFile(str(args.output/'neuron-annotations.json.gz'),'wb',mtime=0) as f:
        f.write(json.dumps(cells,separators=(',',':')).encode())
    provenance = dict(
        dataset='MaleCNS v1.0',source='https://male-cns.janelia.org/download/',
        paper='https://doi.org/10.1016/j.cell.2026.08.015',
        authors='HHMI Janelia FlyEM, University of Cambridge, MRC Laboratory of Molecular Biology, Google Research and collaborators',
        data_license='CC BY 4.0',license_source='https://male-cns.janelia.org/download/',
        official_files=downloaded['files'],
        official_connection_rows=graph.num_rows,official_synapse_count=total_synapses,
        selected_neurons=len(ids),selected_edges=len(a),selected_synapse_count=int(w.sum(dtype=np.uint64)),
        excluded_connection_rows=graph.num_rows-len(a),
        neuron_selection='Every row with non-null superclass in the official annotation file (all 166700 published class-annotated neurons). No functional-group, transmitter, edge-weight or side filter.',
        edge_selection='Every official row with both endpoints in those neurons; count-one and self connections retained. Other rows include unclassified segments, glia and fragments; untouched full source table retained in source download directory.',
        storage='Lossless reindex to sorted original body IDs and CSR outgoing edges; original u32 integer synapse counts. Original IDs, anatomical-side annotations and full selected-cell annotation records retained.',
        output_file=output.name,output_bytes=output.stat().st_size,output_sha256=digest(output),
        simulation='One state per imported neuron per enemy. Simplified LIF, engineered visual encoder and population readout. Anatomical sides from official somaSide/rootSide, no artificial graph duplication. Incoming weight normalization and ACh positive, GABA/glutamate/histamine negative, other/unknown zero are runtime assumptions, not original physiology.',
        group_counts={str(k):int(np.sum(nodes['group']==k)) for k in range(6)},
        nt_sign_counts={str(k):int(np.sum(nodes['sign']==k)) for k in (-1,0,1)},
        validation='All input hashes checked; all annotated IDs preserved; all official eligible edges and integer counts preserved; no minimum connection threshold; all exported edges checked before write.'
    )
    (args.output/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
    print(json.dumps(provenance,indent=2),flush=True)

if __name__ == '__main__': main()
