package VerificationCaseType {
    verification def V1;
    verification def V2;
    verification def V3 :> V1;
    case def C;
    part def P;

    // Error: two verification case definitions, a case definition that is
    // not one, and a part definition.
    verification v1 : V1, V2;
    verification v2 : C;
    verification v3 : P;

    // No error: one verification case definition, the library default, and
    // a definition with its own specialization (V1 is redundant).
    verification v4 : V1;
    verification v5;
    verification v6 : V1, V3;
    verification v7 :> v4;
}
