"""Validate bundled STL surfaces and neural graph integrity; no dependencies."""
from pathlib import Path
from collections import Counter
import json
import math
import struct
import hashlib

ROOT=Path(__file__).resolve().parents[1]

def stl(path):
    raw=path.read_bytes(); n=struct.unpack_from('<I',raw,80)[0]
    assert len(raw)==84+n*50,(path,'incorrect byte length')
    edges=Counter(); direction=Counter(); degenerate=0
    for i in range(n):
        values=struct.unpack_from('<12fH',raw,84+i*50)
        assert all(math.isfinite(v) for v in values)
        normal=values[:3]; assert abs(sum(x*x for x in normal)-1)<1e-4
        points=[tuple(round(x,6) for x in values[j:j+3]) for j in (3,6,9)]
        assert len(set(points))==3,(path,'degenerate triangle')
        for j in range(3):
            a,b=points[j],points[(j+1)%3]; key=tuple(sorted((a,b)))
            edges[key]+=1; direction[key]+=1 if a<b else -1
    # Geometry has intentionally overlapping closed parts, so cap edges that
    # coincide may have multiplicity 4. No exposed edges or inconsistent winding.
    assert all(count%2==0 for count in edges.values()),(path,'exposed boundary edges',Counter(edges.values()))
    assert all(d==0 for d in direction.values()),(path,'inconsistent winding',sum(d!=0 for d in direction.values()))
    return {'file':path.name,'triangles':n,'closed_component_edges':True,'consistent_winding':True}

if __name__=='__main__':
    report=[stl(p) for p in (ROOT/'assets/models').glob('*.stl')]
    path=ROOT/'assets/neural/malecns-v1.0.bin'
    provenance=json.loads((path.parent/'provenance.json').read_text())
    with path.open('rb') as f:
        magic,version,n,e,synapses=struct.unpack('<4sIIIQ',f.read(24))
    assert magic==b'SWMC' and version==1
    assert (n,e,synapses)==(166700,25582938,124177617)
    assert path.stat().st_size==24+n*12+(n+1)*4+e*8
    with path.open('rb') as f: assert hashlib.file_digest(f,'sha256').hexdigest()==provenance['output_sha256']
    print(json.dumps({'models':report,'circuit':{'dataset':'MaleCNS v1.0','neurons':n,'edges':e,'synapses':synapses,'checksum_matches':True}},indent=2))
