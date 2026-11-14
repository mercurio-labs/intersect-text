use intersect_model::definitions::bundle::{DefinitionSource, SourceKind};
use intersect_web::function_jobs::FunctionJobs;
use serde_json::{Value, json};

fn input(body: &str) -> Value {
    let source = DefinitionSource::new(
        "memory:jobs-behavior.rcore",
        SourceKind::RcoreBehavior,
        format!("rcore 1; package jobs version \"1\"; {body}"),
    );
    json!({"method":"rcore_behavior/evaluateFunction","bundle":{"version":1,"sources":[source],"namespaces":[]},"entry":source.id,"selection":{"symbol":{"source":source.id,"declaration":0},"types":[],"arguments":[]},"arguments":[]})
}
fn start(jobs: &mut FunctionJobs, request: &Value) -> Value {
    let mut request = request.clone();
    request["method"] = json!("rcore_behavior/startFunction");
    jobs.dispatch(&request.to_string()).expect("start")
}
fn command(jobs: &mut FunctionJobs, method: &str, id: &Value) -> Result<Value, String> {
    jobs.dispatch(&json!({"method":method,"job":id,"quantum":1}).to_string())
}
fn finish(jobs: &mut FunctionJobs, id: &Value) -> Value {
    for _ in 0..1000 {
        let response = command(jobs, "rcore_behavior/resumeFunction", id).expect("resume");
        if response["pending"] == false {
            return response["result"].clone();
        }
    }
    panic!("small fixture failed to finish");
}
#[test]
fn retained_jobs_preserve_complete_results_and_release_terminal_state() {
    let mut jobs = FunctionJobs::new("native-host-a".into()).unwrap();
    for body in [
        "fn run()->Int8={let f=make(40);f(2)};fn make(x:Int8)->(Int8)->Int8=|y:Int8|x+y;",
        "fn run()->Int8=fail::<Int8>(0);fn fail<T>(x:Int8)->Int8=1/x;",
    ] {
        let request = input(body);
        let expected = intersect_web::dispatch(&request.to_string()).unwrap();
        let started = start(&mut jobs, &request);
        assert_eq!(started["pending"], true, "{started}");
        let id = &started["job"];
        assert_eq!(jobs.active_jobs(), 1);
        assert!(jobs.retained_encoded_bytes() > 0);
        assert_eq!(finish(&mut jobs, id), expected);
        assert_eq!(jobs.active_jobs(), 0);
        assert_eq!(jobs.retained_encoded_bytes(), 0);
        assert!(command(&mut jobs, "rcore_behavior/resumeFunction", id).is_err());
    }
}
#[test]
fn cancellation_drop_and_foreign_ids_do_not_affect_other_jobs() {
    let mut jobs = FunctionJobs::new("native-host-a".into()).unwrap();
    let mut foreign = FunctionJobs::new("native-host-b".into()).unwrap();
    let request = input("fn run()->Int8={let a:Int8=20;let b:Int8=22;a+b};");
    let first = start(&mut jobs, &request)["job"].clone();
    let second = start(&mut jobs, &request)["job"].clone();
    let other = start(&mut foreign, &request)["job"].clone();
    assert!(command(&mut jobs, "rcore_behavior/resumeFunction", &other).is_err());
    for _ in 0..3 {
        assert_eq!(
            command(&mut jobs, "rcore_behavior/resumeFunction", &first).unwrap()["pending"],
            true
        );
    }
    let cancel = jobs
        .dispatch(&json!({"method":"rcore_behavior/cancelFunction","job":first}).to_string())
        .unwrap();
    assert_eq!(cancel["pending"], false);
    assert_eq!(cancel["result"]["failure"]["kind"], "Cancelled");
    assert_eq!(jobs.active_jobs(), 1);
    assert!(command(&mut jobs, "rcore_behavior/resumeFunction", &first).is_err());
    assert_eq!(
        finish(&mut jobs, &second),
        intersect_web::dispatch(&request.to_string()).unwrap()
    );
    assert_eq!(jobs.retained_encoded_bytes(), 0);
    let fresh = start(&mut jobs, &request)["job"].clone();
    assert_ne!(fresh, first);
    let drop_request = json!({"method":"rcore_behavior/dropFunction","job":fresh}).to_string();
    assert_eq!(jobs.dispatch(&drop_request).unwrap()["dropped"], true);
    assert!(jobs.dispatch(&drop_request).is_err());
    assert_eq!(jobs.retained_encoded_bytes(), 0);
    assert_eq!(finish(&mut foreign, &other)["ok"], true);
}
#[test]
fn admission_and_invalid_commands_preserve_existing_jobs() {
    let mut jobs = FunctionJobs::new("native-host-a".into()).unwrap();
    let request = input("fn run()->Int8=42;");
    let mut ids = Vec::new();
    for _ in 0..4 {
        ids.push(start(&mut jobs, &request)["job"].clone());
    }
    let retained = jobs.retained_encoded_bytes();
    let mut overflow = request.clone();
    overflow["method"] = json!("rcore_behavior/startFunction");
    assert!(
        jobs.dispatch(&overflow.to_string())
            .unwrap_err()
            .contains("Active function job budget")
    );
    for quantum in [json!(0), json!(4097), json!("1"), json!(null), json!(-1)] {
        assert!(
            jobs.dispatch(
                &json!({"method":"rcore_behavior/resumeFunction","job":ids[0],"quantum":quantum})
                    .to_string()
            )
            .is_err()
        );
    }
    assert!(
        jobs.dispatch(
            &json!({"method":"rcore_behavior/dropFunction","job":ids[0],"extra":true}).to_string()
        )
        .is_err()
    );
    assert_eq!(jobs.active_jobs(), 4);
    assert_eq!(jobs.retained_encoded_bytes(), retained);
    for id in ids {
        assert_eq!(finish(&mut jobs, &id)["ok"], true);
    }
    let fresh = start(&mut jobs, &request);
    assert_eq!(finish(&mut jobs, &fresh["job"])["ok"], true);
}
#[test]
fn preparation_rejections_are_terminal_and_do_not_reserve_slots() {
    let mut jobs = FunctionJobs::new("native-host-a".into()).unwrap();
    let request = input("fn run()->Int8=missing;");
    let rejected = start(&mut jobs, &request);
    assert_eq!(rejected["pending"], false, "{rejected}");
    assert_eq!(
        rejected["result"],
        intersect_web::dispatch(&request.to_string()).unwrap()
    );
    assert_eq!(rejected["result"]["ok"], false);
    let mut replay = input("fn run()->Int8=42;");
    replay["expected_program"] =
        intersect_web::dispatch(&replay.to_string()).unwrap()["program"].clone();
    replay["expected_program"]["entry"] = json!(900);
    let rejected = start(&mut jobs, &replay);
    assert_eq!(rejected["pending"], false);
    assert_eq!(rejected["result"]["stage"], "function_replay");
    assert_eq!(
        rejected["result"],
        intersect_web::dispatch(&replay.to_string()).unwrap()
    );
    assert_eq!(jobs.active_jobs(), 0);
    assert_eq!(jobs.retained_encoded_bytes(), 0);
    for namespace in ["", "short", "host:invalid", "host space"] {
        assert!(FunctionJobs::new(namespace.into()).is_err());
    }
}

