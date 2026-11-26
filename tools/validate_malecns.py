"""Independently compare every imported edge with the original official table."""
import argparse, hashlib, json, struct
from pathlib import Path
import numpy as np
import pyarrow.feather as feather

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source',type=Path,required=True)
    p.add_argument('--graph',type=Path,default=Path(__file__).resolve().parents[1]/'assets/neural/malecns-v1.0.bin')
    args=p.parse_args()
    raw=np.memmap(args.graph,mode='r',dtype='u1')
    magic,version,n,e,synapses=struct.unpack_from('<4sIIIQ',raw)
    assert (magic,version)==(b'SWMC',1)
    nodes=np.ndarray((n,),dtype=[('body','<u8'),('group','u1'),('side','i1'),('sign','i1'),('reserved','u1')],buffer=raw,offset=24)
    offsets=np.ndarray((n+1,),dtype='<u4',buffer=raw,offset=24+n*12)
    edges=np.ndarray((e,),dtype=[('post','<u4'),('weight','<u4')],buffer=raw,offset=24+n*12+(n+1)*4)
    assert int(offsets[0])==0 and int(offsets[-1])==e and np.all(np.diff(offsets.astype(np.int64))>=0)
    assert len(raw)==24+n*12+(n+1)*4+e*8 and np.all(edges['post']<n)
    annotations=feather.read_table(args.source/'body-annotations-male-cns-v1.0-minconf-0.5.feather').to_pylist()
    expected_ids=np.array(sorted(r['bodyId'] for r in annotations if r['superclass'] is not None),dtype='<u8')
    assert np.array_equal(nodes['body'],expected_ids)
    dtype=[('pre','<u8'),('post','<u8'),('weight','<u4')]
    actual=np.empty(e,dtype=dtype)
    actual['pre']=np.repeat(nodes['body'],np.diff(offsets.astype(np.int64)))
    actual['post']=nodes['body'][edges['post']]
    actual['weight']=edges['weight']
    source=feather.read_table(args.source/'connectome-weights-male-cns-v1.0-minconf-0.5.feather',memory_map=True)
    expected_parts=[]
    for batch in source.to_batches(max_chunksize=1_000_000):
        pre,post,weight=[batch.column(i).to_numpy() for i in range(3)]
        keep=np.isin(pre,expected_ids)&np.isin(post,expected_ids)
        part=np.empty(int(keep.sum()),dtype=dtype)
        part['pre'],part['post'],part['weight']=pre[keep],post[keep],weight[keep]
        expected_parts.append(part)
    expected=np.concatenate(expected_parts)
    actual.sort(order=['pre','post','weight'])
    expected.sort(order=['pre','post','weight'])
    assert np.array_equal(actual,expected),'Edge multiset differs from official neuron graph'
    assert int(actual['weight'].sum(dtype=np.uint64))==synapses
    print(json.dumps(dict(neurons=n,connections=e,synapses=synapses,
        all_original_ids_match=True,all_official_neuron_edges_match=True,
        all_integer_synapse_counts_match=True,no_connection_threshold=True,
        count_one_edges=int(np.sum(actual['weight']==1)),self_connections=int(np.sum(actual['pre']==actual['post'])),
        graph_sha256=hashlib.file_digest(args.graph.open('rb'),'sha256').hexdigest()),indent=2))
if __name__=='__main__':main()
