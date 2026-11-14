package DerivedFeatureRules {
    // Batch B4 rules with non-firing controls.

    // validateClassifierDefaultSupertype_: a conjugation cycle, and a class
    // conjugating a classifier (not an occurrence type).
    classifier C ~ C;
    classifier G;
    class F ~ G;
    // Controls.
    class E;
    classifier H ~ E;
    class F2 ~ E;

    // validateFeatureIsVariable.
    datatype V {
        var feature z;
    }
    var feature z2;
    // Control.
    class K {
        var feature w;
    }

    // validateExpressionResultExpressionMembership.
    function Fn {
        1
    }
    function Gn :> Fn {
        2
    }
    expr fx : Fn {
        3
    }
    expr gx :> fx;
    // Controls: one result expression, owned or inherited.
    function Hn :> Fn;
    function In {
        4
    }

    // validateFeatureValueOverriding.
    class X {
        feature x1 = 1;
        feature x2 default = 2;
    }
    class Y specializes X {
        feature :>> x1 = 3;
        // Control: the redefined value is a default.
        feature :>> x2 = 4;
    }
    // Control: no value to override.
    class Y2 specializes X {
        feature :>> x1;
    }
}
