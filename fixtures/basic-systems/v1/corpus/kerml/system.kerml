package BasicSystem {
    private import BasicLibrary::*;
    private import ScalarValues::*;
    private import ScalarFunctions::*;
    feature source : Sensor;
    feature destination : Sensor;
    connector signal from source to destination;
    feature total : Integer = 1 + 2 * 3;
    feature doubled : Integer = total * 2;
}
