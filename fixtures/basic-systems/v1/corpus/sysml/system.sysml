package BasicAssembly {
    private import BasicDefinitions::*;
    private import ScalarValues::*;
    private import ScalarFunctions::*;
    part def Assembly {
        part sensor : Sensor;
        part controller : Controller;
        part spareController : Controller;
        attribute limit : Integer = 10;
        connect sensor.output to controller.input;
        assert constraint bounded { sensor.sample <= limit }
    }
    part system : Assembly;
    requirement def BoundedReading {
        subject unit : Assembly;
        require constraint { unit.sensor.sample <= unit.limit }
    }
    requirement boundedReading : BoundedReading;
    satisfy boundedReading by system;
}
