"""Pure evidence checks; Java/native production execution is not required."""
from pathlib import Path, PureWindowsPath
import copy
import hashlib
import json

HERE = Path(__file__).resolve().parent
def read(name):
    return json.loads((HERE / name).read_text(encoding='utf-8'))
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def comparable(row, output):
    """Only the independently owned, exact provider output location may vary."""
    value = copy.deepcopy(row)
    provider = value['provider']
    path = PureWindowsPath(provider['genmodel'])
    root = PureWindowsPath(output)
    assert path.is_absolute() and root.is_absolute()
    assert '..' not in path.parts and '..' not in root.parts
    assert path == root / ('provider-' + provider['ecore_sha256'][:12]) / 'provider.genmodel'
    provider['genmodel'] = '<verified-owned-provider-output>'
    return value

def verify():
    manifest = read('provenance.json')
    for name, expected in manifest['files'].items():
        path = HERE / name
        assert path.stat().st_size == expected['bytes'], name
        assert sha(path) == expected['sha256'], name
    roots = read('recorded-output-roots.json')
    for folder, key, count in [('', 'main', 3), ('negatives/', 'negative', 6)]:
        pins = read(folder + 'inputs/pins.json')
        for name, digest in pins.items():
            assert sha(HERE / folder / 'inputs' / name) == digest, name
        cases = read(folder + 'inputs/cases.json')
        record = read(folder + 'oracle-record/observations.json')
        replay = read(folder + 'oracle-replay/observations.json')
        assert len(cases) == len(record) == len(replay) == count
        for c, first, second in zip(cases, record, replay):
            assert c['id'] == first['id'] == second['id']
            assert c['inputs'] == first['original_inputs'] == second['original_inputs']
            assert comparable(first, roots[key]['record']) == comparable(second, roots[key]['replay'])
            for source in c['sources']:
                assert sha(HERE / folder / 'inputs' / c['id'] / source['file']) == source['source_id']['sha256']
            assert first['provider']['generation_exit'] == first['provider']['compile_exit'] == 0
            if key == 'main':
                assert first['generation']['exit_code'] == first['compilation']['exit_code'] == 0
                assert len(first['runtime']) == 6
                assert not any(r['errors'] for r in first['metadata']['observation']['syntax'])
                assert not any(r['issues'] for r in first['metadata']['observation']['validation'])
            else:
                assert first['generation']['exit_code'] != 0
                assert first['runtime'] == [] and first['compilation'] is None
                assert any(r['issues'] for r in first['metadata']['observation']['validation'])
    validation = read('validation-addendum-v2/record.json')
    assert validation == read('validation-addendum-v2/replay.json')
    assert len(validation) == 3 and sum(len(c['runtime']) for c in validation) == 18
    for index, c in enumerate(validation):
        for j, run in enumerate(c['runtime']):
            item = run['observation']
            assert run['exit_code'] == 0
            assert item['feature']['unique'] == (index != 2)
            assert item['direct_list_probe']['second_same_target_add_changed'] is False
            if j < 4:
                assert item['resource']['errors'] == []
                assert item['xtext_validation']['issues'] == []
                assert item['emf_validation']['severity'] == 0
    current = read('native/public-WASM-record.json')
    assert current['transcript'] == read('native/public-WASM-replay.json')['transcript']
    assert len(current['transcript']) == 116
    old = sum((read(name)['transcript'] for name in
               ['native-before-v64.json', 'native-before-v65.json', 'native-negative-before-v64.json']), [])
    assert len(old) == 60 and current['transcript'][:60] == old
    native = read('native/complete-model-observations.json')
    assert len(native) == 18
    invalid_duplicate_count = 0
    for index, c in enumerate(current['rows']):
        assert c['package']['compilation'] == c['compilation']
        assert c['package']['schema']['classes']['Model']['features']['refs']['unique'] == (index != 2)
        for source in c['compilation']['bundle']['sources']:
            assert hashlib.sha256(source['text'].encode('utf-8')).hexdigest() == source['id']['sha256']
        for j, result in enumerate(c['models']):
            expected = next(v for v in native if v['id'] == c['id'] and v['text'] == result['text'])
            assert expected['result'] == result['result']
            if index != 2 and j in [1, 2]:
                assert result['result']['diagnostics'] == [{
                    'source': 'memory:' + c['id'] + '/model.unique', 'start': 0, 'end': 1,
                    'message': 'Duplicate value: Model.refs'}]
                invalid_duplicate_count += 1
            elif j < 4:
                assert result['result']['diagnostics'] == []
    assert invalid_duplicate_count == 4  # Explicit disagreement with resolved upstream resources.
    sysml = read('full-source-probes/sysml-proper-execution-v66-v2/terminal.json')
    assert len(sysml['runs']) == 6 and all(v['exit_code'] == 0 for v in sysml['runs'])
    for row in sysml['runs']:
        source = row['observation']['sources'][-1]
        assert source['uri'] == 'org.omg.sysml.xtext/src/org/omg/sysml/xtext/SysML.xtext'
        assert source['sha256'] == '7d9142742c454f5810fe4db61a089abef5e0b8d61ad8bf552631d9136b04f298'
    for mode in ['dev', 'production']:
        browser = read('native/browser-' + mode + '-terminal.json')
        assert browser['exit_code'] == 0
        assert browser['stats']['expected'] == 2
        assert browser['stats']['unexpected'] == browser['stats']['skipped'] == browser['stats']['flaky'] == 0
    return {'oracle_graphs': 9, 'main_runtime_observations': 18, 'negative_unexecuted': 6,
            'native_models': 18, 'native_duplicate_diagnostic_disagreements': 4,
            'complete_ABI_calls_per_run': 116, 'browser_tests_per_mode': 2}

if __name__ == '__main__':
    print(json.dumps(verify(), sort_keys=True))
