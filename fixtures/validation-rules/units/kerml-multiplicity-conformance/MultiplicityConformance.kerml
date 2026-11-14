package MultiplicityConformance {
    // validateRedefinitionMultiplicityConformance and
    // validateSubsettingMultiplicityConformance (batch B4, warnings), with
    // non-firing controls.
    class A {
        feature x[0..2];
        feature y[2..5];
        feature z[1];
        feature u[*];
        feature v;
    }
    class B specializes A {
        // Warnings: larger upper bound.
        feature x1[0..3] redefines x;
        feature z1[*] subsets z;
        // Warning: smaller lower bound.
        feature y1[1..5] redefines y;
        // Warnings: * against 2..5 (both bounds).
        feature y3[*] redefines y;
        // Warning: * against 0..2 (upper only; 0 is not positive).
        feature x2[*] redefines x;
        // Controls.
        feature u1[5] subsets u;
        feature y2[3..4] redefines y;
        feature z2[0..1] subsets z;
        feature v1[*] subsets v;
    }
    assoc E {
        end feature e1[1];
        end feature e2[1];
    }
    // Ends are compared with ends: a warning on e3 (upper bound only; the
    // lower bound is not checked for ends), none on e4.
    assoc E2 specializes E {
        end feature e3[*] redefines e1;
        end feature e4[0..1] redefines e2;
    }
    // No multiplicity check for a feature that is not an end subsetting an
    // end; validateSubsettingFeaturingTypes (e1 is featured by E).
    class F {
        feature f1[*] subsets E::e1;
    }
}
