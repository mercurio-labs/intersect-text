# Separate optional regeneration continuation

This additive history preserves the successful selected-cohort continuation,
including complete raw generation, javac, metadata, lexer, resource, named-parser,
node/leaf observations, command output and artifact manifests. It changes neither
the original frozen corpus nor its separate failed full-attempt history.

The original hsp1 attempt remains a process timeout (124 under the unchanged
15-second deadline), despite complete equivalent JSON. The new hsp2 attempt
independently records and regenerates discriminator2, qualified2, reference2 and
suffix1: seven graphs per run, 65 resource observations, 86 named JSON
observations, and 14 source-failure unexecuted resource requests. Availability
failures remain distinct from executed-parser rejection; see review.json.

Only the exact absolute owned provider GenModel output path is excepted from
comparison. No input/source URI, identity, classifier, diagnostic, node, span or
outcome normalization applies. No single all-seven-cohort attempt passed.
Java remains verification-only; this addendum awards no support gate.
