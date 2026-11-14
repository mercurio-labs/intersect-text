# Independent guarded-sequence evidence

This preimplementation fixture records a confirmed mismatch with pinned Xtext
2.38.0. A false guarded group inside a mandatory sequence is removed upstream;
Intersect currently executes a failure primitive instead. For example,
`{Model} 'pre' (<false> child=Node<false>) 'post'` accepts `pre post` upstream
with a Model whose child is null. The baseline native/WASM parser rejects it with
`Boolean guard is false`. This fixture awards no native support and closes no gate.
Full parameterization, X03/X07/X10, S09 and full Rcore/Xtext support remain OPEN.

Fifteen ordinary imported graphs preserve thirteen metadata acceptances and two
rejections. Twelve parsers compile and execute 103 registered resource requests:
fourteen rooted models and 89 errors. One metadata-accepted whole-rule false guard
fails upstream parser generation with a null AbstractElement and unhandled null
parameter diagnostic; its eight requests were not executed. Wrong call arity in a
false group and two conflicting construction actions fail metadata validation;
their seventeen requests were not executed. The exploratory plan expected the
second graph to pass metadata validation; that original assumption is retained
alongside the independently observed rejection, never substituted for it. All 25 unexecuted requests are retained
separately, never counted as runtime syntax rejections. Twelve probes without the
required model provider remain separate observations.

Four inferred graphs compile and execute 32 resource requests: six rooted models
and 26 errors. The cases distinguish mandatory/optional/star/plus false groups,
extra parentheses, active alternatives, all-false alternatives, false/true bound
parameters and fragments. Extra parentheses around a false choice branch do not
make it an epsilon alternative. Full partial upstream models, diagnostics,
source ranges, imported metadata and generated parser inventories are retained.

Ordinary graphs use byte-identical Ecore from the frozen false_call provider;
provider regeneration verifies that identity. Ordinary grammars generate no Ecore.
Inference retains generated Ecore artifact hashes. Original parameter and entry
fixtures remain unchanged. `native-before.json` preserves fourteen observations
from the preimplementation release WASM; it documents the defect, not desired
behavior. Java is optional independent verification only and is never production
execution authority.

Offline integrity and completeness verification needs no Java or network:

~~~sh
python -B fixtures/xtext/guard-sequences/verify.py
~~~

Fresh read-only replay needs an explicit JDK and populated pinned Maven cache:

~~~sh
python -B fixtures/xtext/guard-sequences/verify.py --replay target/guard-readonly --java-home /path/to/jdk --maven-cache /path/to/maven/repository
~~~

Replay creates a new directory outside fixtures, reproduces the metadata and
generator failures, and compares every canonical observation. There is no
expectation-update option. Shared oracle adapters and dependencies are pinned by
hash. Assembly normalizes only duplicate/order of the two stable generator
failure diagnostics; complete raw logs remain in the replay directory.

The next implementation must distinguish structural omission from an empty
alternative and a rule without a retained body, while preserving validation of
dead guarded calls. It must use the shared specialized grammar/compiler and
runtime for ordinary and inferred definitions, then verify source replay, archive,
JSON/WASM and worker adoption. Neither filtering imported source information nor
adding a separate runtime parameter interpreter satisfies that contract.
