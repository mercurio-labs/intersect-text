package InstantiatedType {
    package Q;
    function F { in x; return r; }
    class C;
    // Fires: the invoked and instantiated elements are packages, not types.
    feature i = Q(1);
    feature n = new Q();
    // Does not fire.
    feature j = F(1);
    feature m = new C();
}