#[test]
fn instance_adapter_initializes_once_and_preserves_legacy_dispatch() {
    use intersect_web::function_jobs::FunctionJobHost;
    let mut host = FunctionJobHost::default();
    let request = input("fn run()->Int8=42;");
    let expected = intersect_web::dispatch(&request.to_string()).unwrap();
    assert_eq!(host.dispatch(&request.to_string()).unwrap(), expected);
    let mut start = request.clone();
    start["method"] = json!("rcore_behavior/startFunction");
    assert!(
        host.dispatch(&start.to_string())
            .unwrap_err()
            .contains("not initialized")
    );
    let initialization = json!({"method":"rcore_behavior/jobHost","namespace":"host-instance-one"});
    let mut invalid = initialization.clone();
    invalid["extra"] = json!(true);
    assert!(host.dispatch(&invalid.to_string()).is_err());
    assert_eq!(
        host.dispatch(&initialization.to_string()).unwrap()["initialized"],
        true
    );
    let started = host.dispatch(&start.to_string()).unwrap();
    assert_eq!(started["pending"], true);
    assert!(
        host.dispatch(&initialization.to_string())
            .unwrap_err()
            .contains("already initialized")
    );
    let result = host
        .dispatch(
            &json!({"method":"rcore_behavior/resumeFunction","job":started["job"],"quantum":4096})
                .to_string(),
        )
        .unwrap();
    assert_eq!(result["pending"], false);
    assert_eq!(result["result"], expected);
}

