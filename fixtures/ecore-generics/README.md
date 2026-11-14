# Generic signature checks

signatures.ecore is an E07 definition-checking fixture. It includes an F-bound,
class and operation type parameters, an operation result/parameter, a raw generic
reference (warning), and a wildcard application. Unicode preceding the declarations
exercises byte-accurate diagnostic locations.

Use host ecore/generic-type or ecore/generic-parameter with a retained definition
bundle and exact declaration identity; worker methods are intersect/checkGenericType
and intersect/checkGenericParameter. A source created as memory:generics uses:

- Box.T: parameter path [1,0].
- Box.value type: [1,1,0].
- convert result: [1,2,1]; input type: [1,2,2,0].
- Holder.raw type: [2,0,0], with an explicit raw_type warning.
- Holder.boxed type: [2,1,0], with one wildcard argument.

The report is executable=false. These checks resolve signature identities, scope,
arity and positional validity; they do not qualify substitution/bound satisfaction,
generic runtime construction, or operation execution. E07 signature checking and concrete schema integration are complete.
The E08 generic conformance milestone remains open.

To check every signature, use host `ecore/generic-signatures` or worker
`intersect/checkGenericSignatures` with the retained bundle and no declaration
selection. The fixture yields three formal declarations and five top-level
generic types, with bounds/arguments retained in their owning trees. The report
includes a raw-type warning and remains definition-only (`executable: false`).

Resolved generic types also expose their exact `raw_type`. Complete inventories
retain `raw_signatures` for legacy/generic projections, including void results.
For example Box.value erases to Comparable through the parameter's F-bound;
the convert result erases to pinned EJavaObject because U is unbounded. These
raw identities do not substitute generic arguments or prove bound satisfaction.

`library.ecore` is executable: it expresses the existing Library model's feature
types and supertypes using concrete generic classifier forms. Compile it with
`fixtures/library/library.xtext` and `fixtures/library/library-behavior.rcore`, then use
the catalog/loans model texts from that directory. The schema retains checked
generic signatures. Type arguments and formal parameters remain an explicit E08
runtime dependency; `signatures.ecore` is still definition-checking input only.

For structural substitution, use host `ecore/generic-substitute` or worker
`intersect/substituteGenericType` with `bundle`, `declaration` and a `bindings`
array of `{parameter, argument}` exact declaration IDs. Arguments select retained
generic type uses, not classifier declarations or caller-supplied expressions.
For example select Box.value `[1,1,0]`, bind Box.T `[1,0]` to Holder.boxed's type
`[2,1,0]`, and obtain Box<?>. This substitution does not prove that Box<?> satisfies
T's Comparable<T> bound: the report explicitly has `bounds_verified: false` and
`executable: false`. Selecting the F-bound `[1,0,0]` as T's argument rejects an
expanding substitution cycle. Empty bindings return the normalized source type.

`inheritance.ecore` checks receiver-specific generic ancestry. Select the type use
`[6,0,0]` (Diamond<Value>) with host `ecore/generic-inheritance` or worker
`intersect/genericInheritance`. It reports Left<Value>, Base<Value>, Right<Value>
in declared depth-first order, retaining Base's first path through Left. Conflicting
Base arguments and cyclic inheritance reject. This remains definition-only:
`bounds_verified: false`, `executable: false`; raw ancestry and bound satisfaction
remain E08 work.

`bounds.ecore` checks Box<Good> where T is bounded by Comparable<T> and Good
inherits Comparable<Good>. Select `[4,0,0]` with host
`ecore/generic-application-bounds` or worker `intersect/checkGenericApplicationBounds`.
The closed nominal check succeeds; changing the actual to Bad, or changing Good's
parent to Comparable<Bad>, fails with retained per-bound evidence. Reports remain
`executable: false`; open/raw/provider conformance and model execution are still
unfinished E08 work.

`members.ecore` projects members of Child, which inherits Base<Value>. Select the
receiver type `[3,0,0]` with host `ecore/generic-member-types` or worker
`intersect/genericMemberTypes`. Base.value and convert's input become Value;
convert's same-named local T remains a distinct parameter whose bound becomes
Value. IDs, exception signatures and source metadata are retained. This does not
select overrides or execute operations (`selection_verified: false`,
`executable: false`).

