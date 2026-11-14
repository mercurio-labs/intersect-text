package ParameterUsages {
    // Positive control for the parameter membership rules: no diagnostic
    // expected.
    attribute def T;
    action def A {
        in x : T;
        out y : T;
        inout z : T;
    }
    calc def C {
        in a : T;
        return r : T;
    }
    part def P {
        in p : T;
    }
}
