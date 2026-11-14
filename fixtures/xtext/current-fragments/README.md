# Fragments after current-object actions

This authored NativeV29 example uses the unchanged current-actions metamodel and
semantic definitions. It calls nested fragments after an assigned-current capture
and after an explicit simple action. Fragments retain the caller object and use
its effective feature contracts; they do not create replacement objects.

The native tests and real-WASM probe share positive, rejection, model-validation,
source/package replay, object-budget and recovery controls. This example is not
the pinned KerML grammar. Calling a fragment without an existing current object,
fragment actions, assigned fragment values and incompatible caller/member types
remain unsupported. Earlier profile admission remains unchanged.

The third control uses deliberately invalid caller feature metadata to compare
fragment and inline rejection. It is not Ecore feature-redefinition qualification.
