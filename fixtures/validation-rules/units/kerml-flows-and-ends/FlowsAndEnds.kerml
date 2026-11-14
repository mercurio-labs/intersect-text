package FlowsAndEnds {
    // Flow end, connector and association end rules (batch B4) with
    // non-firing controls.
    class P {
        out feature o;
        in feature i;
    }
    class Sys {
        feature p : P;
        feature q : P;
        // Control: dot notation.
        flow f from p.o to q.i;
        // validateFlowEndSubsetting on both ends, which have no related
        // feature (validateConnectorRelatedFeatures) and redefine features
        // they cannot access (validateSubsettingFeaturingTypes).
        flow g from P::o to P::i;
    }
    // validateAssociationBinarySpecialization on the third end.
    assoc Bin {
        end feature s;
        end feature t;
    }
    assoc Tri specializes Bin {
        end feature s2;
        end feature t2;
        end feature u;
    }
    // Control: three ends, not binary.
    assoc Tri2 {
        end feature a1;
        end feature a2;
        end feature a3;
    }
}
