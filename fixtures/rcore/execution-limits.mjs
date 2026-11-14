// Shared host admission configuration; Rust owns execution semantics.
export const executionLimits = {
  scalar: { max_steps: 100000, max_value_bytes: 4194304,
    values: { max_value_bytes: 262144, max_digits: 4096 },
    numeric: { max_digits: 4096, max_intermediate_digits: 8192, max_scale_alignment: 4096,
      max_value_bytes: 262144, max_total_value_bytes: 1048576, max_work: 16777216 } },
  max_frames: 256, max_closures: 65536, max_captures: 65536, max_value_nodes: 65536,
  max_value_depth: 64, max_collection_comparisons: 1048576, max_trace_frames: 256,
  max_trace_bytes: 65536, max_model_contexts: 8, max_reference_domains: 128,
  max_context_bytes: 16777216, max_model_elements: 100000, max_membership_checks: 1048576,
  max_provider_evidence_bytes: 65536, max_provider_invocations: 1048576,
  max_provider_work: 1048576, max_model_storage_bytes: 16777216,
};
