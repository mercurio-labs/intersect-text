//! Handwritten Xtext hidden-reference admission over the existing source world.
//! A terminal's spelling is not authority: resolve its declaration/category in
//! the exact authored grammar scope. Execution uses the shared retained cursor.
use super::*;

impl Engine<'_> {
    pub(super) fn check_hidden_policy(&self) -> Result<()> {
        let check = |policy: &[Located<String>]| -> Result<()> {
            for target in policy {
                // The upstream hidden EReference uses the unqualified rule scope,
                // unlike ordinary explicit RuleCalls. Preserve qualified failures.
                if target.value.contains("::") {
                    return Err(self.at(
                        target,
                        "Qualified hidden terminal references are unresolved",
                    ));
                }
                let declaration = self
                    .binding(&target.value)
                    .ok_or_else(|| self.at(target, "Unresolved hidden terminal declaration"))?;
                if declaration.kind != RuleKind::Terminal {
                    return Err(self.at(
                        target,
                        "Hidden tokens require non-fragment terminal declarations",
                    ));
                }
            }
            Ok(())
        };
        if let Some(policy) = &self.syntax.hidden {
            check(policy)?;
        }
        for rule in &self.syntax.rules {
            if let Some(policy) = &rule.value.hidden {
                check(policy)?;
            }
        }
        Ok(())
    }
}
