package Parameters {
    // Positive control for the parameter and result membership rules
    // (validateParameterMembershipOwningType, validateParameterMembershipDirection,
    // validateReturnParameterMembershipOwningType,
    // validateResultExpressionMembershipOwningType,
    // validateExpressionResultParameterMembership,
    // validateFunctionResultParameterMembership): no diagnostic expected.
    function F {
        in x;
        out y;
        inout z;
        return r;
    }
    feature e = F(1);
    expr g { in a; a }
    class C {
        in p;
    }
}
