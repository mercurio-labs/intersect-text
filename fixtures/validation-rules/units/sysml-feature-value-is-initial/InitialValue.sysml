package InitialValueUsages {
    attribute def D {
        // Fires: an attribute of an attribute definition may not vary in
        // time, so it is not variable.
        attribute a := 1;
    }
    part def P {
        // Does not fire: a usage owned by an occurrence definition is
        // variable.
        attribute b := 1;
    }
}
