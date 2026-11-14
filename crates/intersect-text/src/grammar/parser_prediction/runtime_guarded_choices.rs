// Private native obligations over the SAME source/graph completion verifier.
// Included inside runtime_guarded::implementation; never serialized authority.
fn choice_lookup(len: usize, key: usize, work: &mut usize) -> Result<(), String> {
    let levels = usize::BITS as usize - len.max(1).leading_zeros() as usize;
    let units = levels
        .checked_mul(2)
        .and_then(|n| n.checked_mul(1 + key.div_ceil(64)))
        .and_then(|n| n.checked_add(1))
        .ok_or("Runtime choice lookup work overflow")?;
    for _ in 0..units {
        pay(work)?;
    }
    Ok(())
}

fn choice_bindings_equal(
    record: &ChoiceBinding,
    actual: &ChoiceBinding,
    work: &mut usize,
) -> Result<bool, String> {
    let chunks = record
        .owner
        .len()
        .max(actual.owner.len())
        .div_ceil(64)
        .checked_add(
            record
                .source
                .uri
                .len()
                .max(actual.source.uri.len())
                .div_ceil(64),
        )
        .and_then(|n| {
            n.checked_add(
                record
                    .source
                    .sha256
                    .len()
                    .max(actual.source.sha256.len())
                    .div_ceil(64),
            )
        })
        .and_then(|n| {
            record
                .entries
                .len()
                .max(actual.entries.len())
                .checked_mul(4)
                .and_then(|entries| n.checked_add(entries))
        })
        .and_then(|n| n.checked_add(12))
        .ok_or("Runtime choice comparison overflow")?;
    for _ in 0..chunks {
        pay(work)?;
    }
    Ok(record == actual)
}

