# Exact RealValue / EDouble checkpoint

The grammar and metamodel are an independently authored teaching model. The
RealValue, DECIMAL_VALUE, EXP_VALUE and WS declarations are exact byte slices of
the pinned KerMLExpressions grammar; source-provenance.json records the upstream
revision, raw source hash and each byte span/hash. The original copyright notice
and EPL-2.0 license accompany those slices. Existing fixtures are unchanged.

DoubleOracle.java is Intersect-authored. It calls pinned Xtext 2.38.0
EFactoryValueConverter with EMF 2.38.0 EDouble, using EMF common 2.42.0 and the
recorded JDK. The 25 observations cover rounding ties, normal/subnormal boundaries,
overflow to infinity, underflow, signed zero, special and hexadecimal spellings,
and invalid internal whitespace. They establish scalar conversion, not grammar
acceptance. In particular RealValue does not admit standalone integer, signed,
NaN, Infinity or hexadecimal spellings even though the EDouble converter does.

NativeV10 consumes explicit EDouble datatype parser rules through the shared
Ecore IEEE codec. Older profiles continue to reject them. The fourteen initial
oracle inputs are separately executed through the exact RealValue grammar;
rejections, resource rollback, exact/one-short budgets, source identities and
package replay are native/WASM controls. Values use the existing float64 bit
representation so infinity and subnormal values survive JSON/native/WASM transport.

The browser example has five objects in its first resource and two in its second.
Try `values . 5` to observe conversion rejection; restoring the original input
recovers the model. This demonstrates a pinned grammar fragment on an authored
metamodel; it does not qualify the complete KerML grammar or its domain semantics.

Run `python fixtures/xtext/double-datatypes/verify.py` for fixture integrity. Add
`--upstream <Pilot root>` to compare the exact raw upstream slices. For independent
Java replay add `--jdk <jdk> --artifact-dir <isolated output>` and, if needed,
`--ecore-jar <frozen 2.38.0 jar>`; the other jars use the Maven cache. Hashes must
match provenance and frozen upstream pins. The verifier never downloads jars.
Java remains optional verification tooling, never a native/WASM build or runtime
requirement. One missing verification-only EMF archive was fetched into the host
artifact directory and checked against the existing frozen SHA-256.
