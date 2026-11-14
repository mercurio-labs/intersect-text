package RenderingType {
    rendering def R1;
    rendering def R2;
    rendering def R3 :> R1;
    view def VD;
    part def P;

    // Error: two rendering definitions, a view definition and a part
    // definition.
    rendering r1 : R1, R2;
    rendering r2 : VD;
    rendering r3 : P;

    // No error: one rendering definition, the library default, a redundant
    // general, and a subsetted rendering.
    rendering r4 : R1;
    rendering r5;
    rendering r6 : R1, R3;
    rendering r7 :> r4;
}
