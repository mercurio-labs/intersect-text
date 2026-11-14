package SendReceiver {
    item def Msg;
    port def P;
    part def Sys {
        port p : P;
        part q;
        part inner {
            port r : P;
        }
        item m : Msg;
        action a {
            // Warn: sending "to" a port, by name, by qualified name and as
            // the last feature of a chain.
            send m to p;
            send m to Sys::p;
            send m to inner.r;

            // Do not warn: "to" a part, and "via" a port.
            send m to q;
            send m via p;
        }
    }
}
