package SysmlMetadata {
    // validateMetadataFeatureMetadataNotAbstract (KerML checkMetadataFeature
    // on SysML metadata usages), with non-firing controls.
    abstract metadata def AMD;
    metadata def MD;
    part def P {
        // Must have a concrete type.
        @AMD;
        // Controls.
        @MD;
        metadata m : MD;
    }
}
