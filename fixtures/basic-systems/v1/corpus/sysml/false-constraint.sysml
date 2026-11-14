package FalseConstraint {
    private import ScalarValues::*;
    private import ScalarFunctions::*;
    attribute measured : Integer = 11;
    attribute limit : Integer = 10;
    assert constraint bounded { measured <= limit }
}
