package InheritedSubjects {
    part def V;

    requirement def A { subject a : V; }
    requirement def B { subject b : V; }

    // Error on the definition: two inherited subjects (each redefines
    // RequirementCheck::subj, neither the other).
    requirement def C :> A, B;

    // No error: d redefines both inherited subjects by position.
    requirement def D :> A, B { subject d : V; }

    // Error on the usage: two subjects inherited through its definition.
    requirement c : C;

    // No error: an explicit redefinition, and one inherited subject.
    requirement def E :> A { subject e :>> a; }
    requirement r1 : A;

    // Error on the definition: its first parameter redefines the inherited
    // subject by position, so the first input is not a subject.
    requirement def F { in attribute x; }

    // Error on the owned subject: an actor comes first.
    requirement def G { actor ac : V; subject g : V; }

    // Error on its second owned subject (the first redefines A::a).
    requirement r2 : A {
        subject s1 : V;
        subject s2 : V;
    }

    // Cases: two inherited subjects, and an owned one after an input.
    case def K1 { subject k1 : V; }
    case def K2 { subject k2 : V; }
    case def K3 :> K1, K2;
    case k4 : K1 { in ref y : V; subject k4s : V; }

    // No error: one subject redefining both.
    case def K5 :> K1, K2 { subject k5 : V; }
}
