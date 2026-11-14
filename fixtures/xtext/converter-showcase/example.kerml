package //* hidden note */ Demo /* significant package documentation */ {
  abstract type Vehicles::'Wheel assembly' visibility public operator if count 12 magnitude 1e3 description "Reusable wheel";
  feature front ordered nonunique direction out enabled true :> Vehicles::'Wheel assembly';
  feature spare nonunique ordered direction inout enabled false :> Library::Spare; // cross-resource reference
  feature standard :> Library::Spare; // default uniqueness retained
}