fn choice_binding(
    grammar: &Grammar,
    program: &ParserPrediction,
    path: &str,
    proof: &ChoiceProof,
    work: &mut usize,
    logical: &mut usize,
) -> Result<ChoiceBinding, String> {
    // Prepay fixed range/shape/header checks and temporary cleanup.
    for _ in 0..32 {
        pay(work)?;
    }
    let base = path
        .strip_suffix("/choice")
        .ok_or("Runtime choice has no exact suffix")?;
    let (owner, rule, expression) =
        super::super::super::nullable_repetition::prefix_source_occurrence_with_work(
            grammar, base, work,
        )?
        .ok_or("Runtime choice has no canonical source occurrence")?;
    choice_lookup(grammar.declaration_sources.len(), owner.len(), work)?;
    let source = grammar
        .declaration_sources
        .get(owner)
        .ok_or("Runtime choice has no original owner")?;
    let range = expression
        .original_range
        .as_ref()
        .ok_or("Runtime choice lacks original range")?;
    pay(work)?;
    if source.start >= source.end
        || range.start >= range.end
        || range.start < source.start
        || range.end > source.end
        || expression.cardinality != '1'
        || expression.first_set_predicated
        || !matches!(&expression.kind, Kind::Alternative(items) if items.len() >= 2)
        || rule
            .body
            .original_range
            .as_ref()
            .is_none_or(|r| r.start < source.start || r.end > source.end || r.start >= r.end)
    {
        return Err("Runtime choice source/owner interval differs".into());
    }
    choice_lookup(program.decisions.len(), path.len(), work)?;
    let decision = program
        .decisions
        .get(path)
        .ok_or("Runtime choice graph is absent")?;
    let Kind::Alternative(items) = &expression.kind else {
        return Err("Runtime choice is not an original Alternative".into());
    };
    if decision.entries.len() < 2 || items.len() != decision.entries.len() {
        return Err("Runtime choice fanout differs".into());
    }
    admit(
        logical,
        base.len()
            .checked_add(9)
            .ok_or("Runtime choice temporary path overflow")?,
        work,
    )?;
    let recovery_key = format!("{base}/recovery");
    choice_lookup(program.decisions.len(), recovery_key.len(), work)?;
    let parent = program
        .decisions
        .get(&recovery_key)
        .ok_or("Runtime choice lacks canonical expression endpoints")?;
    let [expression_start, expression_end] = parent.entries.as_slice() else {
        return Err("Runtime choice expression endpoints differ".into());
    };
    pay(work)?;
    if parent.exit.is_some() {
        return Err("Runtime choice expression endpoints have an exit".into());
    }
    let fanout = program
        .states
        .get(*expression_start)
        .ok_or("Runtime choice expression start is dangling")?;
    pay(work)?;
    if fanout.edges.len() != decision.entries.len() {
        return Err("Runtime choice canonical fanout differs".into());
    }
    let entry_bytes = decision
        .entries
        .len()
        .checked_mul(4 * intersect_model::extensions::LOGICAL_WORD)
        .ok_or("Runtime choice entries storage overflow")?;
    // Variable payload, equality reads and retention are paid before allocation.
    admit(logical, entry_bytes, work)?;
    let mut entries = Vec::new();
    let mut guards = Vec::new();
    guards
        .try_reserve_exact(decision.entries.len())
        .map_err(|_| "Runtime choice guard allocation denied")?;
    if guards.capacity() != decision.entries.len() {
        return Err("Runtime choice guard allocation capacity differs".into());
    }
    entries
        .try_reserve_exact(decision.entries.len())
        .map_err(|_| "Runtime choice entries allocation denied")?;
    if entries.capacity() != decision.entries.len() {
        return Err("Runtime choice entries allocation capacity differs".into());
    }
    for (ordinal, (&entry, item)) in decision.entries.iter().zip(items).enumerate() {
        for _ in 0..6 {
            pay(work)?;
        }
        let child = item
            .original_range
            .as_ref()
            .ok_or("Runtime choice alternative lacks original range")?;
        if child.start >= child.end || child.start < range.start || child.end > range.end {
            return Err("Runtime choice alternative has a foreign source interval".into());
        }
        let state = program
            .states
            .get(entry)
            .ok_or("Runtime choice entry is dangling")?;
        if state.accepting || state.rule_stop || state.edges.is_empty() {
            return Err("Runtime choice entry has an invalid state shape".into());
        }
        for &previous in &entries {
            pay(work)?;
            if previous == entry {
                return Err("Runtime choice repeats an ordered entry".into());
            }
        }
        if ordinal > 0 {
            pay(work)?;
            let previous = items[ordinal - 1]
                .original_range
                .as_ref()
                .ok_or("Runtime choice previous alternative lacks original range")?;
            if previous.end > child.start {
                return Err("Runtime choice alternative source order differs".into());
            }
        }
        // The exact source occurrence path is assigned by the SAME expression
        // builder that emitted this branch. Bind each entry and common end,
        // including alternatives after the first pair.
        // usize decimal digits fit within 20 on supported native/WASM targets.
        let key_bytes = base
            .len()
            .checked_mul(2)
            .and_then(|n| n.checked_add(112))
            .ok_or("Runtime choice temporary path storage overflow")?;
        admit(logical, key_bytes, work)?;
        let child_path = format!("{base}/a{ordinal}");
        for _ in 0..1 + child_path.len().div_ceil(64) {
            pay(work)?;
        }
        if item.prediction_path.as_deref() != Some(child_path.as_str()) {
            return Err("Runtime choice alternative canonical source path differs".into());
        }
        let child_key = format!("{child_path}/recovery");
        choice_lookup(program.decisions.len(), child_key.len(), work)?;
        let child_graph = program
            .decisions
            .get(&child_key)
            .ok_or("Runtime choice alternative has no canonical endpoints")?;
        for _ in 0..5 {
            pay(work)?;
        }
        if child_graph.exit.is_some()
            || child_graph.entries.as_slice() != [entry, *expression_end]
            || !matches!(fanout.edges.get(ordinal), Some(Edge::Epsilon(target)) if *target == entry)
        {
            return Err("Runtime choice alternative source/graph endpoints differ".into());
        }
        // Same conservative prefix-walk precharge as the existing exact
        // source-priority binder; native builder depth is bounded at64.
        for _ in 0..128 {
            pay(work)?;
        }
        let source_guard = super::super::super::visible_entry_guard_expression(item);
        let graph_guard = match state.edges.as_slice() {
            [
                Edge::First {
                    path: actual_path,
                    target,
                },
            ] => {
                let guard =
                    source_guard.ok_or("Runtime choice graph First lacks visible source guard")?;
                let guard_path = guard
                    .prediction_path
                    .as_deref()
                    .ok_or("Runtime choice source First lacks path")?;
                for _ in 0..3 + guard_path.len().max(actual_path.len()).div_ceil(64) {
                    pay(work)?;
                }
                if !guard.first_set_predicated || guard_path != actual_path {
                    return Err("Runtime choice First source/graph guard differs".into());
                }
                choice_lookup(program.first_guards.len(), guard_path.len(), work)?;
                if program
                    .first_guards
                    .get(guard_path)
                    .is_none_or(|symbols| symbols.is_empty())
                {
                    return Err("Runtime choice First lacks canonical predicate symbols".into());
                }
                pay(work)?;
                if program.states.get(*target).is_none() {
                    return Err("Runtime choice First target is dangling".into());
                }
                [1, *target, *expression_end]
            }
            [Edge::Full { target, stop }] => {
                let guard =
                    source_guard.ok_or("Runtime choice graph Full lacks visible source guard")?;
                let Kind::SyntacticPredicate(operand) = &guard.kind else {
                    return Err("Runtime choice Full source kind differs".into());
                };
                let operand_path = operand
                    .prediction_path
                    .as_deref()
                    .ok_or("Runtime choice Full operand lacks path")?;
                let guard_path = guard
                    .prediction_path
                    .as_deref()
                    .ok_or("Runtime choice Full guard lacks path")?;
                for _ in 0..3 + operand_path.len().max(guard_path.len()).div_ceil(64) {
                    pay(work)?;
                }
                if guard.first_set_predicated
                    || operand_path.strip_suffix("/value") != Some(guard_path)
                {
                    return Err("Runtime choice Full operand source differs".into());
                }
                let bytes = operand_path
                    .len()
                    .checked_add(9)
                    .ok_or("Runtime choice Full path overflow")?;
                admit(logical, bytes, work)?;
                let key = format!("{operand_path}/recovery");
                choice_lookup(program.decisions.len(), key.len(), work)?;
                let bound = program
                    .decisions
                    .get(&key)
                    .ok_or("Runtime choice Full lacks operand endpoints")?;
                pay(work)?;
                if bound.exit.is_some() || bound.entries.as_slice() != [*target, *stop] {
                    return Err("Runtime choice Full operand graph endpoints differ".into());
                }
                [2, *target, *stop]
            }
            _ => {
                if source_guard.is_some() {
                    return Err(
                        "Runtime choice visible source guard is absent from graph entry".into(),
                    );
                }
                [0, entry, *expression_end]
            }
        };
        if let Some(guard) = source_guard {
            pay(work)?;
            if guard
                .original_range
                .as_ref()
                .is_none_or(|r| r.start >= r.end || r.start < child.start || r.end > child.end)
            {
                return Err("Runtime choice visible guard has foreign source interval".into());
            }
        }
        guards.push(graph_guard);
        entries.push(entry);
    }
    // Copy the explicit constructor's finite proof kind; cardinality is never
    // authority for the independent asymmetric-pair witness.
    let proof = match proof {
        ChoiceProof::AsymmetricPair => ChoiceProof::AsymmetricPair,
        ChoiceProof::ConditionalMultiway => ChoiceProof::ConditionalMultiway,
    };
    pay(work)?;
    if decision.exit.is_some() {
        return Err("Runtime choice is not an ordinary alternative".into());
    }
    choice_lookup(program.rules.len(), owner.len(), work)?;
    let &(start, stop) = program
        .rules
        .get(owner)
        .ok_or("Runtime choice owner has no graph")?;
    pay(work)?;
    let state = program
        .states
        .get(stop)
        .ok_or("Runtime choice owner stop is dangling")?;
    if !state.rule_stop || state.accepting || !state.edges.is_empty() {
        return Err("Runtime choice owner has no canonical rule stop".into());
    }
    let bytes = RECORD
        .checked_add(
            path.len()
                .checked_mul(2)
                .ok_or("Runtime choice key overflow")?,
        )
        .and_then(|n| n.checked_add(owner.len()))
        .and_then(|n| n.checked_add(source.source.uri.len()))
        .and_then(|n| n.checked_add(source.source.sha256.len()))
        .ok_or("Runtime choice retention overflow")?;
    admit(logical, bytes, work)?;
    Ok(ChoiceBinding {
        owner: owner.into(),
        source: source.source.clone(),
        declaration: source.start..source.end,
        occurrence: range.clone(),
        graph: [start, stop],
        entries,
        guards,
        proof,
    })
}

