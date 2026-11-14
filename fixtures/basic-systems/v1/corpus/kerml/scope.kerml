package ScopedSystem {
    private import BasicLibrary::*;
    package Local {
        type Sensor :> Base::Anything;
        feature localSensor : Sensor;
        feature importedSensor : BasicLibrary::Sensor;
    }
}
