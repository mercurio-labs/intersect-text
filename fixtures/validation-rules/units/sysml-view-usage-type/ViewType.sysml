package ViewType {
    view def VD1;
    view def VD2;
    view def VD3 :> VD1;
    rendering def R;
    part def P;

    // Error: two view definitions, a rendering definition and a part
    // definition.
    view v1 : VD1, VD2;
    view v2 : R;
    view v3 : P;

    // No error: one view definition, the library default, a redundant
    // general, and a subsetted view.
    view v4 : VD1;
    view v5;
    view v6 : VD1, VD3;
    view v7 :> v4;
}
