"""Byteexact disjoint corpus extraction and NEW explicit logical URI binding addendum."""
import hashlib
import json
import os
import shutil
from pathlib import Path
HERE = Path(__file__).resolve().parent
TARGET = HERE.parent
OUT = HERE / 'portable-terminal-cardinality-v1'
OUT.mkdir(exist_ok=False)

def disk(p):
    p = Path(p).resolve()
    return p if str(p).startswith('\\\\?\\') else Path('\\\\?\\' + str(p))

def pin(p):
    b = disk(p).read_bytes()
    return {'bytes': len(b), 'sha256': hashlib.sha256(b).hexdigest()}

def read(p):
    return json.loads(disk(p).read_text(encoding='utf-8'))

def save(p, v):
    disk(p).parent.mkdir(parents=True, exist_ok=True)
    with disk(p).open('x', encoding='utf-8', newline='\n') as f:
        f.write(json.dumps(v, indent=2) + '\n')

copies = {}
def copy_tree(source, relative):
    for folder, _, names in os.walk(disk(source)):
        for name in names:
            src = Path(folder) / name
            rel = (Path(relative) / src.relative_to(disk(source))).as_posix()
            dest = disk(OUT / rel)
            dest.parent.mkdir(parents=True, exist_ok=True)
            assert not dest.exists(), rel
            shutil.copyfile(src, dest)
            expected = pin(src)
            assert pin(dest) == expected, rel
            copies[rel] = {'original_path': str(src), **expected}

cohorts = [
    ('original8', TARGET / 'terminal-cardinality-after-v62-v1/frozen-inputs-v1', TARGET / 'tc-full-oracle-v1'),
    ('typed8', TARGET / 'terminal-cardinality-after-v62-v1/typed-companion-inputs-v1', TARGET / 'tct-oracle-v1'),
    ('fragment2', TARGET / 'terminal-cardinality-after-v62-v1/fragment-header-companion-inputs-v1', TARGET / 'tcf-oracle-v1'),
    ('return12', TARGET / 'terminal-return-identity-after-v62-v1/frozen-inputs-v1', TARGET / 'trm-oracle-v1'),
]
for label, source, oracle in cohorts:
    copy_tree(source, 'inputs/' + label)
    copy_tree(oracle, 'oracle/' + label)
copy_tree(TARGET / 'trm-return-metadata-v1', 'oracle/return-metadata12')
copy_tree(TARGET / 'terminal-inherited-library-workflow-addendum-v1', 'oracle/inherited-workflow/producer-and-receipts')
copy_tree(TARGET / 'tilw-a4-record', 'oracle/inherited-workflow/record')
copy_tree(TARGET / 'tilw-a4-replay', 'oracle/inherited-workflow/replay')
save(OUT / 'oracle-provenance.json', {
    'state': 'byteexact_original_corpus_extracted_no_reclassification',
    'files': copies,
    'original30_semantics': 'All original stage failures, partials, timeouts and unexecuted probes retained.',
    'same_source_inherited_workflow': 'Distinct successful workflow, not a repair/reclassification of original failed workflow.',
    'normalization': 'None on copied bytes. Existing generated replay comparison permits only strict owned provider.genmodel output path.',
    'jars': 'External60 pinned prerequisites, not copied', 'production_changes': False, 'gate_closures': []})

