# Checked feature-map attribute values

Compile features.ecore, features.xtext and features-behavior.rcore with conversions.json,
then parse sample.features. Standard and declared entry/map attributes default to
null. Contracts select Holder.text/count/mode/color by exact source and ordinal,
with aligned builtin, nominal scalar, enum and converted RGB policies. Changing
source text requires updating every selected source hash.

Installed ecore.feature-entry/1 and ecore.feature-map/1 bind the corresponding
FeatureMap$Entry and FeatureMap classes. Entry payloads are {feature,value}; map
payloads contain an ordered entries array. Converted envelopes preserve datatype
and provider identity. Repeated features and pairs retain order. These detached
values do not apply owner multiplicities, containment/opposites or ExtendedMetaData
views; that model lifecycle remains E13. Both datatypes are nonserializable, so
lexical conversion rejects while checked internal snapshots persist.

JSON host feature-map/step and worker intersect/stepFeatureMap provide entry/empty
construction, insert/set/remove/move/clear and values-by-feature projection through
one shared native implementation. Operations are pure; normal model/Python
transactions publish results. Rcore v2 checks nominal reads but does not execute.

Reference-valued entries require an explicit Reference member policy retaining a
checked immutable schema/model context. Native contexts validate real objects
and subtype conformance. The companion ../feature-references fixture exercises
host/browser readback, persistence and export/reload for reference-valued entries.
This fixture covers the attribute-valued portion of the E06 feature-map contract. No Java
object or self-asserted runtime class is trusted implicitly.
