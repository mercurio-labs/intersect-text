package FeatureReference {
    // The two B1 cases of the Pilot's FeatureChain_invalid test, without the
    // chain `v2.m::mm` that Intersect does not resolve yet (which stops that
    // unit before validation).
    private import ScalarValues::*;
    struct V {
        feature n : Integer;
    }
    feature v1 : V;
    // Fires: the referent is a structure, not a feature.
    feature v2_V : Integer = V;
    // Fires: one chaining feature.
    feature v1_ chains v1;
    // Does not fire.
    feature v1_n : Integer = v1.n;
    feature v1_n2 chains v1.n;
}
