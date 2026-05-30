use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

use crate::client::{self, DepError};

pub const SERVICE: &str = "srvcs-between";
pub const CONCERN: &str = "comparison: is value within [lo, hi]";
pub const DEPENDS_ON: &[&str] = &[
    "srvcs-greaterthanorequalto",
    "srvcs-lessthanorequalto",
    "srvcs-and",
];

/// Dependency endpoints, injected as router state so tests can point them at
/// mock services.
#[derive(Clone)]
pub struct Deps {
    pub greaterthanorequalto_url: String,
    pub lessthanorequalto_url: String,
    pub and_url: String,
}

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    #[schema(value_type = Object)]
    pub value: Value,
    #[schema(value_type = Object)]
    pub lo: Value,
    #[schema(value_type = Object)]
    pub hi: Value,
}

#[derive(Serialize, ToSchema)]
pub struct BetweenResponse {
    #[schema(value_type = Object)]
    pub value: Value,
    #[schema(value_type = Object)]
    pub lo: Value,
    #[schema(value_type = Object)]
    pub hi: Value,
    /// `true` iff `lo <= value <= hi`.
    pub result: bool,
}

fn ok(value: Value, lo: Value, hi: Value, result: bool) -> Response {
    (
        StatusCode::OK,
        Json(json!({ "value": value, "lo": lo, "hi": hi, "result": result })),
    )
        .into_response()
}

fn degraded(dependency: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "dependency unavailable", "dependency": dependency })),
    )
        .into_response()
}

/// Forward a dependency's response verbatim (used to propagate `422` for invalid
/// input, so between reports the same rejection a leaf dependency did).
fn forward(status: u16, body: Value) -> Response {
    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_GATEWAY);
    (code, Json(body)).into_response()
}

/// Ask one boolean dependency with `payload` for its `result`, mapping its
/// failures to the response this service should return.
async fn ask(url: &str, payload: &Value, dependency: &str) -> Result<bool, Response> {
    match client::call(url, payload).await {
        Err(DepError::Unreachable) => Err(degraded(dependency)),
        Ok((200, body)) => Ok(body.get("result").and_then(Value::as_bool).unwrap_or(false)),
        // Invalid input propagates from the leaf dependency; forward it.
        Ok((422, body)) => Err(forward(422, body)),
        Ok(_) => Err(degraded(dependency)),
    }
}

/// `POST /` — is `value` within the inclusive range `[lo, hi]`?
///
/// This service does no comparison of its own. It computes
/// `(value >= lo) AND (value <= hi)` by asking three primitives:
/// `srvcs-greaterthanorequalto` for `value >= lo`,
/// `srvcs-lessthanorequalto` for `value <= hi`, and `srvcs-and` to combine the
/// two booleans. Invalid operands are rejected by the leaf comparison
/// dependencies and the resulting `422` is forwarded unchanged.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = BetweenResponse),
        (status = 422, description = "an operand is not a valid integer (forwarded)"),
        (status = 500, description = "a dependency returned an unexpected shape"),
        (status = 503, description = "a dependency is unavailable")
    )
)]
pub async fn evaluate(State(deps): State<Deps>, Json(req): Json<EvalRequest>) -> Response {
    // value >= lo
    let g = match ask(
        &deps.greaterthanorequalto_url,
        &json!({ "a": req.value, "b": req.lo }),
        "srvcs-greaterthanorequalto",
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    // value <= hi
    let l = match ask(
        &deps.lessthanorequalto_url,
        &json!({ "a": req.value, "b": req.hi }),
        "srvcs-lessthanorequalto",
    )
    .await
    {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    // g AND l
    let result = match ask(&deps.and_url, &json!({ "a": g, "b": l }), "srvcs-and").await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    ok(req.value, req.lo, req.hi, result)
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, BetweenResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some());
        assert!(root.post.is_some());
    }

    #[tokio::test]
    async fn index_reports_all_dependencies() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-between");
        assert_eq!(
            info.depends_on,
            vec![
                "srvcs-greaterthanorequalto",
                "srvcs-lessthanorequalto",
                "srvcs-and"
            ]
        );
    }
}
