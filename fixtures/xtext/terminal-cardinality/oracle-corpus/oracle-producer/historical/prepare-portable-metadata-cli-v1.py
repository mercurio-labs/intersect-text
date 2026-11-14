"""Path-discovery-only portable CLI, separately pinned from original execution."""
import hashlib
import json
from pathlib import Path
HERE = Path(__file__).resolve().parent
OUT = HERE / 'portable-terminal-cardinality-v1/oracle-producer/metadata-uri-binding'
original = HERE / 'run-portable-metadata-binding-v1.py'
source = original.read_text(encoding='utf-8')
old = "HERE = Path(__file__).resolve().parent\nCORPUS = HERE / 'portable-terminal-cardinality-v1'\nADD = CORPUS / 'oracle-producer/metadata-uri-binding'\nOUT = HERE / 'metadata-binding-attempt1'\nJDK = Path('D:/dev/jdks/jdk21')\nCACHE = Path('C:/Users/MarkPetrotta/.m2/repository')\nOUT.mkdir(exist_ok=False)"
new = """import argparse
ADD = Path(__file__).resolve().parent
CORPUS = ADD.parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--jdk', type=Path, required=True)
parser.add_argument('--maven-cache', type=Path, required=True)
args = parser.parse_args()
OUT = args.output.resolve()
JDK = args.jdk.resolve()
CACHE = args.maven_cache.resolve()
assert OUT != CORPUS and CORPUS not in OUT.parents, 'Output must be NEW and outside immutable corpus'
OUT.mkdir(exist_ok=False)"""
assert source.count(old) == 1
for name, text in [('original-replay-producer-v1.py', source), ('replay.py', source.replace(old, new))]:
    with (OUT / name).open('x', encoding='utf-8', newline='\n') as f:
        f.write(text)
def pin(p):
    b = Path(p).read_bytes()
    return {'bytes': len(b), 'sha256': hashlib.sha256(b).hexdigest()}
receipt = {'state': 'new_portable_cli_frozen_path_discovery_only',
    'original_producer': pin(original), 'original_copy': pin(OUT / 'original-replay-producer-v1.py'),
    'portable_producer': pin(OUT / 'replay.py'), 'exact_replacement': {'old': old, 'new': new},
    'baseline_metadata_unchanged': True, 'normalization': None, 'execution': 'Original external workflow completed24fresh processes exact; this CLI adaptation awaits separate optional replay',
    'cli': 'python -B oracle-producer/metadata-uri-binding/replay.py --output NEW_EXTERNAL --jdk JDK21 --maven-cache MAVEN_CACHE'}
with (OUT / 'portable-cli-freeze.json').open('x', encoding='utf-8', newline='\n') as f:
    f.write(json.dumps(receipt, indent=2) + '\n')
print(json.dumps({'path': str(OUT / 'portable-cli-freeze.json'), **pin(OUT / 'portable-cli-freeze.json')}))
