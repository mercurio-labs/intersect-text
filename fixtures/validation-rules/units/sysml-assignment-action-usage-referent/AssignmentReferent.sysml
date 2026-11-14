package AssignmentReferent {
    part def Engine;
    item def Tank {
        attribute level;
    }
    action def Fill {
        in item tank : Tank;

        // Fires: the referent names a part definition, not a feature
        // (reported on the assignment's second owned relationship).
        assign Engine := 1;

        // Does not fire: the referent is a time-varying attribute of an item.
        assign tank.level := 1;
    }
}