// This is a source-policy proof, not a second prediction interpreter. Every
// syntactic callee reachable from the actual entry is included, even after the
// selected owner's return or inside a guard. None inherits the uniform global
// policy; an explicit unequal override cannot be lent to the token-only worker.
enum ChoiceHiddenVisit<'g> {
    Rule(&'g str, bool),
    Expression(&'g Expr, usize),
    Children(&'g [Expr], usize, usize),
}
struct ChoiceHiddenClosure<'g, 'w> {
    grammar: &'g Grammar,
    owner: &'w str,
    work: &'w mut usize,
    logical: usize,
    seen: std::collections::BTreeSet<&'g str>,
    found_owner: bool,
}
impl<'g> ChoiceHiddenClosure<'g, '_> {
    fn rule(&mut self, name: &'g str, required: bool) -> Result<Option<&'g Expr>, String> {
        pay(self.work)?;
        choice_lookup(self.seen.len(), name.len(), self.work)?;
        if self.seen.contains(name) {
            return Ok(None);
        }
        let grammar = self.grammar;
        choice_lookup(grammar.rules.len(), name.len(), self.work)?;
        let Some(rule) = grammar.rules.get(name) else {
            if !required {
                // SAME canonical Call split: a non-parser name is a terminal,
                // including inherited built-ins absent from the explicit map.
                // It has no rule hidden override. This proves only hidden
                // uniformity; existing graph/token validation is not bypassed.
                return Ok(None);
            }
            return Err("Runtime choice hidden proof has an unknown datatype callee".into());
        };
        choice_lookup(grammar.declaration_sources.len(), name.len(), self.work)?;
        let source = grammar
            .declaration_sources
            .get(name)
            .ok_or("Runtime choice hidden proof lacks original rule source")?;
        pay(self.work)?;
        if rule.body.original_range.as_ref().is_none_or(|range| {
            source.start >= source.end
                || range.start < source.start
                || range.end > source.end
                || range.start >= range.end
        }) {
            return Err("Runtime choice hidden proof has a foreign source interval".into());
        }
        if let Some(hidden) = &rule.hidden {
            pay(self.work)?;
            if let Some(global) = &grammar.hidden {
                for token in hidden.iter().chain(global.iter()) {
                    for _ in 0..1 + token.len().div_ceil(64) {
                        pay(self.work)?;
                    }
                }
            }
            if Some(hidden) != grammar.hidden.as_ref() {
                return Err(format!(
                    "Runtime choice reachable rule switches hidden policy: {name}"
                ));
            }
        }
        // Borrowed-key node/descriptor storage and the actual ordered probe are
        // admitted before insertion. This scratch service uses the same finite
        // work and existing 1MiB logical ceiling; it retains no runtime cache.
        admit(&mut self.logical, 512, self.work)?;
        choice_lookup(self.seen.len(), name.len(), self.work)?;
        self.seen.insert(name);
        for _ in 0..1 + name.len().max(self.owner.len()).div_ceil(64) {
            pay(self.work)?;
        }
        self.found_owner |= name == self.owner;
        Ok(Some(&rule.body))
    }
    fn push(
        &mut self,
        stack: &mut Vec<ChoiceHiddenVisit<'g>>,
        visit: ChoiceHiddenVisit<'g>,
    ) -> Result<(), String> {
        pay(self.work)?;
        if stack.len() == stack.capacity() {
            let capacity = stack
                .capacity()
                .max(2)
                .checked_mul(2)
                .ok_or("Runtime choice hidden scratch capacity overflow")?;
            let bytes = capacity
                .checked_mul(intersect_model::logical_size!(ChoiceHiddenVisit<'g>, 40))
                .ok_or("Runtime choice hidden scratch storage overflow")?;
            // Every replacement allocation and actual descriptor move is paid
            // before allocation. Old buffers are not refunded from this pool.
            admit(&mut self.logical, bytes, self.work)?;
            let moves = stack
                .len()
                .checked_mul(1 + intersect_model::logical_size!(ChoiceHiddenVisit<'g>, 40).div_ceil(64))
                .ok_or("Runtime choice hidden scratch move overflow")?;
            for _ in 0..moves {
                pay(self.work)?;
            }
            let mut next = Vec::new();
            next.try_reserve_exact(capacity)
                .map_err(|_| "Runtime choice hidden scratch allocation failed")?;
            if next.capacity() != capacity {
                return Err("Runtime choice hidden scratch capacity differs".into());
            }
            next.append(stack);
            *stack = next;
        }
        stack.push(visit);
        Ok(())
    }
    fn run(&mut self) -> Result<(), String> {
        let mut stack = Vec::new();
        self.push(
            &mut stack,
            ChoiceHiddenVisit::Rule(&self.grammar.entry, true),
        )?;
        while !stack.is_empty() {
            pay(self.work)?;
            let visit = stack
                .pop()
                .ok_or("Runtime choice hidden scratch disappeared")?;
            match visit {
                ChoiceHiddenVisit::Rule(name, required) => {
                    if let Some(body) = self.rule(name, required)? {
                        // A rule call is graph reachability, not a nested Expr.
                        // Preserve the per-body structural128 boundary; runtime
                        // caller depth128 is independently enforced unchanged.
                        self.push(&mut stack, ChoiceHiddenVisit::Expression(body, 1))?;
                    }
                }
                ChoiceHiddenVisit::Expression(expression, depth) => {
                    pay(self.work)?;
                    if depth > 128 {
                        return Err("Runtime choice hidden expression depth exceeded".into());
                    }
                    match &expression.kind {
                        // SAME Builder.body implicit ID/parser/terminal split.
                        Kind::Reference(_) => {
                            self.push(&mut stack, ChoiceHiddenVisit::Rule("ID", false))?
                        }
                        Kind::Call(name) | Kind::ScalarReference { rule: name, .. } => {
                            self.push(&mut stack, ChoiceHiddenVisit::Rule(name, false))?;
                        }
                        Kind::Sequence(items) | Kind::Alternative(items) => {
                            self.push(
                                &mut stack,
                                ChoiceHiddenVisit::Children(items, 0, depth + 1),
                            )?;
                        }
                        Kind::Assign { value, .. }
                        | Kind::BooleanAssign { value, .. }
                        | Kind::SyntacticPredicate(value) => {
                            self.push(&mut stack, ChoiceHiddenVisit::Expression(value, depth + 1))?;
                        }
                        Kind::Action(_) | Kind::Capture { .. } | Kind::Literal(_) => {}
                    }
                }
                ChoiceHiddenVisit::Children(items, index, depth) => {
                    pay(self.work)?;
                    if let Some(item) = items.get(index) {
                        // A continuation preserves original left-to-right DFS
                        // and source-error order without a recursive call stack.
                        self.push(
                            &mut stack,
                            ChoiceHiddenVisit::Children(items, index + 1, depth),
                        )?;
                        self.push(&mut stack, ChoiceHiddenVisit::Expression(item, depth))?;
                    }
                }
            }
        }
        Ok(())
    }
}
fn choice_uniform_hidden(grammar: &Grammar, owner: &str, work: &mut usize) -> Result<(), String> {
    let mut closure = ChoiceHiddenClosure {
        grammar,
        owner,
        work,
        logical: 0,
        seen: std::collections::BTreeSet::new(),
        found_owner: false,
    };
    // Retain the prior conservative128-frame allowance, plus the actual Vec
    // descriptor. Every iterative backing allocation/move is paid separately.
    // This is not a process-stack/RSS measurement.
    admit(
        &mut closure.logical,
        intersect_model::logical_size!(ChoiceHiddenClosure<'_, '_>, 72)
            + intersect_model::logical_size!(Vec<ChoiceHiddenVisit<'_>>, 24)
            + 128 * 128,
        closure.work,
    )?;
    closure.run()?;
    pay(closure.work)?;
    if !closure.found_owner {
        return Err("Runtime choice owner is not reachable from the actual grammar entry".into());
    }
    Ok(())
}

fn multiway_hidden_owners<'g>(
    grammar: &'g Grammar,
    work: &mut usize,
) -> Result<std::collections::BTreeSet<&'g str>, String> {
    let mut closure = ChoiceHiddenClosure {
        grammar,
        owner: grammar.entry.as_str(),
        work,
        logical: 0,
        seen: std::collections::BTreeSet::new(),
        found_owner: false,
    };
    admit(
        &mut closure.logical,
        intersect_model::logical_size!(ChoiceHiddenClosure<'_, '_>, 72)
            + intersect_model::logical_size!(Vec<ChoiceHiddenVisit<'_>>, 24)
            + 128 * 128,
        closure.work,
    )?;
    closure.run()?;
    Ok(closure.seen)
}

// A recordless recursive decision is eligible only for the SAME canonical
// early-guard commitment already accepted by Legacy/Strict completion. The
// private fresh-builder companion remains the surrounding authority. Rebind
// the exact source fan-out and every visible guard; scope success is not proof.
fn required_source_entry_priority(
    grammar: &Grammar,
    program: &ParserPrediction,
    bound: &super::super::BoundCompletionDecision<'_>,
    scope: &super::super::CompletionScope<'_>,
    work: &mut usize,
) -> Result<bool, String> {
    if !program.completion_entry_priority(bound.decision, work)? {
        return Ok(false);
    }
    let base = bound
        .path
        .strip_suffix("/choice")
        .or_else(|| bound.path.strip_suffix("/repeat"))
        .ok_or("Required priority has no canonical decision suffix")?;
    let (owner, rule, expression) =
        super::super::super::nullable_repetition::prefix_source_occurrence_with_work(
            grammar, base, work,
        )?
        .ok_or("Required priority has no canonical source occurrence")?;
    for _ in 0..1 + base.len().div_ceil(64) {
        pay(work)?;
    }
    if expression.prediction_path.as_deref() != Some(base) {
        return Err("Required priority source prediction path differs".into());
    }
    choice_lookup(grammar.declaration_sources.len(), owner.len(), work)?;
    let source = grammar
        .declaration_sources
        .get(owner)
        .ok_or("Required priority has no original owner source")?;
    pay(work)?;
    let retained = |expr: &Expr| {
        expr.original_range.as_ref().is_some_and(|range| {
            source.start < source.end
                && source.start <= range.start
                && range.start < range.end
                && range.end <= source.end
        })
    };
    if !retained(expression) || !retained(&rule.body) {
        return Err("Required priority has a foreign original interval".into());
    }
    let alternatives = if bound.path.ends_with("/choice") {
        let Kind::Alternative(items) = &expression.kind else {
            return Err("Required priority choice source kind differs".into());
        };
        pay(work)?;
        if expression.cardinality != '1'
            || expression.first_set_predicated
            || bound.decision.exit.is_some()
            || items.len() != bound.decision.entries.len()
        {
            return Err("Required priority source choice fanout differs".into());
        }
        Some(items.as_slice())
    } else {
        pay(work)?;
        if !matches!(expression.cardinality, '?' | '*' | '+')
            || bound.decision.exit != Some(1)
            || bound.decision.entries.len() != 2
        {
            return Err("Required priority source repetition differs".into());
        }
        None
    };
    let mut logical = 0;
    for (index, &entry) in bound
        .decision
        .entries
        .iter()
        .take(bound.decision.entries.len().saturating_sub(1))
        .enumerate()
    {
        pay(work)?;
        let source_entry = match alternatives {
            Some(items) => items
                .get(index)
                .ok_or("Required priority source entry is absent")?,
            None => expression,
        };
        // This calls the SAME existing source predicate, now retaining its
        // guard reference. The native builder has already enforced structural
        // depth64; prepay the conservative128-node prefix before that old walk.
        for _ in 0..128 {
            pay(work)?;
        }
        let guard = super::super::super::visible_entry_guard_expression(source_entry)
            .ok_or("Required priority lacks a visible source entry guard")?;
        if !retained(source_entry) || !retained(guard) {
            return Err("Required priority guard has a foreign source interval".into());
        }
        let guard_path = guard
            .prediction_path
            .as_deref()
            .ok_or("Required priority guard lacks its bound source path")?;
        pay(work)?;
        let state = program
            .states
            .get(entry)
            .ok_or("Dangling required priority entry")?;
        choice_lookup(scope.guards.len(), 0, work)?;
        if guard.first_set_predicated {
            let [super::super::Edge::First { path, .. }] = state.edges.as_slice() else {
                return Err("Required priority source First differs from graph guard".into());
            };
            for _ in 0..1 + path.len().max(guard_path.len()).div_ceil(64) {
                pay(work)?;
            }
            if path != guard_path
                || scope.guards.get(&entry)
                    != Some(&super::super::CompletionGuard::First(guard_path))
            {
                return Err("Required priority First source/visibility differs".into());
            }
        } else {
            let Kind::SyntacticPredicate(operand) = &guard.kind else {
                return Err("Required priority source Full kind differs".into());
            };
            let [super::super::Edge::Full { target, stop }] = state.edges.as_slice() else {
                return Err("Required priority source Full differs from graph guard".into());
            };
            if scope.guards.get(&entry) != Some(&super::super::CompletionGuard::Full)
                || !retained(operand)
            {
                return Err("Required priority Full visibility/source differs".into());
            }
            let operand_path = operand
                .prediction_path
                .as_deref()
                .ok_or("Required priority Full operand has no bound source path")?;
            for _ in 0..1 + operand_path.len().max(guard_path.len()).div_ceil(64) {
                pay(work)?;
            }
            if operand_path.strip_suffix("/value") != Some(guard_path) {
                return Err("Required priority Full operand source path differs".into());
            }
            let bytes = operand_path
                .len()
                .checked_add("/recovery".len())
                .ok_or("Required priority recovery key overflow")?;
            admit(
                &mut logical,
                bytes
                    .checked_add(intersect_model::logical_size!(String, 24))
                    .ok_or("Required priority key retention overflow")?,
                work,
            )?;
            let key = format!("{operand_path}/recovery");
            choice_lookup(program.decisions.len(), key.len(), work)?;
            let occurrence = program
                .decisions
                .get(&key)
                .ok_or("Required priority Full operand lacks its actual occurrence")?;
            pay(work)?;
            if occurrence.exit.is_some() || occurrence.entries.as_slice() != [*target, *stop] {
                return Err("Required priority Full operand endpoints differ".into());
            }
        }
    }
    Ok(true)
}

impl RuntimeGuardedAdmission {
    pub(in crate::grammar::parser_prediction) fn prebind_multiway_choices(
        &mut self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
    ) -> Result<(), String> {
        self.bind_source_choices(grammar, program, work, false)
    }

    // Static classification has now published its actual flags. Resolve the
    // reserved ordinary pairs before any enclosing completion scope or marker
    // can demand their obligations. No classifier result is manufactured.
    pub(super) fn bind_classified_pairs(
        &mut self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
    ) -> Result<(), String> {
        self.bind_source_choices(grammar, program, work, true)
    }

    fn bind_source_choices(
        &mut self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
        classified_pairs: bool,
    ) -> Result<(), String> {
        if self.policy != RuntimeGuardedPolicy::InputCheckedCompletionScopes {
            return Ok(());
        }
        let result = (|| {
            pay(work)?;
            if self.failed
                || !matches!(self.owner, Owner::Unsealed)
                || grammar.profile != Profile::NativeV80
            {
                return Err(
                    "Conditional multiway choice requires fresh Native80 input-checked owner"
                        .into(),
                );
            }
            let mut hidden_owners = None;
            for (path, decision) in &program.decisions {
                pay(work)?;
                if !path.ends_with("/choice")
                    || decision.exit.is_some()
                    || decision.entries.len() < 2
                    || (classified_pairs && decision.entries.len() != 2)
                {
                    continue;
                }
                if classified_pairs {
                    choice_lookup(self.choices.len(), path.len(), work)?;
                    if self.choices.contains_key(path) {
                        continue;
                    }
                }
                let mut guarded = false;
                for &entry in &decision.entries {
                    pay(work)?;
                    let state = program
                        .states
                        .get(entry)
                        .ok_or("Conditional multiway entry is dangling")?;
                    for edge in &state.edges {
                        pay(work)?;
                        guarded |= matches!(edge, Edge::First { .. } | Edge::Full { .. });
                    }
                }
                // Unguarded ordinary alternatives retain their prior classifier.
                if !guarded {
                    continue;
                }
                // Preserve canonical early-guard commitment. This existing
                // paid predicate needs no recursive classification metadata.
                // Exclusion grants no authority: ordinary classification and
                // exact source/scope priority validation must still succeed.
                if program.completion_entry_priority(decision, work)? {
                    continue;
                }
                // Preserve the prior exact-pair admission and its independent
                // witness. This paid shape exclusion grants no runtime proof.
                if !classified_pairs
                    && decision.entries.len() == 2
                    && program.input_checked_asymmetric_guarded_shape(path, decision, None, work)?
                {
                    continue;
                }
                let proof = if classified_pairs
                    && program.input_checked_runtime_source_choice(path, grammar, work)?
                {
                    // Only the existing positive source/scope witness grants
                    // the independent pair kind, with FINAL classifier flags.
                    ChoiceProof::AsymmetricPair
                } else {
                    ChoiceProof::ConditionalMultiway
                };
                let binding = choice_binding(
                    grammar,
                    program,
                    path,
                    &proof,
                    work,
                    &mut self.logical_bytes,
                )?;
                if hidden_owners.is_none() {
                    hidden_owners = Some(multiway_hidden_owners(grammar, work)?);
                }
                let owners = hidden_owners
                    .as_ref()
                    .ok_or("Conditional hidden proof disappeared")?;
                choice_lookup(owners.len(), binding.owner.len(), work)?;
                if !owners.contains(binding.owner.as_str()) {
                    return Err(
                        "Conditional multiway owner is not reachable from the actual grammar entry"
                            .into(),
                    );
                }
                pay(work)?;
                if self
                    .records
                    .len()
                    .checked_add(self.choices.len())
                    .is_none_or(|n| n >= MAX_KEYS)
                {
                    return Err("Runtime obligation key bound exceeded".into());
                }
                choice_lookup(self.choices.len(), path.len(), work)?;
                if self.choices.contains_key(path) {
                    return Err("Duplicate conditional multiway choice".into());
                }
                pay(work)?;
                self.choices.insert(path.clone(), binding);
            }
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    pub(in crate::grammar::parser_prediction) fn conditional_choice_with_remaining(
        &self,
        path: &str,
        work: &mut usize,
    ) -> Result<bool, String> {
        if self.policy != RuntimeGuardedPolicy::InputCheckedCompletionScopes
            || self.choices.is_empty()
        {
            return Ok(false);
        }
        choice_lookup(self.choices.len(), path.len(), work)?;
        Ok(self
            .choices
            .get(path)
            .is_some_and(|b| b.proof == ChoiceProof::ConditionalMultiway))
    }

    pub(super) fn verify_multiway_scopes(
        &self,
        grammar: &Grammar,
        program: &ParserPrediction,
        work: &mut usize,
    ) -> Result<(), String> {
        for (path, binding) in &self.choices {
            pay(work)?;
            let mut logical = 0;
            let actual =
                choice_binding(grammar, program, path, &binding.proof, work, &mut logical)?;
            if !choice_bindings_equal(binding, &actual, work)? {
                return Err("Conditional choice final binding differs".into());
            }
            if binding.proof == ChoiceProof::AsymmetricPair {
                // Final publication observes the SAME actual old source/scope
                // proof, in addition to every fresh source/graph binding field.
                if !program.input_checked_runtime_source_choice(path, grammar, work)? {
                    return Err("Classified pair lost its final old source/scope witness".into());
                }
                continue;
            }
            let scope = program.verify_completion_scope_with_verification(
                binding.graph[0],
                binding.graph[1],
                work,
                super::super::CompletionVerification::InputCheckedCompletion,
            )?;
            for nested in scope.decisions.values() {
                pay(work)?;
                if nested.recursive_first
                    && !self.decision_is_runtime_with_remaining(nested.path, work)?
                    && !required_source_entry_priority(grammar, program, nested, &scope, work)?
                {
                    return Err(format!(
                        "Conditional multiway reaches a recursive Decision without its exact obligation at {}",
                        nested.path
                    ));
                }
            }
            // Generic choices use the exact ordinary completion scope above.
            // Mandatory nullable-call Full-exit proof remains only on the
            // actual required-call validation route below.
        }
        Ok(())
    }

    /// Same obligation classification with all actual map work charged. This
    /// does not itself grant runtime authority; the native owner and source are
    /// still validated by the ordinary runtime constructor and selected binder.
    pub fn decision_is_runtime_with_remaining(
        &self,
        path: &str,
        work: &mut usize,
    ) -> Result<bool, String> {
        if *work == 0 {
            return Err("Execution step budget exceeded before runtime requirement lookup".into());
        }
        *work -= 1; // Literal prior receipt lookup tariff.
        if self.records.contains_key(path) {
            return Ok(true);
        }
        if self.choices.is_empty() {
            return Ok(false);
        }
        choice_lookup(self.choices.len(), path.len(), work)?;
        Ok(self.choices.contains_key(path))
    }

    pub(in crate::grammar) fn verify_deferred_with_live_choices(
        &mut self,
        grammar: &Grammar,
        program: &mut ParserPrediction,
        path: &str,
        work: &mut usize,
    ) -> Result<(), String> {
        let result = (|| {
            if self.failed || self.policy != RuntimeGuardedPolicy::InputCheckedCompletionScopes {
                return Err("Runtime choice requires its actual input-checked companion".into());
            }
            choice_lookup(self.choices.len(), path.len(), work)?;
            if let Some(binding) = self.choices.get(path) {
                if binding.proof == ChoiceProof::AsymmetricPair {
                    if !program.input_checked_runtime_source_choice(path, grammar, work)? {
                        return Err("Reserved pair lost its actual old source/scope witness".into());
                    }
                    choice_uniform_hidden(grammar, &binding.owner, work)?;
                }
                let Owner::Reserved { marker } = self.owner else {
                    return Err("Existing choice needs its fresh reserved native owner".into());
                };
                pay(work)?;
                let requirement = program
                    .runtime_requirement
                    .as_deref()
                    .ok_or("Existing choice lacks sealed marker")?;
                if marker != requirement as *const Requirement as usize {
                    return Err("Existing choice marker differs from reserved owner".into());
                }
                let Requirement::InputCheckedCompletionScopes { decisions } = requirement else {
                    return Err("Existing choice marker has another policy".into());
                };
                choice_lookup(decisions.len(), path.len(), work)?;
                if !decisions.contains(path) {
                    return Err("Existing choice is absent from reserved marker".into());
                }
                let owner = binding.owner.as_str();
                let mut shared = Some(*work);
                let checked = self.live_choice_boundary(grammar, program, path, owner, &mut shared);
                *work = shared.ok_or("Conditional choice lost finite pool")?;
                return checked?
                    .map(|_| ())
                    .ok_or("Conditional choice lacks live boundary".into());
            }
            if !program.input_checked_runtime_source_choice(path, grammar, work)? {
                return program
                    .verify_input_checked_deferred_decision_with_remaining(path, grammar, work);
            }
            let Owner::Reserved { marker } = self.owner else {
                return Err("Runtime choice requires the fresh reserved native owner".into());
            };
            pay(work)?;
            if self
                .records
                .len()
                .checked_add(self.choices.len())
                .is_none_or(|n| n >= MAX_KEYS)
            {
                return Err("Runtime obligation key bound exceeded".into());
            }
            choice_lookup(self.choices.len(), path.len(), work)?;
            if self.choices.contains_key(path) {
                return Err("Duplicate runtime choice obligation".into());
            }
            // Prepay the two additional fixed length/count reads at final owner publication.
            pay(work)?;
            pay(work)?;
            let binding = choice_binding(
                grammar,
                program,
                path,
                &ChoiceProof::AsymmetricPair,
                work,
                &mut self.logical_bytes,
            )?;
            choice_uniform_hidden(grammar, &binding.owner, work)?;
            let requirement = program
                .runtime_requirement
                .as_deref_mut()
                .ok_or("Runtime choice lacks sealed marker")?;
            pay(work)?;
            if marker != requirement as *const Requirement as usize {
                return Err("Runtime choice marker differs from reserved owner".into());
            }
            let Requirement::InputCheckedCompletionScopes { decisions } = requirement else {
                return Err("Runtime choice marker has another policy".into());
            };
            choice_lookup(decisions.len(), path.len(), work)?;
            if decisions.contains(path) {
                return Err("Runtime choice collides with an existing obligation".into());
            }
            // Both path payloads and map-node descriptors were admitted before
            // these copies; the marker's Box/address is never replaced.
            choice_lookup(self.choices.len(), path.len(), work)?;
            choice_lookup(decisions.len(), path.len(), work)?;
            self.choices.insert(path.into(), binding);
            decisions.insert(path.into());
            Ok(())
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    // Required nullable object calls retain the real Call/return and the
    // materializing Full-exit obligation. Additional authority comes only from
    // this compiler companion: every recursive nested Decision is freshly
    // rebound to the SAME reserved source/graph obligation. No record is made.
    pub(in crate::grammar) fn verify_required_input_checked_call(
        &mut self,
        grammar: &Grammar,
        program: &ParserPrediction,
        owner: &str,
        path: &str,
        callee: &str,
        work: &mut usize,
    ) -> Result<(), String> {
        let result = (|| {
            pay(work)?;
            if self.failed
                || self.policy != RuntimeGuardedPolicy::InputCheckedCompletionScopes
                || grammar.profile != Profile::NativeV80
            {
                return Err("Required Call needs its actual input-checked companion".into());
            }
            let Owner::Reserved { marker } = self.owner else {
                return Err("Required Call needs the fresh reserved native owner".into());
            };
            pay(work)?;
            let requirement = program
                .runtime_requirement
                .as_deref()
                .ok_or("Required Call lacks the sealed runtime marker")?;
            if marker != requirement as *const Requirement as usize {
                return Err("Required Call has a foreign runtime marker".into());
            }
            let Requirement::InputCheckedCompletionScopes { decisions } = requirement else {
                return Err("Required Call runtime marker has another policy".into());
            };
            self.validate_choice_keys(decisions, work)?;
            let (actual_owner, rule, expression) =
                super::super::super::nullable_repetition::prefix_source_occurrence_with_work(
                    grammar, path, work,
                )?
                .ok_or("Required Call has no canonical source occurrence")?;
            for _ in 0..(2
                + owner.len().max(actual_owner.len()).div_ceil(64)
                + callee.len().div_ceil(64)
                + path.len().div_ceil(64))
            {
                pay(work)?;
            }
            if actual_owner != owner
                || expression.cardinality != '1'
                || expression.first_set_predicated
                || expression.prediction_path.as_deref() != Some(path)
                || !matches!(&expression.kind, Kind::Call(actual) if actual == callee)
            {
                return Err("Required Call source owner/callee differs".into());
            }
            choice_lookup(grammar.rules.len(), callee.len(), work)?;
            let target = grammar
                .rules
                .get(callee)
                .ok_or("Required Call source callee is absent")?;
            pay(work)?;
            if target.fragment || target.datatype.is_some() {
                return Err("Required Call source callee is not a nonfragment object rule".into());
            }
            choice_lookup(grammar.declaration_sources.len(), owner.len(), work)?;
            let source = grammar
                .declaration_sources
                .get(owner)
                .ok_or("Required Call has no original owner source")?;
            pay(work)?;
            if source.start >= source.end
                || expression.original_range.as_ref().is_none_or(|range| {
                    range.start >= range.end || range.start < source.start || range.end > source.end
                })
                || rule.body.original_range.as_ref().is_none_or(|range| {
                    range.start >= range.end || range.start < source.start || range.end > source.end
                })
            {
                return Err("Required Call has a foreign original source interval".into());
            }
            choice_uniform_hidden(grammar, owner, work)?;
            let (start, end) = program.required_object_call_binding(path, callee, work)?;
            let scope = program.verify_completion_scope_with_verification(
                start,
                end,
                work,
                super::super::CompletionVerification::InputCheckedCompletion,
            )?;
            for bound in scope.decisions.values() {
                pay(work)?;
                if !bound.recursive_first {
                    continue;
                }
                if !self.decision_is_runtime_with_remaining(bound.path, work)? {
                    // Recursive metadata alone does not mean an asymmetric
                    // per-input obligation: Legacy/Strict already qualify an
                    // actual guard on every nonlast entry. Rebind that SAME
                    // canonical source/graph priority, never scope success alone.
                    if required_source_entry_priority(grammar, program, bound, &scope, work)? {
                        continue;
                    }
                    return Err(format!(
                        "Required Call reaches a recursive Decision without its runtime obligation at {}",
                        bound.path,
                    ));
                }
                for _ in 0..1 + bound.path.len().div_ceil(64) {
                    pay(work)?;
                }
                if bound.path.ends_with("/repeat") {
                    self.validate_decision_with_remaining(grammar, program, bound.path, work)?;
                } else if bound.path.ends_with("/choice") {
                    let nested_owner = bound
                        .path
                        .split('/')
                        .nth(1)
                        .ok_or("Required Call nested choice has no exact owner")?;
                    // Adapt only the existing budget representation. Success
                    // and failure propagate the SAME consumed remainder.
                    let mut shared = Some(*work);
                    let checked = self.live_choice_boundary(
                        grammar,
                        program,
                        bound.path,
                        nested_owner,
                        &mut shared,
                    );
                    *work = shared.ok_or("Required Call lost its finite shared work")?;
                    if checked?.is_none() {
                        return Err(
                            "Required Call nested choice has no bound runtime obligation".into(),
                        );
                    }
                } else {
                    return Err(
                        "Required Call recursive obligation has an unsupported source shape".into(),
                    );
                }
            }
            program.verify_required_full_exit(&scope, work)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn validate_choice_keys(
        &self,
        decisions: &std::collections::BTreeSet<String>,
        work: &mut usize,
    ) -> Result<(), String> {
        pay(work)?;
        if decisions.len()
            != self
                .records
                .len()
                .checked_add(self.choices.len())
                .ok_or("Runtime choice count overflow")?
        {
            return Err("Runtime requirement differs from compiler companion".into());
        }
        for path in decisions {
            choice_lookup(self.records.len(), path.len(), work)?;
            if self.records.contains_key(path) {
                continue;
            }
            choice_lookup(self.choices.len(), path.len(), work)?;
            if !self.choices.contains_key(path) {
                return Err("Runtime requirement differs from compiler companion".into());
            }
        }
        Ok(())
    }

    pub(in crate::grammar::parser_prediction) fn live_choice_boundary(
        &self,
        grammar: &Grammar,
        program: &ParserPrediction,
        path: &str,
        owner: &str,
        work: &mut Option<usize>,
    ) -> Result<Option<usize>, String> {
        // No new lookup/fee on every existing default/Strict/repeat-only route.
        if self.choices.is_empty() {
            return Ok(None);
        }
        let remaining = work
            .as_mut()
            .ok_or("Runtime choice needs the original finite work")?;
        choice_lookup(self.choices.len(), path.len(), remaining)?;
        let Some(record) = self.choices.get(path) else {
            return Ok(None);
        };
        if self.failed || self.policy != RuntimeGuardedPolicy::InputCheckedCompletionScopes {
            return Err("Runtime choice companion is unavailable".into());
        }
        // Immutable grammar/program/marker addresses are checked by the ordinary
        // LiveCaller constructor. The selected occurrence is freshly rebound.
        for _ in 0..(1 + record.owner.len().max(owner.len()).div_ceil(64)) {
            pay(remaining)?;
        }
        if record.owner != owner {
            return Err("Runtime choice has a foreign live owner".into());
        }
        let mut logical = 0;
        let actual = choice_binding(
            grammar,
            program,
            path,
            &record.proof,
            remaining,
            &mut logical,
        )?;
        if !choice_bindings_equal(record, &actual, remaining)? {
            return Err("Runtime choice source/graph binding differs".into());
        }
        Ok(Some(record.graph[1]))
    }
}

#[cfg(test)]
mod hidden_closure_tests {
    use super::*;
    #[test]
    fn input_checked_live_choice_hidden_worklist_separates_call_reachability_from_expr_depth()
    -> Result<(), String> {
        use intersect_model::definitions::bundle::DefinitionSource;
        use intersect_model::definitions::bundle::SourceKind;
        use intersect_model::definitions::bundle::SourceSpan;
        use crate::grammar::backend_test_support::inspect_ecore_schema;
        let schema = inspect_ecore_schema(
            r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="h" nsURI="urn:hidden-worklist"><eClassifiers x:type="e:EClass" name="Root"/></e:EPackage>"#,
        ).map_err(|e| format!("{e:?}"))?;
        let parsed = |override_after128: bool| -> Result<Grammar, String> {
            let mut text = String::from(
                "grammar hiddenWorklist hidden() import \"urn:hidden-worklist\" as t Root returns t::Root: {t::Root} R0;",
            );
            // Real parsed, individually shallow bodies; this is source graph
            // reachability, not160 simultaneous runtime calls or Expr nesting.
            for index in 0..160 {
                let hidden = if override_after128 && index == 140 {
                    " hidden(WS)"
                } else {
                    ""
                };
                let body = if index == 159 {
                    "'end' R0?".into()
                } else {
                    format!("'r' R{}", index + 1)
                };
                text.push_str(&format!(
                    " fragment R{index} returns t::Root{hidden}: {body};"
                ));
            }
            text.push_str(" terminal WS: ' ';");
            let source = DefinitionSource::new(
                "memory:live-choice/long-hidden-closure.xtext",
                SourceKind::Xtext,
                text,
            );
            let (mut grammar, _, spans, _) = crate::grammar::parse_declarations(
                &source.text,
                &schema,
                Profile::NativeV80,
                false,
                None,
                256, // Declaration count: Root +160 fragments +WS; not a depth limit.
                std::rc::Rc::default(),
            )
            .map_err(|e| format!("{e:?}"))?;
            grammar.declaration_sources = spans
                .into_iter()
                .map(|(name, range)| {
                    (
                        name,
                        SourceSpan {
                            source: source.id.clone(),
                            start: range.start,
                            end: range.end,
                        },
                    )
                })
                .collect();
            Ok(grammar)
        };
        let grammar = parsed(false)?;
        assert_eq!(grammar.rules.len(), 161);
        let mut remaining = 1_000_000;
        choice_uniform_hidden(&grammar, "R150", &mut remaining)?;
        let used = 1_000_000 - remaining;
        assert!(used > 160);
        for initial in [0, used - 1, used] {
            let mut remaining = initial;
            let result = choice_uniform_hidden(&grammar, "R150", &mut remaining);
            assert_eq!(remaining, 0);
            if initial == used {
                result?;
            } else {
                assert!(
                    result
                        .expect_err("Short proof cannot publish authority")
                        .contains("budget")
                );
            }
        }
        let grammar = parsed(true)?;
        let mut remaining = 1_000_000;
        let error = choice_uniform_hidden(&grammar, "R150", &mut remaining)
            .expect_err("Future reachable hidden override remains relevant after128 source calls");
        assert!(
            error.contains("reachable rule switches hidden policy: R140"),
            "{error}"
        );
        assert!(!error.contains("depth"), "{error}");
        assert!(remaining > 0);
        // Public syntax has its own earlier nesting bound. A hostile retained
        // Expr is constructed here only to prove this private traversal still
        // refuses true structural nesting above128; no grant is manufactured.
        let mut grammar = parsed(false)?;
        let rule = grammar
            .rules
            .get_mut("R0")
            .ok_or("Missing real parsed R0")?;
        for _ in 0..129 {
            rule.body = Expr {
                original_range: rule.body.original_range.clone(),
                first_set_predicated: false,
                prediction_path: None,
                cardinality: '1',
                kind: Kind::Sequence(vec![rule.body.clone()]),
            };
        }
        let mut remaining = 1_000_000;
        let error = choice_uniform_hidden(&grammar, "R150", &mut remaining)
            .expect_err("Actual Expr nesting still has the128 boundary");
        assert_eq!(error, "Runtime choice hidden expression depth exceeded");
        assert!(remaining > 0);
        Ok(())
    }
    #[test]
    fn input_checked_live_choice_implicit_id_reference_hidden_closure() -> Result<(), String> {
        use intersect_model::definitions::bundle::DefinitionSource;
        use intersect_model::definitions::bundle::SourceKind;
        use intersect_model::definitions::bundle::SourceSpan;
        use crate::grammar::backend_test_support::inspect_ecore_schema;
        let schema = inspect_ecore_schema(
            r##"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="h" nsURI="urn:hidden-closure"><eClassifiers x:type="e:EClass" name="Root"><eStructuralFeatures x:type="e:EReference" name="ref" eType="#//Root"/></eClassifiers></e:EPackage>"##,
        )
        .map_err(|e| format!("{e:?}"))?;
        let parsed = |reference: bool| -> Result<Grammar, String> {
            let tail = if reference { "ref=[t::Root]" } else { "'tail'" };
            let source = DefinitionSource::new(
                "memory:live-choice/implicit-id.xtext",
                SourceKind::Xtext,
                format!(
                    "grammar hiddenClosure hidden() import \"urn:hidden-closure\" as t Root returns t::Root: {{t::Root}} Body Tail; fragment Body returns t::Root: 'body'; fragment Tail returns t::Root: {tail}; fragment Unused returns t::Root: ref=[t::Root]; fragment ID returns t::Root hidden(WS): 'id'; terminal WS: ' ';"
                ),
            );
            let (mut grammar, _, spans, _) = crate::grammar::parse_declarations(
                &source.text,
                &schema,
                Profile::NativeV80,
                false,
                None,
                128,
                std::rc::Rc::default(),
            )
            .map_err(|e| format!("{e:?}"))?;
            grammar.declaration_sources = spans
                .into_iter()
                .map(|(name, range)| {
                    (
                        name,
                        SourceSpan {
                            source: source.id.clone(),
                            start: range.start,
                            end: range.end,
                        },
                    )
                })
                .collect();
            Ok(grammar)
        };
        let grammar = parsed(true)?;
        assert!(matches!(
            &grammar.rules["Tail"].body.kind,
            Kind::Assign { value, .. } if matches!(value.kind, Kind::Reference(_))
        ));
        let mut work = 100_000;
        let error = choice_uniform_hidden(&grammar, "Body", &mut work)
            .expect_err("Canonical Reference invokes the reachable ID parser rule");
        assert!(
            error.contains("Runtime choice reachable rule switches hidden policy: ID"),
            "{error}"
        );
        assert!(!error.contains("budget"), "{error}");
        assert!(work > 0);
        // Same overridden ID and same real Reference declaration remain in the
        // parsed grammar, but no entry-reachable reference calls that rule.
        let grammar = parsed(false)?;
        assert!(matches!(
            &grammar.rules["Unused"].body.kind,
            Kind::Assign { value, .. } if matches!(value.kind, Kind::Reference(_))
        ));
        assert!(grammar.rules["ID"].hidden.is_some());
        let mut work = 100_000;
        choice_uniform_hidden(&grammar, "Body", &mut work)?;
        let used = 100_000 - work;
        assert!(used > 0);
        for limit in [0, used - 1, used] {
            let mut remaining = limit;
            let result = choice_uniform_hidden(&grammar, "Body", &mut remaining);
            assert_eq!(remaining, 0);
            if limit == used {
                result?;
            } else {
                assert!(
                    result
                        .expect_err("Short hidden proof must fail")
                        .contains("budget")
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod required_priority_controls {
    use super::*;

    #[test]
    fn input_checked_required_entry_priority_missing_asymmetric_obligation_stays_hard()
    -> Result<(), String> {
        // Genuine public native compilation and the same canonical reserved
        // stages run before this deliberately destructive private corruption.
        // Removing authority never constructs or serializes a replacement.
        let (grammar, mut program, mut admission, required, _) =
            crate::grammar::parser_prediction::tests::required_call_reserved_stage()?;
        let path = admission
            .choices
            .keys()
            .find(|path| path.starts_with("rule/Item/"))
            .cloned()
            .ok_or("No genuine asymmetric runtime choice")?;
        let decision = program.decisions.get(&path).ok_or("Missing real choice")?;
        assert!(program.recursive_decisions.contains(&path));
        assert!(!program.completion_entry_priority(decision, &mut 100_000)?);
        assert!(admission.choices.remove(&path).is_some());
        let Some(Requirement::InputCheckedCompletionScopes { decisions }) =
            program.runtime_requirement.as_deref_mut()
        else {
            return Err("Missing real native marker".into());
        };
        assert!(decisions.remove(&path));
        let call = required
            .iter()
            .find(|call| call.owner == "Envelope" && call.callee == "Holder")
            .ok_or("Missing actual required Call")?;
        let mut remaining = 10_000_000;
        let error = admission
            .verify_required_input_checked_call(
                &grammar,
                &program,
                &call.owner,
                &call.path,
                &call.callee,
                &mut remaining,
            )
            .expect_err("Asymmetric recursion still requires its exact runtime obligation");
        assert_eq!(
            error,
            format!(
                "Required Call reaches a recursive Decision without its runtime obligation at {path}"
            )
        );
        assert!(remaining > 0);
        let retry = admission
            .verify_required_input_checked_call(
                &grammar,
                &program,
                &call.owner,
                &call.path,
                &call.callee,
                &mut remaining,
            )
            .expect_err("Caught missing authority must poison the same companion");
        assert!(retry.contains("actual input-checked companion"));
        Ok(())
    }
}
