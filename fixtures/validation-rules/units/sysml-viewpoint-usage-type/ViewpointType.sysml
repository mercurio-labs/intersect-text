package ViewpointType {
    viewpoint def VP1;
    viewpoint def VP2;
    viewpoint def VP3 :> VP1;
    requirement def R;
    part def P;

    // Error: two viewpoint definitions (also not one requirement
    // definition), a requirement definition that is not a viewpoint
    // definition, and a part definition (neither).
    viewpoint vp1 : VP1, VP2;
    viewpoint vp2 : R;
    viewpoint vp3 : P;

    // No error: one viewpoint definition, the library default, a redundant
    // general, and a subsetted viewpoint.
    viewpoint vp4 : VP1;
    viewpoint vp5;
    viewpoint vp6 : VP1, VP3;
    viewpoint vp7 :> vp4;
}
