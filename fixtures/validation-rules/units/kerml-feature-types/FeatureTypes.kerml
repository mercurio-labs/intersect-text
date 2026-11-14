package FeatureTypes {
    // The KerML rules over all types of a feature (batch B3), with
    // non-firing controls.
    class C1;
    class C2;
    class C3 specializes C1;

    // validateFeatureHasType_: a feature conjugated to itself has no type.
    feature c ~ c;
    // Control: the original feature has a type (Base::Anything).
    feature d;
    feature e ~ d;

    // validateAssociationEndTypes on y (two types) and on A2's x, which
    // redefines A1's x by position (C2 and C1); A3's x has C3 and C1, and
    // C1 is redundant (control).
    assoc A0 { end x : C1; end y : C1, C2; }
    assoc A1 { end x : C1; end y : C2; }
    assoc A2 specializes A1 { end x : C2; end y : C2; }
    assoc A3 specializes A1 { end x : C3; end y : C2; }

    metaclass M;
    abstract metaclass AM;

    classifier K {
        // validateMetadataFeatureMetadataNotAbstract.
        @AM;
        // Controls (also for validateMetadataFeatureMetadata: the notation
        // types a metadata feature by one metaclass).
        @M;
        metadata m : M;
    }
}
