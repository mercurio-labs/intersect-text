#[path = "../../support/lifecycle.rs"]
mod lifecycle;
#[path = "../../support/sessions.rs"]
mod sessions;
pub fn observe() -> serde_json::Value {
    serde_json::json!({"sessions":sessions::observe(),"lifecycle":lifecycle::observe()})
}
