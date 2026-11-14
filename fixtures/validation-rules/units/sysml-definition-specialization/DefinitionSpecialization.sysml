package DefinitionSpecialization {
    item def I;
    attribute def A;
    // Fires: an attribute definition specializing an item definition
    // (SysML message for validateDataTypeSpecialization).
    attribute def A1 :> I;
    // Fires: an item definition specializing an attribute definition
    // (SysML message for validateClassSpecialization).
    item def I1 :> A;
    // Does not fire.
    attribute def A2 :> A;
    item def I2 :> I;
    part def P :> I;
}
