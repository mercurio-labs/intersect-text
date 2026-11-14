package SysmlDistinguishability {
    // validateNamespaceDistinguishablity (warnings) through the implicit
    // library supertypes of usages, with non-firing controls. Usages typed
    // by definitions of another kind also get the Pilot's B3 typing errors.
    attribute def AD;
    part def PD;
    port def PoD;

    // "... 'self' from DataValue, Part" on p1.
    part p1 : AD;
    // "... 'done'", "'start'" and "'self' from Action, Part" on a1.
    action a1 : PD;
    // "... 'self' from Part, Port" on p3.
    part p3 : PoD;
    // Controls.
    part p2 : PD;
    attribute v : AD;
    action a2;

    // Parameters redefine the general's parameters at their position.
    action def A0 { in item x; out item y; }
    action def A1 :> A0 { in item x; out item y; }
    action a3 : A0 { in item x; }

    // "... 'm' from W" on W2::m; W3 redefines m (control).
    part def W { attribute m; }
    part def W2 :> W { attribute m; }
    part def W3 :> W { attribute :>> m; }

    // Owned members of a definition: "Duplicate of other owned member name"
    // on both.
    part def W4 { part w : W; part w; }
}