# The original helper/rawmetadata stay byteexact. A separately named helper uses an
# explicit checked URIConverter binding; it never rewrites observed identities.
original = OUT / 'inputs/return12/TerminalReturnIdentityOracle.java'
text = disk(original).read_text(encoding='utf-8')
assert text.count('public final class TerminalReturnIdentityOracle') == 1
text = text.replace('public final class TerminalReturnIdentityOracle', 'public final class TerminalReturnIdentityOracleMapped')
needle = '    var imported = set.getResource(URI.createFileURI(root.resolve("imported.ecore").toString()), true);'
assert text.count(needle) == 1
replacement = '''    if (args.length != 2) throw new IllegalArgumentException("Expected physical case root and pinned provider binding TSV");
    var rows = Files.readAllLines(Path.of(args[1]), StandardCharsets.UTF_8);
    if (rows.size() != 1) throw new IllegalArgumentException("Expected exactly one provider binding");
    var binding = rows.get(0).split("\\t", -1);
    if (binding.length != 3 || !binding[0].startsWith("file:/") || !binding[1].matches("[0-9a-f]{64}") || !binding[2].equals("imported.ecore")) throw new IllegalArgumentException("Invalid provider binding");
    Path physical = root.resolve(binding[2]).toAbsolutePath().normalize();
    if (!physical.getParent().equals(root.normalize())) throw new IllegalArgumentException("Provider binding escaped case root");
    String actualProvider = HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(physical)));
    if (!actualProvider.equals(binding[1])) throw new IllegalArgumentException("Provider binding bytes differ");
    URI logicalProvider = URI.createURI(binding[0]);
    if (set.getURIConverter().getURIMap().put(logicalProvider, URI.createFileURI(physical.toString())) != null) throw new IllegalArgumentException("Duplicate provider URI binding");
    var imported = set.getResource(logicalProvider, true);'''
text = text.replace(needle, replacement)
helper = OUT / 'oracle-producer/metadata-uri-binding/TerminalReturnIdentityOracleMapped.java'
disk(helper).parent.mkdir(parents=True, exist_ok=True)
with disk(helper).open('x', encoding='utf-8', newline='\n') as f:
    f.write(text)

original_rows = read(OUT / 'oracle/return-metadata12/record/observations.json')
bindings = []
def provider_uris(value, found):
    if isinstance(value, dict):
        for key, item in value.items():
            if key == 'resource_uri' and isinstance(item, str) and item.startswith('file:/') and item.endswith('/imported.ecore'):
                found.add(item)
            provider_uris(item, found)
    elif isinstance(value, list):
        for item in value:
            provider_uris(item, found)
for row in original_rows:
    selected = set()
    provider_uris(row, selected)
    assert len(selected) == 1, (row['id'], selected)
    logical = next(iter(selected))
    assert logical.endswith('/' + row['id'] + '/imported.ecore'), logical
    relative = 'inputs/return12/inputs/' + row['id'] + '/imported.ecore'
    expected = pin(OUT / relative)
    mapping = OUT / 'oracle-producer/metadata-uri-binding/mappings' / (row['id'] + '.tsv')
    disk(mapping).parent.mkdir(parents=True, exist_ok=True)
    with disk(mapping).open('x', encoding='utf-8', newline='\n') as f:
        f.write(logical + '\t' + expected['sha256'] + '\timported.ecore\n')
    bindings.append({'id': row['id'], 'original_logical_provider_uri': logical, 'physical_provider': relative,
                     'physical_provider_pin': expected, 'mapping': mapping.relative_to(OUT).as_posix(), 'mapping_pin': pin(mapping)})
save(OUT / 'oracle-producer/metadata-uri-binding/frozen-addendum.json', {
    'state': 'frozen_before_execution', 'helper': {'path': helper.relative_to(OUT).as_posix(), **pin(helper)},
    'original_helper': {'path': original.relative_to(OUT).as_posix(), **pin(original)},
    'original_metadata': {'path': 'oracle/return-metadata12/record/observations.json', **pin(OUT / 'oracle/return-metadata12/record/observations.json')},
    'bindings': bindings, 'comparison': 'Exact complete original JSON, no normalization or additional exception',
    'source_and_grammar_bytes_changed': False, 'production_changes': False, 'gate_closures': []})
print(json.dumps({'corpus': str(OUT), 'raw_files': len(copies), 'provenance': pin(OUT / 'oracle-provenance.json'),
                  'binding_freeze': pin(OUT / 'oracle-producer/metadata-uri-binding/frozen-addendum.json')}))
