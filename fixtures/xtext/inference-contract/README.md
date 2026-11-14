# Independent return-type inference observations

This optional Java oracle calls the actual Xtext 2.38.0 transformer on nine
programmatically constructed grammar objects. It uses reflection to observe its
private return-type computation; no upstream classes or signed JARs are modified.
It does not parse a grammar, generate a language, or execute a model. No native
Intersect inference or recursive/nullable execution support is awarded here.

The omitted literal rule named Operator resolves to EString even with an imported
EClass Operator. Explicit model returns, assignment and action bodies retain the
model type. Forward scalar calls infer EString. An unresolved call is not inferred
as a scalar (this does not claim the unresolved grammar is valid). Recursive and
optional bodies can classify as EString; admission/execution remains a separate
obligation. These observations require a new opt-in native profile to preserve
existing accepted same-name model rules in older profiles.

`provenance.json` pins the independent harness, observations, six runtime JARs,
the upstream source JAR and the two inspected source members. Xtext archive hashes
must also match the repository's frozen upstream lock. The auxiliary EMF versions
are explicitly recorded and do not replace that lock or qualify EMF compatibility.
The harness is independently authored repository code; inspected upstream source
remains in its original EPL-2.0 archive and is not copied into this fixture.

Run with an installed JDK and populated local Maven cache (no downloads):

```powershell
python -B fixtures/xtext/inference-contract/verify.py --jdk D:/dev/jdks/jdk21 --artifact-dir target/inference-oracle
```

`--maven-cache` may select another cache. This is verification only: no build,
generation, browser or production path depends on Java. Generated classes stay in
the chosen artifact directory. A successful rerun compares all nine exact records.
