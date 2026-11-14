package VariantMembership {
    part def Engine;

    // Fires: a variant owned by a part definition that is not a variation.
    part def Car {
        variant part engine : Engine;
    }

    // Do not fire: variants of a variation definition and a variation usage.
    variation part def EngineChoice {
        variant part small : Engine;
        variant part large : Engine;
    }
    part def Truck {
        variation part engine : Engine {
            variant part diesel : Engine;
        }
    }
}
