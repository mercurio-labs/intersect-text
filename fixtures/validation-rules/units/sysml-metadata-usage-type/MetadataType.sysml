package MetadataType {
    metadata def M1;
    metadata def M2 :> M1;
    part def P;

    // Error: a KerML metaclass is a metaclass, but not a specialization of
    // Metadata::MetadataItem, which a metadata usage implicitly specializes,
    // so the usage has two types.
    metadata m1 : KerML::Feature about P;

    // No error: a metadata definition, its specialization, a SysML
    // metaclass (a metadata definition of the Systems Library) and the
    // prefix notation.
    metadata m2 : M1 about P;
    metadata m3 : M2 about P;
    metadata m4 : SysML::PartUsage about P;
    #M1 part q;
}
