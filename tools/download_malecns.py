"""Download unmodified, public MaleCNS v1.0 files directly from Janelia's bucket."""
import concurrent.futures
import hashlib
import json
from pathlib import Path
import time
import urllib.request

DEST = Path(__file__).resolve().parents[1] / 'data' / 'malecns-v1.0'
BASE = 'https://storage.googleapis.com/flyem-male-cns/v1.0/connectome-data/flat-connectome/'
PINNED = {'body-annotations-male-cns-v1.0-minconf-0.5.feather': '2177e246113e4cfbf1e7772ec37c6da1955ff22e8063d0b1f833101f99a9a3b2', 'body-neurotransmitters-male-cns-v1.0.feather': '95c9289220663abeb3409f3ad9e5a7f8a53f8093f5139d15502cd08da8879621', 'connectome-weights-male-cns-v1.0-minconf-0.5.feather': 'e35da783d1c686b2b58b3b87cd6a403ae43bfcfba8bff28e08ef752c1a56afc1'}
FILES = [
    'body-annotations-male-cns-v1.0-minconf-0.5.feather',
    'body-neurotransmitters-male-cns-v1.0.feather',
    'connectome-weights-male-cns-v1.0-minconf-0.5.feather',
]

def download(name):
    path = DEST / name
    url = BASE + name
    req = urllib.request.Request(url, method='HEAD')
    with urllib.request.urlopen(req, timeout=60) as res:
        size = int(res.headers['Content-Length'])
        md5 = res.headers.get('x-goog-hash', '')
        generation = res.headers.get('x-goog-generation', '')
    if not path.exists() or path.stat().st_size != size:
        temp = path.with_suffix('.partial')
        start = time.perf_counter()
        with urllib.request.urlopen(url, timeout=120) as res, temp.open('wb') as out:
            last = 0
            while chunk := res.read(8 * 1024 * 1024):
                out.write(chunk)
                if out.tell() - last >= 128 * 1024 * 1024:
                    last = out.tell()
                    print(f'{name}: {last / 1e6:.0f}/{size / 1e6:.0f} MB', flush=True)
        assert temp.stat().st_size == size
        temp.replace(path)
        print(f'{name}: downloaded in {time.perf_counter()-start:.1f}s', flush=True)
    sha = hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()
    assert sha == PINNED[name], f'Official file changed: {name}; inspect the release before updating pins'
    return dict(file=name, url=url, bytes=size, sha256=sha,
                gcs_hash=md5, gcs_generation=generation)

if __name__ == '__main__':
    DEST.mkdir(parents=True, exist_ok=True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
        records = list(pool.map(download, FILES))
    manifest = dict(dataset='MaleCNS v1.0',
                    official_download_page='https://male-cns.janelia.org/download/',
                    files=records)
    (DEST / 'official-downloads.json').write_text(json.dumps(manifest, indent=2)+'\n')
    print(json.dumps(manifest, indent=2), flush=True)
