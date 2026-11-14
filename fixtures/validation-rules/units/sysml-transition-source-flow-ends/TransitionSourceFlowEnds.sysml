package TransitionSourceFlowEnds {
    item def Sig;

    state def S1 {
        action a1;
        // Error: a target transition whose source (the previous feature) is
        // an action, not a state.
        accept Sig then s1;
        state s1;
    }

    state def S2 {
        state s0;
        // No error: the previous feature is a state.
        accept Sig then s1;
        state s1;
        // No error: transitions are skipped when looking for the source.
        accept Sig then s0;
    }

    flow def F2 { end a; end b; }
    // Error on the third owned end (c and d redefine a and b).
    flow def F3 :> F2 {
        end c;
        end d;
        end e;
    }
    // No error: x redefines a, and b is inherited.
    flow def F4 :> F2 { end x; }
}