A concrete type use is not required for explicit requests: host
`ecore/generic-instantiate-members` and worker `intersect/instantiateGenericMembers`
take a closed `receiver` expression with exact classifier IDs and actual arguments.
The resolver rechecks every identity, arity and nominal bound from retained source.
This supports requesting a template application absent from the Ecore text, while
still reporting `selection_verified: false` and `executable: false` until schema
materialization and runtime integration are implemented.


`materialize.ecore` exercises the new explicit closed-set compiler API. Request
ValueBox `[3]`, Value `[0]` and Other `[1]`; Box<Value> is added as an inheritance
ancestor. Value.name defaults to "new", ValueBox.value accepts Value and rejects
Other, and accept's input signature becomes Value. Host `ecore/materialize` and
worker `intersect/materializeGenericTypes` accept a retained bundle and closed
`types`, with optional instances and edits for isolated model construction.
This path does not activate the workbench package or execute operation bodies.
Generic methods, open/raw types and missing target applications remain
unsupported here. Concrete datatype providers can be supplied explicitly as
described below. See the E08 conformance document
for the complete limits and verification evidence.


`materialize-opposites.ecore` extends that model with reciprocal Box<T>.children
and Box<T>.parent references, including containment and child-href syntax. Request
both Box<Value> and Box<Other> to verify that links cannot cross actual arguments;
ValueBox inherits the Box<Value> links. Direct core edits maintain both ends and
roll back invalid assignments through the existing transaction engine.


`materialize-conversions.ecore` specializes Holder<T>.color to a custom RGB
datatype. Select Holder<RGB> and the `example.rgb` version `1` contract for the
exact RGB declaration `[0]`. Host/worker materialization accepts `conversions`;
native callers use `materialize_closed_types_with_conversions`. The default
`#123ABC` is converted and checked by the existing catalog provider, as are edits.
Missing providers and invalid payloads reject. This does not implement generic
datatype constructors or provider assignability.


For explicit generic operation arguments, use `members.ecore` with host
`ecore/generic-operation-types` or worker `intersect/instantiateGenericOperation`.
Select closed Child `[2]`, operation `[1,2]`, and one closed Value `[0]` argument.
The operation-local T becomes Value while preserving its distinct declaration
from Base.T. Result, input and exception signatures become concrete, with retained
bound evidence. This report is not executable and does not perform overload
inference or activate a package; generic method schema integration remains open.


`language-bundle.json` is an executable bundle with explicit `materialized_types`.
Pass it to normal `compile_bundle` or worker `intersect/compileBundle`. It combines
Root containment, ValueBox inheriting Box<Value>, Xtext cross-references and a
Rcore v1 count derivation. Two model resources can contain `root { value v; }`
and `root { box v; }`. Retained bundle replay rebuilds the generic schema before
model evaluation. Ambiguous same-name applications require unambiguous grammar
targets (for example concrete wrapper classes); they do not bind arbitrarily.

`open-bounds.ecore` exercises symbolic application bounds. Select the generic
usage at [4,1,0] with ecore/generic-application-bounds or
intersect/checkGenericApplicationBounds. Box<Use.U> satisfies Box.T's Base bound
because Use.U is bounded by Good, a Base subtype. Removing that bound or replacing
it with Other produces a failed bound check. The report retains Use.U's exact
parameter identity; it does not construct an open runtime type.

`raw-bounds.ecore` selects a raw receiver at [4,0,0]. Its symbolic Carrier<Raw.T>
ancestor satisfies Carrier.C's Base bound through Raw.T's Good bound. Removing
the Good bound fails conformance. Both outcomes retain a raw_type warning for
omitted receiver arguments; neither report constructs a raw runtime object.

`mixed-ancestry.ecore` uses Child with both raw Base and Base<Value> inheritance
paths. Check ancestry at [7,1,0] to see both applications, or bounds at [7,0,0]
to verify Limit<Child> against Base<Value>. Reversing Child's parent order still
satisfies that bound; changing the required Value to Other fails. These reports
preserve raw warnings and do not activate or construct raw runtime types.

`scalar-alias-bounds.ecore` checks Box<Holder<A>> at [4,0,0] against Holder<B>,
where A and B have separate identities but the same native String binding.
Changing B to Integer fails; an unknown binding requires a provider contract.
`materialize-scalar-bounds.ecore` checks Holder<EString> at [1,0,0] against
an Object bound. Its selected expression can be passed to ecore/materialize to
construct a Holder with the default value "hello" and perform checked edits.

