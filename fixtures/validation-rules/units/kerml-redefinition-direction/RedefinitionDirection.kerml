package RedefinitionDirection {
    // validateRedefinitionDirectionConformance (batch B4), explicit
    // redefinitions (on the redefinition) and computed ones (on the
    // feature), with non-firing controls.
    class A {
        in feature x;
        out feature y;
        inout feature z;
        feature w;
    }
    class B specializes A {
        // Errors: out for in, in for out, none for inout.
        out feature x1 redefines x;
        in feature y1 redefines y;
        feature z1 redefines z;
        // Controls: an undirected feature may be redefined with a direction.
        in feature w1 redefines w;
    }
    class B2 specializes A {
        // Controls: the same direction; inout redefined with inout.
        in feature x2 redefines x;
        inout feature z2 redefines z;
    }
    // Through a conjugation the redefined direction is flipped.
    class A1 conjugates A;
    class D1 specializes A1 {
        // Errors: x is out and y is in relative to D1.
        in :>> x;
        out :>> y;
        // Control: inout stays inout.
        inout :>> z;
    }
    // Computed redefinitions of parameters by position.
    behavior P {
        in p;
        out q;
    }
    behavior P2 specializes P {
        // Errors on both parameters.
        out p2;
        in q2;
    }
    behavior P3 specializes P {
        // Controls.
        in p3;
        out q3;
    }
}