#[test]
fn encoded_retention_admits_exact_boundaries_and_releases_every_terminal_path() {
    for body in [
        "fn run()->Int8=42;",
        "fn run()->Int8=1/0;",
        "fn run()->String=\"é😀\";",
    ] {
        let request = input(body);
        let expected = intersect_web::dispatch(&request.to_string()).unwrap();
        let mut start_request = request.clone();
        start_request["method"] = json!("rcore_behavior/startFunction");
        let wire = start_request.to_string();
        let bytes = wire.len()
            + 2 * expected["program"].to_string().len()
            + expected["program"]["policies"].to_string().len();
        let mut below = FunctionJobs::with_retained_limit("below-limit".into(), bytes - 1).unwrap();
        assert!(
            below
                .dispatch(&wire)
                .unwrap_err()
                .contains("Retained job byte budget")
        );
        assert_eq!(below.active_jobs(), 0);
        assert_eq!(below.retained_encoded_bytes(), 0);
        let mut jobs = FunctionJobs::with_retained_limit("exact-limit".into(), 2 * bytes).unwrap();
        let first = jobs.dispatch(&wire).unwrap()["job"].clone();
        let second = jobs.dispatch(&wire).unwrap()["job"].clone();
        assert_eq!(jobs.retained_encoded_bytes(), 2 * bytes);
        assert!(
            jobs.dispatch(&wire)
                .unwrap_err()
                .contains("Retained job byte budget")
        );
        assert_eq!(jobs.active_jobs(), 2);
        assert_eq!(jobs.retained_encoded_bytes(), 2 * bytes);
        assert_eq!(finish(&mut jobs, &first), expected);
        assert_eq!(jobs.retained_encoded_bytes(), bytes);
        let cancelled = jobs
            .dispatch(&json!({"method":"rcore_behavior/cancelFunction","job":second}).to_string())
            .unwrap();
        assert_eq!(cancelled["result"]["failure"]["kind"], "Cancelled");
        assert_eq!(jobs.retained_encoded_bytes(), 0);
        let third = jobs.dispatch(&wire).unwrap()["job"].clone();
        assert_ne!(first, third);
        jobs.dispatch(&json!({"method":"rcore_behavior/dropFunction","job":third}).to_string())
            .unwrap();
        assert_eq!(jobs.retained_encoded_bytes(), 0);
    }
    assert!(
        FunctionJobs::with_retained_limit("above-ceiling".into(), 64 * 1024 * 1024 + 1).is_err()
    );
    let mut zero = FunctionJobs::with_retained_limit("zero-budget".into(), 0).unwrap();
    let mut request = input("fn run()->Int8=42;");
    request["method"] = json!("rcore_behavior/startFunction");
    assert!(zero.dispatch(&request.to_string()).is_err());
    assert_eq!(zero.active_jobs(), 0);
}

#[test]
fn retained_registry_metadata_is_reserved_separately_from_program_copies() {
    let model = DefinitionSource::new(
        "memory:budget.ecore",
        SourceKind::Ecore,
        r#"<e:EPackage xmlns:e="http://www.eclipse.org/emf/2002/Ecore" xmlns:x="http://www.w3.org/2001/XMLSchema-instance" name="budget" nsURI="urn:budget"><eClassifiers x:type="e:EDataType" name="RGB" instanceClassName="example.RGB"/></e:EPackage>"#,
    );
    let (model, ids) = intersect_text_acceptance::migration_fixture::migrate_source_fixture(model);
    let mut source = DefinitionSource::new(
        "memory:jobs-behavior.rcore",
        SourceKind::RcoreBehavior,
        "rcore 1;package jobs version \"1\";import model \"urn:budget\" as m;fn run()->Int8=42;",
    );
    source.dependencies.push(model.id.clone());
    let mut request = json!({"method":"rcore_behavior/evaluateFunction","bundle":{"version":1,"sources":[model,source],"namespaces":[]},"entry":source.id,"selection":{"symbol":{"source":source.id,"declaration":1},"types":[],"arguments":[]},"arguments":[],"conversions":[{"datatype":ids[&vec![0]],"provider":{"name":"example.rgb","version":"1"},"serializable":true,"declarations":[],"members":[]}]});
    let expected = intersect_web::dispatch(&request.to_string()).unwrap();
    assert_eq!(expected["ok"], true, "{expected}");
    assert_eq!(
        expected["program"]["policies"]["conversions"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    request["method"] = json!("rcore_behavior/startFunction");
    let wire = request.to_string();
    let without_registry = wire.len() + 2 * expected["program"].to_string().len();
    let mut missing =
        FunctionJobs::with_retained_limit("no-registry-space".into(), without_registry).unwrap();
    assert!(
        missing
            .dispatch(&wire)
            .unwrap_err()
            .contains("Retained job byte budget")
    );
    assert_eq!(missing.retained_encoded_bytes(), 0);
    let bytes = without_registry + expected["program"]["policies"].to_string().len();
    let mut exact = FunctionJobs::with_retained_limit("registry-space".into(), bytes).unwrap();
    let started = exact.dispatch(&wire).unwrap();
    assert_eq!(exact.retained_encoded_bytes(), bytes);
    assert_eq!(finish(&mut exact, &started["job"]), expected);
    assert_eq!(exact.retained_encoded_bytes(), 0);
}
