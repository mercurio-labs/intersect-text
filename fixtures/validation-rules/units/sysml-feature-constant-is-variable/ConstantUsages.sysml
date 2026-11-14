package ConstantUsages {
    attribute def D {
        // Fires: a constant attribute of an attribute definition is not
        // variable (it may not vary in time).
        constant attribute c;
    }
    part def P {
        // Does not fire: owned by an occurrence definition.
        constant attribute d;
    }
}
