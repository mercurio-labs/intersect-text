package BasicLibrary {
    public import Base::*;
    type Component :> Anything {
        feature serial : ScalarValues::Integer;
    }
    type Sensor :> Component;
}
