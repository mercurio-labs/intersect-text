package InitialValue {
    class C {
        // Fires: an initial value on a feature that is not variable.
        feature x := 1;
        // Does not fire: variable, constant (the KerML parser makes a
        // constant feature variable), or a bound value.
        var feature y := 1;
        const feature z := 1;
        feature w = 1;
    }
}
