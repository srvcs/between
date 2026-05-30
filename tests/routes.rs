use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use axum::{Json, Router as AxumRouter};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use srvcs_between::{api::Deps, health, router, telemetry};
use tower::ServiceExt;

/// Spawn a mock dependency that COMPUTES its answer from the request body using
/// the provided closure, returning `200 {"result": <computed>}`. This genuinely
/// exercises the orchestration: between must thread the right operands to each
/// dependency and combine the booleans correctly.
async fn spawn_computing_mock<F>(compute: F) -> String
where
    F: Fn(Value) -> Value + Clone + Send + Sync + 'static,
{
    let app = AxumRouter::new().route(
        "/",
        post(move |Json(body): Json<Value>| {
            let compute = compute.clone();
            async move { Json(json!({ "result": compute(body) })) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// Spawn a mock that always answers with a fixed status + body (for error paths).
async fn spawn_fixed_mock(status: StatusCode, body: Value) -> String {
    let app = AxumRouter::new().route(
        "/",
        post(move || {
            let body = body.clone();
            async move { (status, Json(body)) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

/// `a >= b` over integers.
fn gte(body: Value) -> Value {
    let a = body["a"].as_i64().unwrap();
    let b = body["b"].as_i64().unwrap();
    json!(a >= b)
}

/// `a <= b` over integers.
fn lte(body: Value) -> Value {
    let a = body["a"].as_i64().unwrap();
    let b = body["b"].as_i64().unwrap();
    json!(a <= b)
}

/// `a AND b` over booleans.
fn and(body: Value) -> Value {
    let a = body["a"].as_bool().unwrap();
    let b = body["b"].as_bool().unwrap();
    json!(a && b)
}

fn app(gte_url: &str, lte_url: &str, and_url: &str) -> axum::Router {
    router(
        telemetry::metrics_handle_for_tests(),
        Deps {
            greaterthanorequalto_url: gte_url.to_string(),
            lessthanorequalto_url: lte_url.to_string(),
            and_url: and_url.to_string(),
        },
    )
}

async fn eval(gte_url: &str, lte_url: &str, and_url: &str, body: Value) -> (StatusCode, Value) {
    let res = app(gte_url, lte_url, and_url)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

const DEAD_URL: &str = "http://127.0.0.1:1";

/// Spawn the three computing mocks (gte, lte, and) and return their URLs.
async fn computing_deps() -> (String, String, String) {
    let g = spawn_computing_mock(gte).await;
    let l = spawn_computing_mock(lte).await;
    let a = spawn_computing_mock(and).await;
    (g, l, a)
}

async fn status_of(uri: &str) -> StatusCode {
    app(DEAD_URL, DEAD_URL, DEAD_URL)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn healthz_ok() {
    assert_eq!(status_of("/healthz").await, StatusCode::OK);
}

#[tokio::test]
async fn readyz_reflects_state() {
    health::set_ready(true);
    assert_eq!(status_of("/readyz").await, StatusCode::OK);
}

#[tokio::test]
async fn openapi_ok() {
    assert_eq!(status_of("/openapi.json").await, StatusCode::OK);
}

// --- Asserted correctness cases from the spec ---

#[tokio::test]
async fn between_5_0_10_is_true() {
    let (g, l, a) = computing_deps().await;
    let (status, body) = eval(&g, &l, &a, json!({ "value": 5, "lo": 0, "hi": 10 })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!(true));
    assert_eq!(body["value"], json!(5));
    assert_eq!(body["lo"], json!(0));
    assert_eq!(body["hi"], json!(10));
}

#[tokio::test]
async fn between_neg1_0_10_is_false() {
    let (g, l, a) = computing_deps().await;
    let (status, body) = eval(&g, &l, &a, json!({ "value": -1, "lo": 0, "hi": 10 })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!(false));
}

#[tokio::test]
async fn above_hi_is_false() {
    // value > hi: gte true, lte false -> AND false.
    let (g, l, a) = computing_deps().await;
    let (status, body) = eval(&g, &l, &a, json!({ "value": 11, "lo": 0, "hi": 10 })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["result"], json!(false));
}

#[tokio::test]
async fn at_boundaries_is_true() {
    // Inclusive range: value == lo and value == hi both qualify.
    let (g, l, a) = computing_deps().await;
    let (s_lo, b_lo) = eval(&g, &l, &a, json!({ "value": 0, "lo": 0, "hi": 10 })).await;
    assert_eq!(s_lo, StatusCode::OK);
    assert_eq!(b_lo["result"], json!(true));
    let (s_hi, b_hi) = eval(&g, &l, &a, json!({ "value": 10, "lo": 0, "hi": 10 })).await;
    assert_eq!(s_hi, StatusCode::OK);
    assert_eq!(b_hi["result"], json!(true));
}

// --- Error / edge cases ---

#[tokio::test]
async fn forwards_invalid_input_from_gte() {
    // The greaterthanorequalto leaf rejects a non-integer operand; between
    // forwards the 422 unchanged.
    let g = spawn_fixed_mock(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "value is not an integer" }),
    )
    .await;
    let l = spawn_computing_mock(lte).await;
    let a = spawn_computing_mock(and).await;
    let (status, body) = eval(&g, &l, &a, json!({ "value": 4.5, "lo": 0, "hi": 10 })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"], "value is not an integer");
}

#[tokio::test]
async fn forwards_invalid_input_from_lte() {
    let g = spawn_computing_mock(gte).await;
    let l = spawn_fixed_mock(
        StatusCode::UNPROCESSABLE_ENTITY,
        json!({ "error": "value is not an integer" }),
    )
    .await;
    let a = spawn_computing_mock(and).await;
    let (status, _) = eval(&g, &l, &a, json!({ "value": 4, "lo": 0, "hi": 10 })).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// --- Degraded paths (one dependency unreachable -> 503) ---

#[tokio::test]
async fn degrades_when_gte_unreachable() {
    let l = spawn_computing_mock(lte).await;
    let a = spawn_computing_mock(and).await;
    let (status, body) = eval(DEAD_URL, &l, &a, json!({ "value": 5, "lo": 0, "hi": 10 })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-greaterthanorequalto");
}

#[tokio::test]
async fn degrades_when_lte_unreachable() {
    let g = spawn_computing_mock(gte).await;
    let a = spawn_computing_mock(and).await;
    let (status, body) = eval(&g, DEAD_URL, &a, json!({ "value": 5, "lo": 0, "hi": 10 })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-lessthanorequalto");
}

#[tokio::test]
async fn degrades_when_and_unreachable() {
    let g = spawn_computing_mock(gte).await;
    let l = spawn_computing_mock(lte).await;
    let (status, body) = eval(&g, &l, DEAD_URL, json!({ "value": 5, "lo": 0, "hi": 10 })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["dependency"], "srvcs-and");
}
