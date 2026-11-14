//! Handwritten source-bound fragment inference. Execution stays in the shared
//! grammar compiler and interpreter; this module owns metamodel correspondence.
use super::*;

pub(super) fn check_actions(engine: &Engine<'_>, element: &Element) -> Result<()> {
    match &element.value {
        ElementKind::Action { .. } => {
            return Err(engine.at(element, "Actions are not allowed in parser fragments"));
        }
        ElementKind::Unordered(items) if engine.unordered_groups => {
            for item in items {
                check_actions(engine, item)?;
            }
        }
        ElementKind::Sequence(items)
        | ElementKind::Alternatives(items)
        | ElementKind::Guard {
            elements: items, ..
        } => {
            for item in items {
                check_actions(engine, item)?;
            }
        }
        ElementKind::Group(item)
        | ElementKind::Cardinality { element: item, .. }
        | ElementKind::Predicate { element: item, .. }
        | ElementKind::Assignment { value: item, .. } => check_actions(engine, item)?,
        _ => {}
    }
    Ok(())
}

impl Engine<'_> {
    pub(super) fn current_source(&self) -> &SourceId {
        self.fragment_scope
            .as_ref()
            .map_or(&self.source.id, |scope| &scope.source)
    }
    pub(super) fn current_aliases(&self) -> &BTreeMap<String, Vec<String>> {
        self.fragment_scope
            .as_ref()
            .map_or(&self.aliases, |scope| &scope.aliases)
    }
    pub(super) fn binding(&self, name: &str) -> Option<RuleBinding> {
        if let Some(scope) = &self.fragment_scope {
            return scope.rules.get(name).cloned();
        }
        if let Some(local) = name.strip_prefix("super::") {
            return self
                .inherited
                .and_then(|p| p.effective_rules().get(local))
                .cloned();
        }
        self.syntax
            .rules
            .iter()
            .find(|r| r.value.name.value == name)
            .map(|r| RuleBinding {
                parameters: r.value.parameters.clone().unwrap_or_default(),
                grammar: self.syntax.name.value.clone(),
                declaration: self.span(r),
                kind: r.value.kind,
                target: self.rules.get(name).cloned(),
                wildcard: r.value.wildcard,
                implicit: r.value.return_type.is_none(),
            })
            .or_else(|| {
                self.inherited
                    .and_then(|p| p.effective_rules().get(name))
                    .cloned()
            })
    }
    fn scope_rules(&self) -> BTreeMap<String, RuleBinding> {
        let mut rules = self
            .inherited
            .map(|p| p.effective_rules().clone())
            .unwrap_or_default();
        if let Some(parent) = self.inherited {
            for (name, binding) in parent.effective_rules() {
                rules.insert(format!("super::{name}"), binding.clone());
            }
        }
        for rule in &self.syntax.rules {
            if let Some(binding) = self.binding(&rule.value.name.value) {
                rules.insert(rule.value.name.value.clone(), binding);
            }
        }
        rules
    }
    pub(super) fn local_scope(&self) -> InferenceScope {
        InferenceScope {
            referenced_ecore: self.referenced_ecore(),
            source: self.source.id.clone(),
            aliases: self.aliases.clone(),
            rules: self.scope_rules(),
            fragments: self
                .syntax
                .rules
                .iter()
                .filter(|r| r.value.kind == RuleKind::Fragment)
                .map(|r| (r.value.name.value.clone(), r.value.body.clone()))
                .collect(),
        }
    }
    pub(super) fn check_fragment_override(
        &self,
        rule: &Located<syntax::Rule>,
        target: Option<&TypeId>,
    ) -> Result<()> {
        let Some(prior) = self
            .inherited
            .and_then(|p| p.effective_rules().get(&rule.value.name.value))
        else {
            return Ok(());
        };
        let fragment = rule.value.kind == RuleKind::Fragment;
        if fragment != (prior.kind == RuleKind::Fragment) {
            return Err(self.at(rule, "Overrides must preserve parser fragment category"));
        }
        if fragment
            && (rule.value.wildcard != prior.wildcard
                || (!rule.value.wildcard && target != prior.target.as_ref()))
        {
            return Err(self.at(
                rule,
                "Fragment overrides must preserve wildcard and exact return identity",
            ));
        }
        Ok(())
    }
    fn fragment_body(
        &self,
        binding: &RuleBinding,
        name: &Located<String>,
    ) -> Result<(InferenceScope, Element)> {
        let scope = if binding.declaration.source == self.source.id {
            self.local_scope()
        } else {
            self.inherited
                .and_then(|p| {
                    p.grammars()
                        .iter()
                        .find(|g| g.source == binding.declaration.source)
                })
                .map(|g| g.scope.clone())
                .ok_or_else(|| self.at(name, "Missing exact fragment source context"))?
        };
        let local = name.value.strip_prefix("super::").unwrap_or(&name.value);
        let body = scope
            .fragments
            .get(local)
            .cloned()
            .ok_or_else(|| self.at(name, "Missing declared fragment body"))?;
        Ok((scope, body))
    }
    pub(super) fn fragment_hierarchy(
        &mut self,
        name: &Located<String>,
        binding: &RuleBinding,
        current: BTreeSet<TypeId>,
        root: &TypeId,
    ) -> Result<BTreeSet<TypeId>> {
        // Wildcard calls inline features, but pinned inference does not derive
        // caller supertypes through their bodies. Preserve that boundary exactly.
        if binding.wildcard {
            return Ok(current);
        }
        let target = binding
            .target
            .as_ref()
            .ok_or_else(|| self.at(name, "Typed fragment has no return identity"))?;
        if !self.class(target) {
            return Ok(current);
        }
        let (scope, body) = self.fragment_body(binding, name)?;
        self.add_parent(root, target, name)?;
        let key = (scope.source.clone(), name.value.clone());
        if !self.active_fragments.insert(key.clone()) {
            return Ok(current);
        }
        if self.active_fragments.len() > 64 {
            return Err(self.at(name, "Fragment inference nesting limit exceeded"));
        }
        let previous = self.fragment_scope.replace(scope);
        let result = self.hierarchy(&body, current, root);
        self.fragment_scope = previous;
        self.active_fragments.remove(&key);
        result
    }
    pub(super) fn fragment_features(
        &mut self,
        name: &Located<String>,
        binding: &RuleBinding,
        current: BTreeSet<TypeId>,
        root: &TypeId,
    ) -> Result<BTreeSet<TypeId>> {
        let (scope, body) = self.fragment_body(binding, name)?;
        let key = (scope.source.clone(), name.value.clone());
        if self.active_fragments.len() >= 64 || !self.active_fragments.insert(key.clone()) {
            return Err(self.at(name, "Cyclic or over-deep wildcard fragment inference"));
        }
        let previous = self.fragment_scope.replace(scope);
        let result = self.features(&body, current, root, None);
        self.fragment_scope = previous;
        self.active_fragments.remove(&key);
        result
    }
}
