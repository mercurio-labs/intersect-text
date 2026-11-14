package InheritedObjectives {
    case def L { objective l1; }
    case def M { objective m1; }

    // Error on the definition: two inherited objectives.
    case def N :> L, M;

    // No error: the first owned objective redefines both.
    case def P :> L, M { objective p1; }

    // Error on the second owned objective only (the first redefines l1).
    case def Q :> L {
        objective q1;
        objective q2;
    }

    // Error on the usage: two objectives inherited through its definition.
    case n1 : N;

    // Views. Error on the usage: two inherited renderings.
    rendering def R;
    view def W { render rendering w1 : R; }
    view def W3 { render rendering w3 : R; }
    view w4 : W, W3;

    // No error: one inherited rendering, and an owned rendering, which
    // hides the inherited ones (every view rendering redefines
    // Views::View::viewRendering).
    view w5 : W;
    view def W2 :> W { render rendering w2 : R; }
    view w6 : W, W3 { render rendering w7 : R; }
}
