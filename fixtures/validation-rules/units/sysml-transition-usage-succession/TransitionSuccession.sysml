package TransitionSuccession {
    state def S {
        entry; then s1;
        state s1;
        part x;
        state s2;

        // Fires: the succession's target is a part, not an action.
        transition t1 first s1 then x;

        // Does not fire: the target is a state.
        transition t2 first s1 then s2;
    }
}
