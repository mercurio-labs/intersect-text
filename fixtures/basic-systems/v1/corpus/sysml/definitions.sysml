package BasicDefinitions {
    private import ScalarValues::*;
    port def DataPort {
        out attribute value : Integer;
    }
    part def Sensor {
        attribute sample : Integer = 2;
        port output : DataPort;
    }
    part def Controller {
        port input : ~DataPort;
    }
}
