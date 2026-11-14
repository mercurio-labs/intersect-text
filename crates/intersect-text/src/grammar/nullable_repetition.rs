//! Borrowed source qualification for the explicit nullable whole-Full slice.
//! Retained source metadata is never serialized executable authority.
use super::{Expr, Grammar, Kind, Profile, Rule};

pub struct QualifiedNullableFullRepeat<'g> {
    expression: &'g Expr,
    callee: &'g str,
    keyword: &'g str,
    call: &'g Expr,
    owner: &'g str,
}
impl<'g> QualifiedNullableFullRepeat<'g> {
    pub fn expression(&self) -> &'g Expr {
        self.expression
    }
    pub fn callee(&self) -> &'g str {
        self.callee
    }
    pub fn keyword(&self) -> &'g str {
        self.keyword
    }
    pub fn call_expression(&self) -> &'g Expr {
        self.call
    }
    pub fn owner(&self) -> &'g str {
        self.owner
    }
}
pub struct QualifiedNullableKeywordOptional<'g> {
    expression: &'g Expr,
    owner: &'g str,
    keyword: &'g str,
}
/// A borrowed source capability, never serialized authority or retained state.
pub enum QualifiedNullableDecision<'g> {
    FullRepeat(QualifiedNullableFullRepeat<'g>),
    KeywordOptional(QualifiedNullableKeywordOptional<'g>),
}
enum SourceWork<'a> {
    Limited(&'a mut usize),
    Unbounded,
}
impl SourceWork<'_> {
    fn charge(&mut self) -> Result<(), String> {
        match self {
            Self::Limited(work) => charge(work),
            Self::Unbounded => Ok(()),
        }
    }
}
impl<'g> QualifiedNullableKeywordOptional<'g> {
    pub fn expression(&self) -> &'g Expr {
        self.expression
    }
    pub fn owner(&self) -> &'g str {
        self.owner
    }
    pub fn keyword(&self) -> &'g str {
        self.keyword
    }
}
#[derive(Clone, Copy)]
struct Occurrence<'g> {
    owner: &'g str,
    rule: &'g Rule,
    expression: &'g Expr,
}
fn charge(work: &mut usize) -> Result<(), String> {
    if *work == 0 {
        return Err("Grammar prediction budget exceeded".into());
    }
    *work -= 1;
    Ok(())
}
fn retained(expr: &Expr, source: &intersect_model::definitions::bundle::SourceSpan) -> bool {
    expr.original_range.as_ref().is_some_and(|range| {
        source.start <= range.start && range.start < range.end && range.end <= source.end
    })
}
fn occurrence<'g>(
    grammar: &'g Grammar,
    path: &str,
    work: &mut SourceWork<'_>,
) -> Result<Option<Occurrence<'g>>, String> {
    if !matches!(
        grammar.profile,
        Profile::NativeV77
            | Profile::NativeV78
            | Profile::NativeV79
            | Profile::NativeV80
            | Profile::NativeV81
            | Profile::NativeV82
    ) || !path.ends_with("/repeat")
    {
        return Ok(None);
    }
    work.charge()?;
    let mut parts = path
        .strip_suffix("/repeat")
        .ok_or("Missing nullable repeat suffix")?
        .split('/');
    if parts.next() != Some("rule") {
        return Err("Nullable decision needs an actual source rule owner".into());
    }
    let owner = parts
        .next()
        .ok_or("Nullable decision has no actual owner")?;
    work.charge()?;
    let (name, rule) = grammar
        .rules
        .get_key_value(owner)
        .ok_or("Unknown nullable decision owner")?;
    let mut expr = &rule.body;
    for part in parts {
        work.charge()?;
        expr = match (part, &expr.kind) {
            (
                "value",
                Kind::Assign { value, .. }
                | Kind::BooleanAssign { value, .. }
                | Kind::SyntacticPredicate(value),
            ) => value,
            (_, Kind::Sequence(items)) if part.starts_with('s') => items
                .get(
                    part[1..]
                        .parse::<usize>()
                        .map_err(|_| "Invalid nullable sequence index")?,
                )
                .ok_or("Nullable sequence index escapes source")?,
            (_, Kind::Alternative(items)) if part.starts_with('a') => items
                .get(
                    part[1..]
                        .parse::<usize>()
                        .map_err(|_| "Invalid nullable alternative index")?,
                )
                .ok_or("Nullable alternative index escapes source")?,
            _ => return Err("Nullable decision path differs from its canonical source".into()),
        };
    }
    Ok(Some(Occurrence {
        owner: name,
        rule,
        expression: expr,
    }))
}
// Diagnostic reuse of the SAME charged canonical source-path resolver.
// The result is not nullable admission or a parser capability.
pub(super) fn prefix_source_occurrence_with_work<'g>(
    grammar: &'g Grammar,
    path: &str,
    work: &mut usize,
) -> Result<Option<(&'g str, &'g Rule, &'g Expr)>, String> {
    charge(work)?;
    let extended;
    let path = if path.ends_with("/repeat") {
        path
    } else {
        let bytes = path
            .len()
            .checked_add("/repeat".len())
            .ok_or("Source prefix diagnostic path size overflow")?;
        super::first_sets::prefix_capture::reserve_copy_bytes(bytes, work)?;
        extended = format!("{path}/repeat");
        &extended
    };
    let value = occurrence(grammar, path, &mut SourceWork::Limited(work))?;
    Ok(value.map(|value| (value.owner, value.rule, value.expression)))
}

