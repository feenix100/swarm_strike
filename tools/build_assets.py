"""Reproducible fly meshes, closed STL components, and sounds.

Uses only the Python standard library. STL units are millimetres; render scale is
an artistic enlargement. Each component is a closed surface. Overlapping parts
are intentionally not boolean-unioned: this is a detailed display/game model.
"""
from pathlib import Path
import argparse
import collections
import gzip
import hashlib
import json
import math
import random
import struct
import wave

ROOT = Path(__file__).resolve().parents[1]
TAU = math.tau
random.seed(57)

def add(a, b): return tuple(x+y for x,y in zip(a,b))
def sub(a, b): return tuple(x-y for x,y in zip(a,b))
def mul(a, s): return tuple(x*s for x in a)
def cross(a,b): return (a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0])
def unit(a): return mul(a, 1/max(1e-12, math.sqrt(sum(x*x for x in a))))

class Solid:
    def __init__(self): self.triangles=[]
    def tri(self,a,b,c,mat):
        if sum(x*x for x in cross(sub(b,a),sub(c,a))) > 1e-18:
            self.triangles.append((a,b,c,mat))
    def sphere(self,p,r,mat,nu=32,nv=18):
        # Explicit poles avoid degenerate triangles and guarantee shared edges.
        rings=[]
        for j in range(1,nv):
            v=math.pi*j/nv
            rings.append([add(p,(r[0]*math.sin(v)*math.cos(TAU*i/nu),r[1]*math.cos(v),r[2]*math.sin(v)*math.sin(TAU*i/nu))) for i in range(nu)])
        top=add(p,(0,r[1],0)); bottom=add(p,(0,-r[1],0))
        for i in range(nu):
            k=(i+1)%nu
            self.tri(top,rings[0][k],rings[0][i],mat)
            self.tri(bottom,rings[-1][i],rings[-1][k],mat)
            for j in range(len(rings)-1):
                a,b=rings[j][i],rings[j][k]; c,d=rings[j+1][i],rings[j+1][k]
                self.tri(a,b,c,mat); self.tri(b,d,c,mat)
    def tube(self,a,b,ra,rb,mat,n=10):
        axis=unit(sub(b,a)); tangent=unit(cross(axis,(0,1,0) if abs(axis[1])<.9 else (1,0,0))); bitangent=cross(axis,tangent)
        rings=[[add(p,add(mul(tangent,r*math.cos(TAU*i/n)),mul(bitangent,r*math.sin(TAU*i/n)))) for i in range(n)] for p,r in [(a,ra),(b,rb)]]
        for i in range(n):
            k=(i+1)%n
            self.tri(a,rings[0][k],rings[0][i],mat); self.tri(b,rings[1][i],rings[1][k],mat)
            self.tri(rings[0][i],rings[0][k],rings[1][i],mat); self.tri(rings[0][k],rings[1][k],rings[1][i],mat)
    def write(self,path):
        path.parent.mkdir(parents=True,exist_ok=True)
        with path.open('wb') as f:
            f.write(b'SWARMSTRIKE original procedural Drosophila | mm'.ljust(80,b' '))
            f.write(struct.pack('<I',len(self.triangles)))
            for a,b,c,mat in self.triangles:
                normal=unit(cross(sub(b,a),sub(c,a)))
                f.write(struct.pack('<12fH',*normal,*a,*b,*c,mat))

def build_fly():
    from fly_model import build
    build(Solid, ROOT)

def sounds():
    path=ROOT/'assets/audio'; path.mkdir(parents=True,exist_ok=True)
    for name,duration,freq in [('rifle',.11,780),('cannon',.38,155),('hit',.14,1500),('kill',.55,520),('boost',.50,100),('reload',.25,350)]:
        samples=[]; rate=22050
        for i in range(int(rate*duration)):
            t=i/rate; env=(1-t/duration)**2
            val=(math.sin(TAU*(freq*t-0.4*freq*t*t/duration))*.6+random.uniform(-1,1)*.4)*env*.23
            if name=='kill': val=math.sin(TAU*(440+int(t*7)*110)*t)*env*.20
            samples.append(struct.pack('<h',int(val*32767)))
        with wave.open(str(path/(name+'.wav')),'wb') as f:
            f.setnchannels(1); f.setsampwidth(2); f.setframerate(rate); f.writeframes(b''.join(samples))

if __name__=='__main__':
    p=argparse.ArgumentParser(); p.add_argument('--models-only', action='store_true'); args=p.parse_args()
    build_fly()
    if not args.models_only: sounds()
