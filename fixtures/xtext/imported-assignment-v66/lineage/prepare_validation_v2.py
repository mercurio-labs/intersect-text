from pathlib import Path
import hashlib,json
H=Path(__file__).resolve().parent
s=(H/'UniqueRuntimeAddendum.java').read_text(encoding='utf-8').replace('public final class UniqueRuntimeAddendum {','public final class UniqueRuntimeAddendumV2 {').replace('RuntimeOracle.identity(', 'MetamodelOracle.identity(')
(H/'UniqueRuntimeAddendumV2.java').write_text(s,encoding='utf-8',newline='\n')
s=(H/'run_validation_addendum.py').read_text(encoding='utf-8').replace("D=H/'validation-addendum-v1'","D=H/'validation-addendum-v2'").replace('UniqueRuntimeAddendum.java','UniqueRuntimeAddendumV2.java').replace("'intersect.oracle.UniqueRuntimeAddendum'","'intersect.oracle.UniqueRuntimeAddendumV2'")
(H/'run_validation_addendum_v2.py').write_text(s,encoding='utf-8',newline='\n')
def pin(p):b=p.read_bytes();return {'path':str(p),'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
(H/'validation-addendum-correction-v2.json').write_text(json.dumps({'scope':'Verification helper compile correction only: call existing MetamodelOracle.identity, not its static importer RuntimeOracle. No graph, generated provider, runtime text, source/profile/limit or production mutation.','v1_compile_failure':pin(H/'validation-addendum-v1/000.stderr.log'),'v1_source':pin(H/'UniqueRuntimeAddendum.java'),'v2_source':pin(H/'UniqueRuntimeAddendumV2.java'),'v2_runner':pin(H/'run_validation_addendum_v2.py'),'shell_preparation_attempt':'Inline python quoting attempt failed SyntaxError before writing either derivative; following missing-file run also preserved in tool output.'},indent=2)+'\n',encoding='utf-8',newline='\n')
