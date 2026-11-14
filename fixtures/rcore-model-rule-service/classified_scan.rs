//! Exact authored selector over a lazy sequence of existing checked observations.
use intersect_model::{
    core::query::QueryBudget, rcore_behavior_functions::ExecutionLimits,
    semantic_reads::ClassifiedValue, source_classified_reads::SourceClassifiedRead,
};
use serde_json::json;
pub const BODY: &str = include_str!("first-conjugator.rcore");
pub fn compile(body: &str) -> Result<SourceClassifiedRead, String> {
    SourceClassifiedRead::compile(
        "intersect-kerml:/semantics/first-conjugator.rcore",
        body,
        1,
        ExecutionLimits {
            max_frames: 4096,
            ..Default::default()
        },
    )
}
type Selection = Option<(usize, serde_json::Value)>;
fn run(
    program: &SourceClassifiedRead,
    stop: usize,
    budget: &mut QueryBudget,
) -> Result<(Selection, Vec<usize>), String> {
    let original = budget as *const QueryBudget;
    let mut calls = Vec::new();
    let result = program.invoke(
        |index, work| {
            assert!(std::ptr::eq(work as *const QueryBudget, original));
            calls.push(index);
            work.charge_many(3)?;
            if index < stop {
                Ok(ClassifiedValue::NonMatch)
            } else if index == stop {
                Ok(ClassifiedValue::Match(json!({"selected":index})))
            } else {
                Err("A later observation must stay unread".into())
            }
        },
        budget,
    )?;
    Ok((result, calls))
}
pub fn verify() -> Result<(), String> {
    let program = compile(BODY)?;
    for stop in [0, 1, 12, 130] {
        let mut measured = QueryBudget::new(10_000_000);
        let (result, calls) = run(&program, stop, &mut measured)?;
        assert_eq!(result, Some((stop, json!({"selected":stop}))));
        assert_eq!(calls, (0..=stop).collect::<Vec<_>>());
        let mut exact = QueryBudget::new(7 + measured.reads);
        exact.charge_many(7)?;
        assert_eq!(run(&program, stop, &mut exact)?.0, result);
        assert_eq!(exact.remaining(), 0);
        let mut short = QueryBudget::new(7 + measured.reads - 1);
        short.charge_many(7)?;
        assert!(run(&program, stop, &mut short).is_err());
        assert_eq!(short.remaining(), 0);
        assert!(run(&program, stop, &mut short).is_err());
    }
    let mut seen = Vec::new();
    assert_eq!(
        program.invoke(
            |index, budget| {
                budget.charge()?;
                seen.push(index);
                Ok(ClassifiedValue::Absent)
            },
            &mut QueryBudget::new(1_000_000)
        )?,
        None
    );
    assert_eq!(seen, [0]);
    let mut seen = Vec::new();
    let failure = program.invoke(
        |index, budget| {
            budget.charge()?;
            seen.push(index);
            if index == 0 {
                Ok(ClassifiedValue::NonMatch)
            } else {
                Err("original checked observation failed".into())
            }
        },
        &mut QueryBudget::new(1_000_000),
    );
    assert_eq!(
        failure.err().as_deref(),
        Some("original checked observation failed")
    );
    assert_eq!(seen, [0, 1]);
    let changed = compile(&BODY.replace(
        "if observation.matches",
        "if observation.matches && observation.position > 0",
    ))?;
    let read = |index, budget: &mut QueryBudget| {
        budget.charge()?;
        Ok(ClassifiedValue::Match(json!(index)))
    };
    assert_eq!(
        program.invoke(read, &mut QueryBudget::new(1_000_000))?,
        Some((0, json!(0)))
    );
    assert_eq!(
        changed.invoke(read, &mut QueryBudget::new(1_000_000))?,
        Some((1, json!(1)))
    );
    assert_eq!(
        program.invoke(
            |_, budget| {
                budget.charge()?;
                Ok(ClassifiedValue::Match(serde_json::Value::Null))
            },
            &mut QueryBudget::new(1_000_000)
        )?,
        Some((0, serde_json::Value::Null))
    );
    Ok(())
}
