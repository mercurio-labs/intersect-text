package AssertSatisfyReference {
    constraint def C;
    requirement def R;
    part def P {
        part x;
        constraint c : C;
        requirement r : R;

        // Fires validateAssertConstraintUsageReference: a part is not a
        // constraint.
        assert x;
        // Fires both: a satisfy requirement usage is an assert constraint
        // usage, and a part is neither a constraint nor a requirement.
        satisfy x by self;
        // Fires validateSatisfyRequirementUsageReference only: a constraint
        // is not a requirement.
        satisfy c by self;

        // Do not fire.
        assert c;
        satisfy r by self;
    }
}