fn contains(parent: &Expr, child: &Expr) -> bool {
    match (&parent.original_range, &child.original_range) {
        (Some(parent), Some(child)) => parent.start <= child.start && child.end <= parent.end,
        _ => false,
    }
}
fn candidate(expr: &Expr) -> bool {
    expr.cardinality == '*'
        && !expr.first_set_predicated
        && (matches!(&expr.kind,Kind::SyntacticPredicate(value)
            if value.cardinality=='1' && !value.first_set_predicated && matches!(&value.kind,Kind::Assign{many:true,value,..}
                if value.cardinality=='1' && !value.first_set_predicated && matches!(value.kind,Kind::Call(_))))
            || matches!(&expr.kind, Kind::Sequence(items) if items.len()==1))
}
// MAY filter only; every retained path still uses the original qualifier.
// All repeats in a two-item owner stay: optional_keyword checks source before
// comparing the actual occurrence pointer. Full wrapper errors stay too.
pub(super) fn inventory_may_qualify(rule: &Rule, expr: &Expr) -> bool {
    candidate(expr) || matches!(&rule.body.kind, Kind::Sequence(items) if items.len() == 2)
}
fn whole_full<'g>(
    repeated: &'g Expr,
    source: &intersect_model::definitions::bundle::SourceSpan,
    work: &mut SourceWork<'_>,
) -> Result<Option<&'g Expr>, String> {
    if repeated.cardinality != '*' || repeated.first_set_predicated {
        return Ok(None);
    }
    let mut full = repeated;
    for depth in 0..=64 {
        if !retained(full, source) {
            return Err("Nullable Full group has unqualified original source boundaries".into());
        }
        match &full.kind {
            Kind::Sequence(items) if items.len() == 1 => {
                // The previous prepaid inspection established one exact group.
                // Pay before following/validating its actual sole child.
                work.charge()?;
                if depth == 64 {
                    return Err("Grammar syntax recursion budget exceeded".into());
                }
                let child = &items[0];
                if child.cardinality != '1' || child.first_set_predicated || !contains(full, child)
                {
                    return Ok(None);
                }
                full = child;
            }
            Kind::SyntacticPredicate(value)
                if value.cardinality == '1'
                    && !value.first_set_predicated
                    && matches!(&value.kind,Kind::Assign{many:true,value,..}
                        if value.cardinality=='1' && !value.first_set_predicated && matches!(value.kind,Kind::Call(_))) =>
            {
                return Ok(Some(full));
            }
            _ => return Ok(None),
        }
    }
    Err("Grammar syntax recursion budget exceeded".into())
}
fn optional_keyword<'g>(
    grammar: &'g Grammar,
    owner: &str,
    rule: &'g Rule,
    work: &mut SourceWork<'_>,
) -> Result<Option<(&'g Expr, &'g str)>, String> {
    work.charge()?;
    if rule.fragment
        || rule.wildcard
        || rule.datatype.is_some()
        || rule.inferred_return
        || rule.hidden.is_some()
        || rule.body.cardinality != '1'
        || rule.body.first_set_predicated
    {
        return Ok(None);
    }
    let Kind::Sequence(items) = &rule.body.kind else {
        return Ok(None);
    };
    let [action, optional] = items.as_slice() else {
        return Ok(None);
    };
    work.charge()?;
    if action.cardinality != '1'
        || action.first_set_predicated
        || !matches!(&action.kind,Kind::Action(class) if class==&rule.class)
        || optional.cardinality != '?'
        || optional.first_set_predicated
    {
        return Ok(None);
    }
    let Kind::Assign {
        many: false, value, ..
    } = &optional.kind
    else {
        return Ok(None);
    };
    work.charge()?;
    let Kind::Literal(keyword) = &value.kind else {
        return Ok(None);
    };
    if value.cardinality != '1' || value.first_set_predicated || keyword.is_empty() {
        return Ok(None);
    }
    work.charge()?;
    let source = grammar
        .declaration_sources
        .get(owner)
        .ok_or("Nullable keyword rule has no retained source identity")?;
    if !retained(&rule.body, source)
        || !retained(action, source)
        || !retained(optional, source)
        || !retained(value, source)
    {
        return Err("Nullable keyword rule source boundaries are unqualified".into());
    }
    Ok(Some((optional, keyword)))
}
impl Grammar {
    /// Explicit new-profile source capability over a canonical authored occurrence.
    /// Checked hosts still compile/replay original inputs; no serialized proof is trusted.
    pub fn qualified_nullable_full_repeat_with_work<'g>(
        &'g self,
        path: &str,
        work: &mut usize,
    ) -> Result<Option<QualifiedNullableFullRepeat<'g>>, String> {
        self.qualified_nullable_full_repeat(path, &mut SourceWork::Limited(work))
    }
    fn qualified_nullable_full_repeat<'g>(
        &'g self,
        path: &str,
        work: &mut SourceWork<'_>,
    ) -> Result<Option<QualifiedNullableFullRepeat<'g>>, String> {
        let Some(site) = occurrence(self, path, work)? else {
            return Ok(None);
        };
        self.qualified_nullable_full_repeat_at_occurrence(site, work)
    }
    fn qualified_nullable_full_repeat_at_occurrence<'g>(
        &'g self,
        site: Occurrence<'g>,
        work: &mut SourceWork<'_>,
    ) -> Result<Option<QualifiedNullableFullRepeat<'g>>, String> {
        work.charge()?;
        if !candidate(site.expression) {
            return Ok(None);
        }
        // Same fixed owner lookup fee as the old direct-shape proof. Retaining
        // the canonical owner first lets every group validate real source ranges.
        work.charge()?;
        let source = self
            .declaration_sources
            .get(site.owner)
            .ok_or("Nullable repeat owner has no retained source identity")?;
        let Some(full) = whole_full(site.expression, source, work)? else {
            return Ok(None);
        };
        let Kind::SyntacticPredicate(assignment) = &full.kind else {
            return Err("Nullable repeat lost its whole Full operand".into());
        };
        let Kind::Assign { value: call, .. } = &assignment.kind else {
            return Err("Nullable repeat lost its actual assignment".into());
        };
        let Kind::Call(callee) = &call.kind else {
            return Err("Nullable repeat lost its actual object call".into());
        };
        work.charge()?;
        let target = self
            .rules
            .get(callee)
            .ok_or("Unknown nullable repeat callee")?;
        let Some((_, keyword)) = optional_keyword(self, callee, target, work)? else {
            return Ok(None);
        };
        if site.rule.hidden.is_some()
            || !retained(site.expression, source)
            || !retained(full, source)
            || !retained(assignment, source)
            || !retained(call, source)
            || !contains(full, assignment)
            || !contains(assignment, call)
        {
            return Err(
                "Nullable repeat owner/operand source policy or boundaries are unqualified".into(),
            );
        }
        Ok(Some(QualifiedNullableFullRepeat {
            expression: site.expression,
            callee,
            keyword,
            call,
            owner: site.owner,
        }))
    }
    /// Compose the exact inner required Call with its canonical whole-Full
    /// loop. Nonmatching call contexts retain the ordinary required-call proof.
    pub(super) fn qualified_required_nullable_full_call_with_work<'g>(
        &'g self,
        owner: &str,
        path: &str,
        callee: &str,
        work: &mut usize,
    ) -> Result<Option<(QualifiedNullableFullRepeat<'g>, String)>, String> {
        if !matches!(
            self.profile,
            Profile::NativeV77
                | Profile::NativeV78
                | Profile::NativeV79
                | Profile::NativeV80
                | Profile::NativeV81
                | Profile::NativeV82
        ) {
            return Ok(None);
        }
        charge(work)?;
        let Some(mut loop_path) = path.strip_suffix("/value/value") else {
            return Ok(None);
        };
        let (proof, repeat_path) = loop {
            // Charge each retained path before allocation. Only actual /s0
            // ancestry may lead to a group; the same canonical qualifier must
            // prove every intervening group contains exactly one whole Full.
            charge(work)?;
            let repeat_path = format!("{loop_path}/repeat");
            if let Some(proof) =
                self.qualified_nullable_full_repeat_with_work(&repeat_path, work)?
            {
                break (proof, repeat_path);
            }
            let Some(parent) = loop_path.strip_suffix("/s0") else {
                return Ok(None);
            };
            loop_path = parent;
        };
        charge(work)?;
        if proof.owner() != owner || proof.callee() != callee {
            return Err(
                "Required nullable Full call differs from its canonical owner/callee".into(),
            );
        }
        // Re-resolve the original Call occurrence rather than trusting suffix
        // spelling alone. The ordinary occurrence walker only accepts a repeat
        // suffix, which is removed before canonical AST traversal.
        charge(work)?;
        let call_path = format!("{path}/repeat");
        let call_site = occurrence(self, &call_path, &mut SourceWork::Limited(work))?
            .ok_or("Missing canonical required nullable Call occurrence")?;
        charge(work)?;
        if !std::ptr::eq(call_site.expression, proof.call_expression()) {
            return Err("Required nullable Call source differs from its whole-Full operand".into());
        }
        Ok(Some((proof, repeat_path)))
    }
    /// The same qualified callee's local keyword optional is greedy at its exact
    /// rule boundary. This explicit new-profile semantics applies to its other
    /// callers too; their whole-language equivalence is not inferred from it.
    pub fn qualified_nullable_keyword_optional_with_work<'g>(
        &'g self,
        path: &str,
        work: &mut usize,
    ) -> Result<Option<QualifiedNullableKeywordOptional<'g>>, String> {
        self.qualified_nullable_keyword_optional(path, &mut SourceWork::Limited(work))
    }
    fn qualified_nullable_keyword_optional<'g>(
        &'g self,
        path: &str,
        work: &mut SourceWork<'_>,
    ) -> Result<Option<QualifiedNullableKeywordOptional<'g>>, String> {
        let Some(site) = occurrence(self, path, work)? else {
            return Ok(None);
        };
        self.qualified_nullable_keyword_optional_at_occurrence(site, work)
    }
    fn qualified_nullable_keyword_optional_at_occurrence<'g>(
        &'g self,
        site: Occurrence<'g>,
        work: &mut SourceWork<'_>,
    ) -> Result<Option<QualifiedNullableKeywordOptional<'g>>, String> {
        let Some((optional, keyword)) = optional_keyword(self, site.owner, site.rule, work)? else {
            return Ok(None);
        };
        work.charge()?;
        if !std::ptr::eq(optional, site.expression) {
            return Ok(None);
        }
        Ok(Some(QualifiedNullableKeywordOptional {
            expression: optional,
            owner: site.owner,
            keyword,
        }))
    }
    /// One canonical source lookup for the explicit V79 compiler path.
    /// Graph binding and prediction still use the original shared workers.
    pub(super) fn qualified_nullable_decision_with_finite_work<'g>(
        &'g self,
        path: &str,
        remaining: &mut usize,
    ) -> Result<Option<QualifiedNullableDecision<'g>>, String> {
        if !matches!(
            self.profile,
            Profile::NativeV79 | Profile::NativeV80 | Profile::NativeV81 | Profile::NativeV82
        ) {
            return Err("Fused source qualification requires its explicit profile".into());
        }
        self.qualified_nullable_decision_at_occurrence(path, &mut SourceWork::Limited(remaining))
    }
    fn qualified_nullable_decision_at_occurrence<'g>(
        &'g self,
        path: &str,
        work: &mut SourceWork<'_>,
    ) -> Result<Option<QualifiedNullableDecision<'g>>, String> {
        let Some(site) = occurrence(self, path, work)? else {
            return Ok(None);
        };
        // Full must remain first: its real source/shape failure is terminal.
        // These shapes cannot both match: '*' whole-Full versus '?' keyword.
        if let Some(proof) = self.qualified_nullable_full_repeat_at_occurrence(site, work)? {
            return Ok(Some(QualifiedNullableDecision::FullRepeat(proof)));
        }
        Ok(self
            .qualified_nullable_keyword_optional_at_occurrence(site, work)?
            .map(QualifiedNullableDecision::KeywordOptional))
    }
    /// The runtime's existing `None` policy removes its step ceiling. It does
    /// not fabricate another finite allowance. Actual new replay capabilities
    /// still enforce their finite shared ledger before reading source tokens.
    pub fn qualified_nullable_decision_with_work<'g>(
        &'g self,
        path: &str,
        remaining: &mut Option<usize>,
    ) -> Result<Option<QualifiedNullableDecision<'g>>, String> {
        let mut work = match remaining.as_mut() {
            Some(work) => SourceWork::Limited(work),
            None => SourceWork::Unbounded,
        };
        if matches!(
            self.profile,
            Profile::NativeV79 | Profile::NativeV80 | Profile::NativeV81 | Profile::NativeV82
        ) {
            return self.qualified_nullable_decision_at_occurrence(path, &mut work);
        }
        if let Some(proof) = self.qualified_nullable_full_repeat(path, &mut work)? {
            return Ok(Some(QualifiedNullableDecision::FullRepeat(proof)));
        }
        Ok(self
            .qualified_nullable_keyword_optional(path, &mut work)?
            .map(QualifiedNullableDecision::KeywordOptional))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use intersect_model::definitions::bundle::DefinitionSource;
    use intersect_model::definitions::bundle::SourceKind;
    use intersect_model::definitions::bundle::SourceSpan;
    use std::collections::BTreeMap;
    fn node(kind: Kind, cardinality: char, start: usize, end: usize) -> Expr {
        Expr {
            kind,
            cardinality,
            original_range: Some(start..end),
            prediction_path: None,
            first_set_predicated: false,
        }
    }
    fn fixture() -> Grammar {
        let call = node(Kind::Call("Target".into()), '1', 2, 3);
        let assignment = node(
            Kind::Assign {
                feature: "children".into(),
                many: true,
                value: Box::new(call),
            },
            '1',
            1,
            3,
        );
        let repeat = node(Kind::SyntacticPredicate(Box::new(assignment)), '*', 1, 4);
        let root = node(
            Kind::Sequence(vec![node(Kind::Action("Root".into()), '1', 0, 1), repeat]),
            '1',
            0,
            4,
        );
        let optional = node(
            Kind::Assign {
                feature: "tag".into(),
                many: false,
                value: Box::new(node(Kind::Literal("x".into()), '1', 7, 8)),
            },
            '?',
            6,
            8,
        );
        let target = node(
            Kind::Sequence(vec![
                node(Kind::Action("Target".into()), '1', 4, 6),
                optional,
            ]),
            '1',
            4,
            8,
        );
        let rule = |class: &str, body| Rule {
            hidden: None,
            enum_literals: BTreeMap::new(),
            inferred_return: false,
            override_declared: false,
            datatype: None,
            nominal_return: None,
            fragment: false,
            wildcard: false,
            class: class.into(),
            body,
        };
        let source = DefinitionSource::new(
            "memory:structural-source-check.xtext",
            SourceKind::Xtext,
            "RRRRTTTT",
        );
        Grammar {
            call_depth: None,
            analysis_work_budget: Default::default(),
            profile: Profile::NativeV77,
            name: "SourceChecks".into(),
            entry: "Root".into(),
            rules: [
                ("Root".into(), rule("Root", root)),
                ("Target".into(), rule("Target", target)),
            ]
            .into(),
            terminals: BTreeMap::new(),
            hidden: None,
            lexical_prediction: None,
            parser_prediction: None,
            declaration_sources: [
                (
                    "Root".into(),
                    SourceSpan {
                        source: source.id.clone(),
                        start: 0,
                        end: 4,
                    },
                ),
                (
                    "Target".into(),
                    SourceSpan {
                        source: source.id,
                        start: 4,
                        end: 8,
                    },
                ),
            ]
            .into(),
            terminal_order: vec![],
            shadowed_declarations: BTreeMap::new(),
            parameterized_rules: Default::default(),
            final_rules: Default::default(),
            computed_settings: vec![],
        }
    }
    fn wrap_repeat(grammar: &mut Grammar) {
        let Kind::Sequence(items) = &mut grammar.rules.get_mut("Root").unwrap().body.kind else {
            panic!("Expected fixture sequence")
        };
        let mut inner = items[1].clone();
        inner.cardinality = '1';
        items[1] = node(Kind::Sequence(vec![inner]), '*', 1, 4);
    }
    #[test]
    fn canonical_nullable_source_qualification_charges_exact_lookup_and_no_old_profile_work() {
        let grammar = fixture();
        let mut work = 10000;
        let proof = grammar
            .qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut work)
            .unwrap()
            .unwrap();
        assert_eq!(proof.callee(), "Target");
        let Kind::Sequence(items) = &grammar.rules["Root"].body.kind else {
            panic!("Expected fixture sequence")
        };
        assert!(std::ptr::eq(proof.expression(), &items[1]));
        let used = 10000 - work;
        assert_eq!(used, 10);
        for amount in 0..=used {
            let mut work = amount;
            let result =
                grammar.qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut work);
            if amount == used {
                assert!(result.unwrap().is_some());
            } else {
                assert!(result.err().unwrap().contains("budget"));
            }
            assert_eq!(work, 0);
        }
        let mut work = 10000;
        let optional = grammar
            .qualified_nullable_keyword_optional_with_work("rule/Target/s1/repeat", &mut work)
            .unwrap()
            .unwrap();
        assert_eq!((optional.owner(), optional.keyword()), ("Target", "x"));
        assert_eq!(10000 - work, 8);
        let mut old = fixture();
        old.profile = Profile::NativeV76;
        let mut work = 10000;
        assert!(
            old.qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut work)
                .unwrap()
                .is_none()
        );
        assert_eq!(work, 10000);
        let mut grouped = fixture();
        wrap_repeat(&mut grouped);
        let mut work = 10000;
        let proof = grouped
            .qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut work)
            .unwrap()
            .unwrap();
        let Kind::Sequence(items) = &grouped.rules["Root"].body.kind else {
            panic!("Expected grouped fixture sequence")
        };
        assert!(std::ptr::eq(proof.expression(), &items[1]));
        assert_eq!(10000 - work, 11);
        for allowance in 0..=11 {
            let mut work = allowance;
            let result =
                grouped.qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut work);
            if allowance == 11 {
                assert!(result.unwrap().is_some());
            } else {
                assert_eq!(result.err().unwrap(), "Grammar prediction budget exceeded");
            }
            assert_eq!(work, 0);
        }
        let mut unlimited = None;
        assert!(
            grouped
                .qualified_nullable_decision_with_work("rule/Root/choice", &mut unlimited)
                .unwrap()
                .is_none()
        );
        assert!(matches!(
            grouped
                .qualified_nullable_decision_with_work("rule/Root/s1/repeat", &mut unlimited)
                .unwrap(),
            Some(QualifiedNullableDecision::FullRepeat(_))
        ));
        assert_eq!(unlimited, None);
    }
    #[test]
    fn nullable_source_rejects_foreign_range_wrong_path_hidden_recursive_or_nonwhole_shapes() {
        for path in [
            "rule/Missing/s1/repeat",
            "rule/Root/s99/repeat",
            "wrong/Root/s1/repeat",
            "rule/Root/value/repeat",
        ] {
            let mut work = 10000;
            assert!(
                fixture()
                    .qualified_nullable_full_repeat_with_work(path, &mut work)
                    .is_err()
            );
            assert!(work < 10000);
        }
        let mut foreign = fixture();
        let Kind::Sequence(items) = &mut foreign.rules.get_mut("Root").unwrap().body.kind else {
            panic!("Expected sequence")
        };
        items[1].original_range = Some(5..7);
        assert!(
            foreign
                .qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut 10000)
                .is_err()
        );
        for mode in 0..4 {
            let mut grammar = fixture();
            match mode {
                0 => grammar.rules.get_mut("Target").unwrap().hidden = Some(vec!["WS".into()]),
                1 => grammar.rules.get_mut("Target").unwrap().fragment = true,
                2 => {
                    let Kind::Sequence(items) =
                        &mut grammar.rules.get_mut("Target").unwrap().body.kind
                    else {
                        panic!("Expected sequence")
                    };
                    let Kind::Assign { value, .. } = &mut items[1].kind else {
                        panic!("Expected assignment")
                    };
                    value.kind = Kind::Call("Target".into());
                }
                _ => {
                    let Kind::Sequence(items) =
                        &mut grammar.rules.get_mut("Root").unwrap().body.kind
                    else {
                        panic!("Expected sequence")
                    };
                    items[1].cardinality = '+';
                }
            }
            assert!(
                grammar
                    .qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut 10000)
                    .unwrap()
                    .is_none()
            );
        }
        for mode in 0..4 {
            let mut grouped = fixture();
            wrap_repeat(&mut grouped);
            let Kind::Sequence(items) = &mut grouped.rules.get_mut("Root").unwrap().body.kind
            else {
                panic!("Expected sequence")
            };
            let Kind::Sequence(group) = &mut items[1].kind else {
                panic!("Expected retained group")
            };
            match mode {
                0 => group.push(node(Kind::Literal("extra".into()), '1', 2, 3)),
                1 => group[0].cardinality = '?',
                2 => group[0].first_set_predicated = true,
                _ => group[0].original_range = Some(5..7),
            }
            assert!(
                grouped
                    .qualified_nullable_full_repeat_with_work("rule/Root/s1/repeat", &mut 10000)
                    .unwrap()
                    .is_none()
            );
        }
    }
    #[test]
    fn required_nullable_full_source_proof_binds_owner_call_and_all_short_work() {
        let grammar = fixture();
        let path = "rule/Root/s1/value/value";
        let mut work = 10000;
        let (proof, repeat) = grammar
            .qualified_required_nullable_full_call_with_work("Root", path, "Target", &mut work)
            .unwrap()
            .unwrap();
        assert_eq!(
            (
                proof.owner(),
                proof.callee(),
                proof.keyword(),
                repeat.as_str()
            ),
            ("Root", "Target", "x", "rule/Root/s1/repeat")
        );
        let used = 10000 - work;
        assert_eq!(used, 20);
        for allowance in 0..=used {
            let mut work = allowance;
            let result = grammar
                .qualified_required_nullable_full_call_with_work("Root", path, "Target", &mut work);
            if allowance == used {
                assert!(result.unwrap().is_some());
            } else {
                assert_eq!(result.err().unwrap(), "Grammar prediction budget exceeded");
            }
            assert_eq!(work, 0);
        }
        let mut grouped = fixture();
        wrap_repeat(&mut grouped);
        let grouped_path = "rule/Root/s1/s0/value/value";
        let mut work = 10000;
        let (proof, repeat) = grouped
            .qualified_required_nullable_full_call_with_work(
                "Root",
                grouped_path,
                "Target",
                &mut work,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            (proof.owner(), proof.callee(), repeat.as_str()),
            ("Root", "Target", "rule/Root/s1/repeat")
        );
        assert_eq!(10000 - work, 28);
        for allowance in 0..=28 {
            let mut work = allowance;
            let result = grouped.qualified_required_nullable_full_call_with_work(
                "Root",
                grouped_path,
                "Target",
                &mut work,
            );
            if allowance == 28 {
                assert!(result.unwrap().is_some());
            } else {
                assert_eq!(result.err().unwrap(), "Grammar prediction budget exceeded");
            }
            assert_eq!(work, 0);
        }
        for (owner, actual, callee) in [
            ("Wrong", path, "Target"),
            ("Root", path, "Wrong"),
            ("Root", "rule/Root/s99/value/value", "Target"),
        ] {
            assert!(
                grammar
                    .qualified_required_nullable_full_call_with_work(
                        owner, actual, callee, &mut 10000
                    )
                    .is_err()
            );
        }
        assert!(
            grammar
                .qualified_required_nullable_full_call_with_work(
                    "Root",
                    "rule/Root/s1/value",
                    "Target",
                    &mut 10000
                )
                .unwrap()
                .is_none()
        );
        let mut old = fixture();
        old.profile = Profile::NativeV76;
        let mut work = 0;
        assert!(
            old.qualified_required_nullable_full_call_with_work("Root", path, "Target", &mut work)
                .unwrap()
                .is_none()
        );
        assert_eq!(work, 0);
    }
    include!("nullable_repetition/fused_tests.rs");
}
