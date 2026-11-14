//! Separately paid immutable initial facts for exact selected terminal programs.
use super::*;
use crate::grammar::terminals::Terminal;
const REGISTRY_HEADER: usize = 64;
const TERMINAL_SLOT: usize = 16;
pub(in crate::grammar) struct PreparedTerminalInitials<'program> {
    terminals: &'program BTreeMap<String, Terminal>,
    selected: Box<[&'program Terminal]>,
    rows: Box<[PreparedLexicalInitial<'program>]>,
    logical_bytes: usize,
}
impl<'program> PreparedLexicalInitial<'program> {
    /// Only selected names are resolved. No input, selected token or body result
    /// is retained. This extends the existing finite pool and combined cap.
    #[cfg(test)]
    pub(crate) fn with_terminal_initials<'selection>(
        self,
        terminals: &'program BTreeMap<String, Terminal>,
        names: impl IntoIterator<Item = &'selection str>,
        work: &mut Option<usize>,
        logical_storage_limit: usize,
    ) -> Result<Self, String> {
        self.with_terminal_initials_observed(terminals, names, work, logical_storage_limit, None)
    }
    pub(super) fn with_terminal_initials_observed<'selection>(
        mut self,
        terminals: &'program BTreeMap<String, Terminal>,
        names: impl IntoIterator<Item = &'selection str>,
        work: &mut Option<usize>,
        logical_storage_limit: usize,
        observation: Option<&InvocationContext<'_>>,
    ) -> Result<Self, String> {
        if observation.is_some() {
            LexerPrediction::consume_observed(work, observation)?;
        } else {
            self.check(self.program, work)?;
        }
        if self.terminal_initials.is_some() {
            return Err("Terminal initials already prepared".into());
        }
        let remaining = logical_storage_limit
            .checked_sub(self.logical_bytes)
            .ok_or("Combined lexical preparation storage budget exceeded")?;
        let registry = PreparedTerminalInitials::prepare_observed(
            terminals,
            names,
            work,
            remaining,
            observation,
        )?;
        self.logical_bytes = self
            .logical_bytes
            .checked_add(registry.logical_bytes)
            .ok_or("Combined lexical preparation storage overflow")?;
        self.terminal_initials = Some(Box::new(registry));
        Ok(self)
    }
    #[cfg(test)]
    pub(crate) fn terminal_initial_programs(&self) -> usize {
        self.terminal_initials
            .as_ref()
            .map_or(0, |registry| registry.rows.len())
    }
    pub(crate) fn check_terminals(
        &self,
        terminals: &BTreeMap<String, Terminal>,
        work: &mut Option<usize>,
    ) -> Result<(), String> {
        if let Some(registry) = &self.terminal_initials {
            LexerPrediction::consume(work)?;
            if !std::ptr::eq(registry.terminals, terminals) {
                return Err(
                    "Prepared terminal initials differ from exact live terminal map".into(),
                );
            }
        }
        Ok(())
    }
    pub(in crate::grammar) fn terminal_registry(
        &self,
    ) -> Option<&PreparedTerminalInitials<'program>> {
        self.terminal_initials.as_deref()
    }
}
impl<'program> PreparedTerminalInitials<'program> {
    fn prepare_observed<'selection>(
        terminals: &'program BTreeMap<String, Terminal>,
        names: impl IntoIterator<Item = &'selection str>,
        work: &mut Option<usize>,
        cap: usize,
        observation: Option<&InvocationContext<'_>>,
    ) -> Result<Self, String> {
        let mut selected_names = BTreeSet::new();
        for name in names {
            LexerPrediction::consume_observed(work, observation)?;
            if !selected_names.insert(name) {
                return Err("Duplicate prepared terminal name".into());
            }
        }
        if selected_names.is_empty() {
            return Err("Prepared terminal selection is empty".into());
        }
        let mut logical = REGISTRY_HEADER;
        if logical > cap {
            return Err("Terminal initial registry storage budget exceeded".into());
        }
        for _ in 0..REGISTRY_HEADER.div_ceil(ADMISSION_QUANTUM) {
            LexerPrediction::consume_observed(work, observation)?;
        }
        if let Some(observation) = observation {
            observation.retain_storage(REGISTRY_HEADER)?;
        }
        let mut selected = Vec::new();
        let mut rows = Vec::new();
        for name in selected_names {
            // Deterministic paid name resolution, independent of BTreeMap's
            // internal comparison implementation or process pointer addresses.
            let mut terminal = None;
            for (key, value) in terminals {
                LexerPrediction::consume_observed(work, observation)?;
                if key == name {
                    terminal = Some(value);
                    break;
                }
            }
            let terminal = terminal.ok_or("Unknown prepared terminal name")?;
            logical = logical
                .checked_add(TERMINAL_SLOT)
                .ok_or("Terminal initial registry storage overflow")?;
            if logical > cap {
                return Err("Terminal initial registry storage budget exceeded".into());
            }
            for _ in 0..TERMINAL_SLOT.div_ceil(ADMISSION_QUANTUM) {
                LexerPrediction::consume_observed(work, observation)?;
            }
            if let Some(observation) = observation {
                observation.retain_storage(TERMINAL_SLOT)?;
            }
            selected
                .try_reserve_exact(1)
                .map_err(|_| "Terminal root allocation failed")?;
            selected.push(terminal);
            let before = rows.len();
            let mut pending = vec![(&terminal.body, 0usize)];
            while let Some((expr, depth)) = pending.pop() {
                LexerPrediction::consume_observed(work, observation)?;
                if depth > 64 {
                    return Err("Prepared terminal expression depth exceeded".into());
                }
                let until = if let Operation::UntilOperand { decision, .. } = &expr.kind {
                    decision.as_ref()
                } else {
                    None
                };
                for program in [
                    expr.cardinality_decision.as_ref(),
                    expr.decision.as_ref(),
                    until,
                ]
                .into_iter()
                .flatten()
                {
                    LexerPrediction::consume_observed(work, observation)?;
                    let remaining = cap
                        .checked_sub(logical)
                        .ok_or("Terminal initial registry storage budget exceeded")?;
                    let row = PreparedLexicalInitial::prepare_observed(
                        program,
                        work,
                        remaining,
                        observation,
                    )?;
                    logical = logical
                        .checked_add(row.logical_bytes())
                        .ok_or("Terminal initial registry storage overflow")?;
                    rows.try_reserve_exact(1)
                        .map_err(|_| "Terminal initial row allocation failed")?;
                    rows.push(row);
                }
                match &expr.kind {
                    Operation::Sequence(items) | Operation::Alternative(items) => {
                        for child in items.iter().rev() {
                            pending.push((child, depth + 1));
                        }
                    }
                    Operation::Call {
                        body: Some(child), ..
                    }
                    | Operation::Negated { operand: child, .. }
                    | Operation::UntilOperand { operand: child, .. } => {
                        pending.push((child, depth + 1))
                    }
                    _ => {}
                }
            }
            if rows.len() == before {
                return Err("Selected terminal has no local prediction programs".into());
            }
        }
        // Boxed slices retain exactly admitted slots, never spare Vec capacity.
        Ok(Self {
            terminals,
            selected: selected.into_boxed_slice(),
            rows: rows.into_boxed_slice(),
            logical_bytes: logical,
        })
    }
    pub(in crate::grammar) fn selects(
        &self,
        terminal: &Terminal,
        work: &mut Option<usize>,
    ) -> Result<bool, String> {
        for actual in self.selected.iter() {
            LexerPrediction::consume(work)?;
            if std::ptr::eq(*actual, terminal) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(in crate::grammar) fn predict(
        &self,
        program: &LexerPrediction,
        units: impl Iterator<Item = u16>,
        work: &mut Option<usize>,
        nongreedy: bool,
    ) -> Result<usize, String> {
        for row in self.rows.iter() {
            LexerPrediction::consume(work)?;
            if std::ptr::eq(row.program, program) {
                return program
                    .predict_units_with_initial::<true>(units, work, nongreedy, Some(row))?
                    .selected();
            }
        }
        // Only a selected terminal uses this policy, and preparation visited
        // all its actual programs. Unselected terminals take the cold policy
        // before entering Matcher; a foreign/missing local program is an error.
        Err("Prepared terminal local program differs from actual bound registry".into())
    }
}
const _: () = assert!(std::mem::size_of::<PreparedTerminalInitials<'static>>() <= REGISTRY_HEADER);
const _: () = assert!(std::mem::size_of::<&Terminal>() <= TERMINAL_SLOT);
#[cfg(test)]
#[path = "lexer_terminal_initials_tests.rs"]
mod tests;
