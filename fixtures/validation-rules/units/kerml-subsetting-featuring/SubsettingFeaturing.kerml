package SubsettingFeaturing {
    // validateSubsettingFeaturingTypes and
    // validateFeatureChainingFeatureConformance (batch B4), with non-firing
    // controls.
    class A {
        feature a;
    }
    class B {
        feature b;
    }
    // Error: a is featured by A, which B does not specialize.
    class C {
        feature c subsets A::a;
    }
    // Controls: featured by a specialization of A, or nested in a feature
    // typed by A.
    class D specializes A {
        feature d subsets a;
    }
    feature p : A {
        feature q subsets a;
    }
    // Control: a package-level feature subsetting a chain.
    feature r subsets p.a;
    // Control for validateFeatureChainingFeatureConformance (the Pilot does
    // not resolve a chaining feature that is not featured within the
    // previous one, so the notation cannot reach the error).
    feature s : B;
    feature t subsets s.b;
}
