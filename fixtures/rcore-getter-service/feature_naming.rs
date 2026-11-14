//! Shared native/WASM controls for the exact authored language policy bodies.
use intersect_model::{
    core::query::QueryBudget,
    definitions::{
        bundle::{DefinitionBundle, DefinitionSource, SourceKind},
        rcore_behavior::{ir_v2::EntrySelection, modules::SymbolId},
    },
    source_scalar_functions::{ScalarType, SourceScalarFunction},
};
use serde_json::{Value, json};

fn compile(text: &str, declaration: usize) -> Result<SourceScalarFunction, String> {
    let source = DefinitionSource::new(
        "intersect-kerml:/semantics/feature-naming.rcore",
        SourceKind::RcoreBehavior,
        text,
    );
    let entry = source.id.clone();
    SourceScalarFunction::compile(
        DefinitionBundle {
            version: 1,
            sources: vec![source],
            namespaces: vec![],
        },
        &entry,
        &EntrySelection {
            symbol: SymbolId {
                source: entry.clone(),
                declaration,
            },
            types: vec![],
            arguments: vec![],
        },
    )
}

fn check(program: &SourceScalarFunction, inputs: &[Value], expected: &Value) -> Result<(), String> {
    let mut measured = QueryBudget::new(1_000_000);
    if &program.invoke(inputs, &mut measured)? != expected {
        return Err("Authored Feature naming result differs".into());
    }
    let prior = 7;
    let mut exact = QueryBudget::new(prior + measured.reads);
    exact.charge_many(prior)?;
    if &program.invoke(inputs, &mut exact)? != expected || exact.remaining() != 0 {
        return Err("Feature naming replaced its original caller ledger".into());
    }
    let mut short = QueryBudget::new(prior + measured.reads - 1);
    short.charge_many(prior)?;
    if program.invoke(inputs, &mut short).is_ok()
        || short.remaining() != 0
        || program.invoke(inputs, &mut short).is_ok()
    {
        return Err("Short Feature naming invocation recovered its allowance".into());
    }
    Ok(())
}

pub fn verify() -> Result<(), String> {
    let text = include_str!("feature-naming.rcore");
    let has = compile(text, 0)?;
    let select = compile(text, 1)?;
    let proof = compile(text, 2)?;
    use ScalarType::{Bool, OptionalString};
    has.check_signature(&[OptionalString, OptionalString], Bool)?;
    select.check_signature(&[OptionalString, OptionalString, Bool], OptionalString)?;
    proof.check_signature(&[Bool; 7], OptionalString)?;
    for (parameters, result) in [
        (&[Bool, OptionalString][..], Bool),
        (&[OptionalString][..], Bool),
        (&[OptionalString, OptionalString][..], OptionalString),
    ] {
        if has.check_signature(parameters, result).is_ok() {
            return Err("Feature policy compatibility ABI changed silently".into());
        }
    }
    for name in [Value::Null, json!(""), json!("π😀")] {
        for short in [Value::Null, json!(""), json!("s")] {
            check(
                &has,
                &[name.clone(), short.clone()],
                &Value::Bool(!name.is_null() || !short.is_null()),
            )?;
            for use_short in [false, true] {
                check(
                    &select,
                    &[name.clone(), short.clone(), Value::Bool(use_short)],
                    if use_short { &short } else { &name },
                )?;
            }
        }
    }
    for bits in 0u8..128 {
        let flags = std::array::from_fn::<_, 7, _>(|index| bits & (1 << index) != 0);
        let [
            assignment,
            specialized,
            end,
            directed,
            constructor,
            transition,
            multiplicity,
        ] = flags;
        let expected = if specialized {
            json!("Unsupported specialized computed Feature naming adapter")
        } else if assignment {
            json!("Unsupported assignment-context computed Feature naming")
        } else if !multiplicity && (end || directed || constructor || transition) {
            json!("Unsupported parameter/end/constructor/transition computed Feature naming")
        } else {
            Value::Null
        };
        check(&proof, &flags.map(Value::Bool), &expected)?;
    }
    let changed = compile(&text.replace("if specialized", "if false"), 2)?;
    if changed.source() == proof.source() || changed.contract_id() == proof.contract_id() {
        return Err("Feature policy source revision was not retained".into());
    }
    check(
        &changed,
        &[false, true, false, false, false, false, false].map(Value::Bool),
        &Value::Null,
    )?;
    for bad in [
        vec![],
        vec![Value::Null],
        vec![json!(true), Value::Null],
        vec![json!(4), Value::Null],
    ] {
        if has.invoke(&bad, &mut QueryBudget::new(1_000_000)).is_ok() {
            return Err("Feature policy accepted a foreign parameter contract".into());
        }
    }
    for body in [
        "fn bad(value: String) -> String { value }",
        "fn bad(value: Bool) -> Int64 { 1 }",
        "extern fn bad(value: Bool) -> Bool;",
    ] {
        if compile(
            &format!("rcore 1; package rejected version \"1\"; {body}"),
            0,
        )
        .is_ok()
        {
            return Err("Scalar codec admitted an unsupported signature/provider".into());
        }
    }
    // Exercise both input and output string-chunk ledgers beyond one block.
    check(
        &select,
        &[json!("é😀".repeat(100)), Value::Null, json!(false)],
        &json!("é😀".repeat(100)),
    )?;
    Ok(())
}
