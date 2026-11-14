from pathlib import Path
H=Path(__file__).resolve().parent
s=(H/'finalize.py').read_text(encoding='utf-8').replace("None if not v.get('model') else", "None if not v.get('model',{}).get('elements') else")
s=s.replace("'UTF-8 inspection of upstream source entry failed; raw source bytes retained separately.',", "'UTF-8 inspection of upstream source entry failed; raw source bytes retained separately.','Finalizer v1 assumed every native error model had an element; malformed syntax produced an empty model. v2 handles absent/empty model in report only, without changing any captured result.',")
(H/'finalize_v2.py').write_text(s,encoding='utf-8',newline='\n')
