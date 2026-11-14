# Imported-only parameter evidence

Pinned Xtext 2.38.0 independent observations; native support remains unqualified.
The 48 exact graphs contain 53 grammar and 53 imported Ecore sources. Metadata
accepts 31 graphs and rejects 17 without changing imported structure. All 31
accepted graphs have generated ordinary parsers. The 91 registered requests
contain 40 rooted successes, 50 rejections and one error-free empty resource.
There are 31 additional unregistered-context probes.

The sources are new imported-only variants of the earlier parameter matrix and
supplemental empty-resource controls. Grammar identities and imported return/action
types are explicit. Original inference graphs, generator obstructions, unexecuted
requests and JVM partial models remain separate and unchanged. Negative graphs use
explicit schema inputs authored from independent partial metamodel projections;
they do not claim successful generation of their original invalid grammars.

Additional cases preserve Unicode producer ranges, aliased/unaliased implicit
returns, observable override behavior and named-entry availability. A declaration
with parameters is not enough to determine whether a no-argument named entry exists:
reachable false contexts can expose an entry while true-only contexts do not.
The first parameterized model entry has its default false context. The captured
unreachable inherited, typed/wildcard fragment and datatype entries remain explicit.

Source hashes cover exact UTF-8 bytes. JVM ranges use UTF-16 code units; the
verifier checks scalar boundaries and their conversion to UTF-8 byte positions.
Declaration references preserve grammar, rule and ordinal, including duplicate names.
Imported before/after metadata and complete rooted/partial/error/empty outcomes are
retained. Rejected JVM partial models are different from native atomic rejection.

Java is an independent verification oracle only. Replay first rebuilds each accepted
case's pinned model-provider scaffold, checks its exact Ecore hashes against the
imported inputs, and then generates the ordinary imported-only parser. The ordinary
grammars generate no new Ecore. Providers are explicitly registered for the main
requests; separate fresh processes retain unregistered-context results.

Run portable integrity checks without Java or network access:

```sh
python -B fixtures/xtext/imported-parameters/verify.py
```

Run a fresh read-only replay with the pinned Maven cache and an explicit JDK:

```sh
python -B fixtures/xtext/imported-parameters/verify.py --replay target/imported-parameter-replay --jdk /path/to/jdk --maven-cache /path/to/.m2/repository
```

Output must be a new directory outside fixtures. Replay compares all metadata,
entry inventories, provider identities and complete runtime observations. There is
no expectation-update mode. Jars, source archive and shared adapters are pinned in
`provenance.json`. Native/WASM/worker/archive/lifecycle adoption and the full original
parameterization/X03/X07/X10/S09 gates remain separate and open.
