package Flows {
    // Positive control for the flow and flow end rules (validateFlowItemFeature,
    // validateFlowEndIsEnd, validateFlowEndNestedFeature,
    // validateFlowEndOwningType): no diagnostic expected.
    class V;
    struct S {
        out feature o : V;
        in feature i : V;
    }
    struct System {
        composite feature s1 : S;
        composite feature s2 : S;
        flow f of payload : V from s1.o to s2.i;
        succession flow g from s1.o to s2.i;
    }
}
