package //* hidden note */ Demo /* significant package documentation */ {
  abstract type Vehicles::'Wheel assembly' operator if count 12 magnitude 1e3 description "Reusable wheel";
  feature front :> Vehicles::'Wheel assembly';
  feature spare :> Library::Spare; // cross-resource reference
}
