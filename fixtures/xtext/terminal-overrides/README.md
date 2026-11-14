# Independent contextual terminal binding

These are Intersect-authored verification inputs to pinned Xtext 2.38.0's
`XtextLinker.updateOverriddenRules`, using the existing checked oracle classpath.
The Java adapter constructs linked ASTs; it does not implement the rebinding
algorithm. `observations.json` records terminal-call, parser-call and hidden-token
identities and the selected literal for five scenarios. Production does not use
Java. Upstream JARs retain their upstream licenses; none are copied here.

Replay with `python oracle.py --jdk JDK --artifact-dir OUTPUT`. The default checks
all input/output hashes and compares fresh upstream observations. `--record` is
only for intentional generation of a new evidence snapshot.

Native/WASM controls compare the executable ordinary and terminal-nested cases
against the observed selected identity and literal. The upstream binding adapter
can operate on invalid linked ASTs: direct parser calls to terminal fragments are
not valid executable evidence. Native controls require rejection of those cases,
even when a child later promotes the invalid parent's fragment. The explicit
qualified-parent call records the upstream retained base identity; NativeV15
rejects that unsupported spelling instead of silently selecting the child.
Hidden-token rebinding is recorded upstream, not fully runtime-qualified here.

This is independent binding evidence, not complete grammar validation, generated
lexer equivalence, broad X02 closure or KerML domain qualification. Original
terminal-overrides execution controls and all earlier oracle snapshots remain.

The additive `scope_oracle.py` replays nine spellings through pinned
`SuperCallScope` and the Xtext parser. `super`, `super::LETTER`, `Base::LETTER`
and `audit::Base::LETTER` select the parent declaration from the child context;
short/full child qualification selects the child. Missing grammar/rule names
have no target. The dotted spelling is not a qualified RuleID: inside a terminal
body it is a Group of calls and wildcard operators. Native AST shape controls
compare that distinction with the upstream parser. Earlier dotted execution
rejection alone did not prove qualified-call rejection; the new valid-syntax
controls supply that missing evidence without removing the old test.

NativeV15 still rejects executable explicit calls. Implementation needs retained
parent rule programs selected by exact source identity, separate from ordinary
child-selected names. Ordinary nested calls inside a retained parent must still
use contextual child dispatch unless themselves explicit. Visible lexical rules
must remain distinct from terminal programs used only by other terminals, and
synthetic program aliases must not double-count source declaration budgets.

The additive `inheritance_oracle.py` checks parent cardinality with the pinned
parser and `XtextValidator.checkGrammarUsesMaxOneOther`. The adapter supplies
linked parent objects explicitly after syntax parsing (IParser alone does not
link cross-references). Multiple parents are retained syntax but semantically
invalid in Xtext 2.38: only zero or one is accepted by this check. The native
consumer verifies rejection with every referenced source supplied. This is one
specific validation rule, not a full grammar-validation oracle.


FinalOracle / final_oracle.py independently invoke pinned
XtextValidator.checkOverridingRule on twelve authored linked-rule cases. Parser,
terminal and enum rules each cover ordinary replacement, final replacement,
final replacement without @Override, and final replacement through an intermediate
ancestor. This checks the final prohibition only, not full grammar validity or
parser execution. final-provenance.json pins every adapter and observed result;
replay uses the same --jdk / --artifact-dir arguments as the other adapters.
Java remains verification-only.


MetamodelOracle / metamodel_oracle.py invoke pinned
Xtext2EcoreTransformer.findMetamodel and XtextValidator.checkMetamodelUris on
linked authored EPackages. Seven cases distinguish shared aliases with unique
classifier names, ambiguous classifier matches, separate aliases, missing aliases,
missing classifiers, omitted aliases and duplicate namespace imports. These are
lookup/validation observations, not full resource linking or generated execution.
The adapter and outputs are pinned in metamodel-provenance.json; run with --jdk
and --artifact-dir to replay without changing the recorded evidence.


CategoryOracle / category_oracle.py invoke the pinned private
Xtext2EcoreTransformer.checkSupertypeOfOverriddenDatatypeRule using reflection.
Thirteen linked-rule controls isolate datatype identity and override category
checks. A shared EEnum is used as the EDataType identity across the category
matrix; this does not claim that every authored rule body would pass full linking,
conversion or generation. Additional cases reject terminal fragments replacing
parser/enum rules, changed datatype identity and missing parent return type.
This Java adapter is verification-only; original observations remain unchanged.

HiddenPolicyOracle / hidden_policy_oracle.py invoke pinned FlattenedGrammarAccess
with RuleNames and RuleFilter on six authored linked grammars. Absent root policy
and inherited absence normalize to an empty list; inherited WS is retained, and
an explicit empty child clears it. This is independent grammar-flattening evidence,
not complete validation or generated parser execution. The adapter, observations
and binary JAR digests are pinned in hidden-policy-provenance.json. Replay uses
the same --jdk and --artifact-dir arguments. Original oracle files are unchanged.
