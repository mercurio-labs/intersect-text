//! Native source-admission proof only; executable package failures remain explicit.
#[path = "support/first_rule_proof_gap_observations.rs"]
mod observations;

#[test]
fn original_five_graphs_and_source_authority_controls_match_pinned_oracle() -> Result<(), String> {
    observations::suite().map(|_| ())
}
