package Bracket {
    private import ScalarValues::*;
    feature a : Integer[*] = (1, 2, 3);
    // Fires (warning): '[' indexing in KerML.
    feature b : Integer = a[1];
    // Does not fire.
    feature c : Integer = a#(1);
}
