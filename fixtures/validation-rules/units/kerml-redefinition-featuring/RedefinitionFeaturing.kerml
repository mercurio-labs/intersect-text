package RedefinitionFeaturing {
    // validateRedefinitionFeaturingTypes (batch B4), with non-firing
    // controls.
    feature f;
    // Error: a package-level feature cannot be redefined.
    feature g redefines f;
    class A {
        feature a;
        // Error: same featuring type (A).
        feature b redefines A::a;
    }
    class B specializes A {
        // Control: featured by B.
        feature c redefines a;
    }
    feature h : A {
        // Control: featured by h.
        feature d redefines a;
    }
}
