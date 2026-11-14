package CrossFeatures {
    // Cross feature rules (batch B4) with non-firing controls.
    class C1 {
        feature a : C2;
    }
    class C2 {
        feature b : C1;
        feature c subsets b;
    }
    // Controls: each end crosses through the other end, with its type.
    assoc A1 {
        end x : C1 crosses y.b;
        end y : C2 crosses x.a;
    }
    assoc A2 specializes A1 {
        // Control: c specializes b.
        end x : C1 crosses y.c;
        // validateFeatureCrossFeatureSpecialization (b does not specialize
        // A1's y's cross feature a), validateFeatureCrossFeatureType and
        // validateCrossSubsettingCrossedFeature.
        end y : C2 crosses y.b;
    }
    // validateCrossSubsettingCrossingFeature (with
    // validateFeatureCrossFeatureType): the crossing feature is not an end,
    // or an end of a type with a single end (validateAssociationRelatedTypes
    // on One).
    class K {
        feature k : C1 crosses k.a;
    }
    assoc One {
        end o : C1 crosses o.a;
    }
}
