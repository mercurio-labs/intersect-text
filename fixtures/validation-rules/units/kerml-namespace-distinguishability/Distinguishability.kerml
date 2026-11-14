package Distinguishability {
    // validateNamespaceDistinguishablity (warnings), with non-firing controls.

    // Owned members: "Duplicate of other owned member name" on both.
    classifier A;
    classifier A;
    classifier <s> B;
    classifier <s> C;
    // Control: a name and a short name of one element.
    classifier <G> G;

    // Aliases: "Duplicate of owned member name" (D names B, not the owned D),
    // "Duplicate of other alias name" on both E aliases; F aliases the
    // same element twice (control).
    classifier D;
    alias D for B;
    alias E for B;
    alias E for C;
    alias F for C;
    alias F for C;

    // Inherited members.
    classifier P { feature x; feature y; }
    classifier Q { feature x; }
    // "Duplicate of inherited member name 'x' from P, Q" on R.
    classifier R specializes P, Q;
    // "Duplicate of inherited member name 'y' from P" on S::y.
    classifier S specializes P { feature y; }
    // Controls: a redefinition hides the redefined member; an unnamed
    // redefinition is named after the feature it redefines.
    classifier T specializes P { feature y redefines P::y; }
    classifier U specializes P { feature redefines x; feature z; }
    // The unnamed redefinition of P::x is named x and hides P::x but not
    // Q::x: "Duplicate of inherited member name 'x' from Q" on it.
    classifier V specializes R { feature redefines P::x; }

    // Sources sorted in Java String order and escaped as names:
    // "... 'w' from Zebra, apple, 'one two'" on W.
    classifier apple { feature w; }
    classifier Zebra { feature w; }
    classifier 'one two' { feature w; }
    classifier W specializes apple, Zebra, 'one two';

    // A diamond brings one member twice: no warning.
    classifier X1 specializes P;
    classifier X2 specializes P;
    classifier X3 specializes X1, X2;

    // An alias in a type: "Duplicate of inherited member name 'x' from P".
    classifier Y specializes P { alias x for Q; }
}