`explicit-arguments.ecore` declares primitive Value [0], Box<T> [1] and Ops [2].
An explicit Box<Value> request or Ops.identity<Value> operation [2,1] rejects
the primitive argument, while Ops.number's ordinary typed use [2,0,0] is legal.
Changing Value's binding to java.lang.Integer or byte[] permits the generic
requests and Box materialization. Both supported instance metadata spellings
must produce the same source and explicit-request results.

- `provider-alias-bounds.ecore`: custom RGB aliases [0]/[1], bounded Holder [2],
  application [3,0,0]. Provider-aware conformance, member projection and materialization
  retain exact datatype identities and bound-only providers. Native package replay
  and browser materialization controls cover defaults, nominal rejection and missing
  providers; an appended Unused datatype [4] checks rejected extra selections.

- `operation-overrides.ecore`: Base<T>.f [2,1] and Derived.f<U> [3,1]
  exercise declared erasure and scalar alias equivalence; Unrelated.f [4,0]
  rejects the owner relation. Pairwise comparison is separate from receiver
  validity, effective operation selection and invocation.

- `materialize-operation-selection.ecore`: A [0], B extends A [1],
  Base<T extends A>.f(T) [2,1], and Derived extends Base<B>.f(B) [3,1].
  Declared erasure keeps f(A) and f(B) separate even though receiver substitution
  gives both a B parameter. Materialization preserves both declared groups in its
  selection contract. Changing the derived parameter to A preserves one override
  despite different projected types. Tests also cover matching projections,
  serialized replay, malformed contracts and sibling ambiguity.


- `method-templates.ecore`: Base<T>.convert<T extends Base.T> [1,1] is inherited
  by Child [2] through Base<Value>, with Value [0]. Result, mapped parameter [1,1,3] and
  exception reference the method formal; input [1,1,2] uses the class formal.
  Materialization retains the open template, flags and annotation. Explicit
  Value arguments close the signature; Other [3], stale IDs and wrong arity reject.

### Checked datatype applications

`datatype-applications.ecore` supplies Base/Child/Other, bounded Box and BoxAlias,
a Pair with a bound depending on its first formal, an unbounded Envelope, and
scalar/primitive/enum controls. `datatype_applications.rs` and the browser harness
use it with `Resolver::instantiate_datatype` / `ecore/generic-datatype-application`.
The report checks concrete application identities and bounds; this fixture does
not supply a collection codec or demonstrate model values of those datatypes.

### Applied collection values

`applied-collections.ecore` declares exact Text/Number domains and generic
nonserializable List<T>/Map<K,V> datatype bindings. The installed application
catalog uses their actual arguments to check member policies and nested applied
value tags. `intersect-language/tests/applied_conversion.rs` and the browser
applied-conversion control consume it. This demonstrates codecs and values;
direct generic attribute materialization and model persistence remain separate.


### Applied datatype model attributes

`applied-model.ecore` extends the collection fixture with Holder [4], generic
GenericHolder<T> [5] and TextHolder [6]. Holder uses List<Text>, List<Number>,
List<List<Text>> and Map<Text,Number>, a unique many-valued List<Text> attribute,
and two operation signatures with distinct applied parameter domains. Language,
SQLite and host tests named `applied_model` exercise source compilation, model
edits, rollback, provider reconstruction and persistence. The browser control
compares native/WASM materialization and package evaluation. Collection values
are assigned through model APIs; the grammar consumes the ordinary name field.


### Provider-dependent applied bounds

`applied-provider-bounds.ecore` declares RGB [0], Alias [1], BoundedList<T extends
RGB> [2], Envelope<T> [3], OrdinaryList [4], Holder [5] and OnlyWrapped [6]. The
installed RGB providers establish bound compatibility while values keep exact
nominal tags. The language and browser controls check staged provider selection,
nested applications and reconstruction of bound-only dependencies. OnlyWrapped
has no direct applied attribute: its ordinary provider contains the applied
member. Reference contexts must supply their own relation providers.


`applied-provider-operations.ecore` preserves the first seven declarations of
the bound fixture, adding FeatureMap [7], FeatureEntry [8] and Maps [9]. Their
selected feature is OnlyWrapped.wrapped [6,0]; its value is an ordinary collection
whose member is the provider-bounded applied list. Host and browser controls check
operation factory reconstruction, successful transitions and nominal rejection.

`metadata-revision-oracles.json` contains independently authored JSON tuple bytes
and SHA-256 values for retained runtime-metadata hashing. These are local
compiler identity fixtures, not upstream EMF observations or language qualification.
