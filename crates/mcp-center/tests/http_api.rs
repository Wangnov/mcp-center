use std::{collections::BTreeMap, fs, sync::Arc};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
};
use mcp_center::{
    Layout,
    config::{ServerConfig, ServerDefinition, ServerProtocol},
    daemon::server_manager::ServerManager,
    project::ProjectRegistry,
    web::http::{self, HttpState},
};
use serde_json::{Value, json};
use tempfile::tempdir;
use tokio::runtime::Runtime;
use tower::ServiceExt;

fn test_runtime() -> Runtime {
    Runtime::new().expect("create tokio runtime")
}

async fn make_router(layout: Layout) -> Router {
    let manager = Arc::new(ServerManager::start(layout.clone()).await.unwrap());
    let registry = ProjectRegistry::new(&layout);
    registry.ensure().unwrap();

    let state =
        HttpState { manager, registry, layout, auth: http::HttpAuth::new(Some("secret".into())) };
    http::build_router(state)
}

fn write_server_config(layout: &Layout, id: &str, name: &str, enabled: bool) {
    let definition = ServerDefinition {
        id: id.to_string(),
        name: Some(name.to_string()),
        protocol: ServerProtocol::StdIo,
        command: "echo".to_string(),
        args: Vec::new(),
        env: BTreeMap::new(),
        endpoint: None,
        headers: BTreeMap::new(),
        enabled,
    };
    let config = ServerConfig::new(definition).unwrap();
    let toml = config.to_toml_string().unwrap();
    fs::write(layout.server_config_path(id), toml).unwrap();
}

#[test]
fn http_api_project_allow_and_deny() {
    test_runtime().block_on(async {
        let tmp = tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        layout.ensure().unwrap();

        write_server_config(&layout, "demo", "Demo", false);

        let router = make_router(layout.clone()).await;

        let project_dir = tmp.path().join("workspace");
        fs::create_dir_all(&project_dir).unwrap();
        let target = project_dir.to_str().unwrap();

        let payload = json!({
            "target": target,
            "servers": ["demo"]
        });

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/project/allow")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret")
                    .header("x-mcp-client", "web")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let summary: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary["allowed_server_ids"], json!(["demo"]));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/project")
                    .header("authorization", "Bearer secret")
                    .header("x-mcp-client", "web")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let projects: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(projects["projects"][0]["allowed_server_ids"], json!(["demo"]));

        let deny_payload = json!({
            "target": target,
            "servers": ["demo"]
        });

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/project/deny")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret")
                    .header("x-mcp-client", "web")
                    .body(Body::from(deny_payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let summary: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(summary["allowed_server_ids"], json!([]));

        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::PATCH)
                    .uri("/api/mcp/demo/enabled")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret")
                    .header("x-mcp-client", "web")
                    .body(Body::from(json!({ "enabled": true }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let config = layout.load_server_config("demo").unwrap();
        assert!(config.definition().enabled);

        // 获取服务器详情
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/mcp/demo")
                    .header("authorization", "Bearer secret")
                    .header("x-mcp-client", "web")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let detail: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(detail["server"]["id"], json!("demo"));
        assert_eq!(detail["tools"], json!([]));

        // 获取服务器工具列表
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/mcp/demo/tools")
                    .header("authorization", "Bearer secret")
                    .header("x-mcp-client", "web")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let tools: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(tools["tools"], json!([]));

        // 通过 API 新增一个服务器
        let payload = json!({
            "name": "Echo",
            "protocol": "stdio",
            "command": "echo",
            "args": "hello"
        });
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/mcp")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret")
                    .header("x-mcp-client", "web")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        // 删除原有服务器
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/api/mcp/demo")
                    .header("authorization", "Bearer secret")
                    .header("x-mcp-client", "web")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        assert!(!layout.server_config_path("demo").exists());

        let configs = layout.list_server_configs().unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].definition().name.as_deref(), Some("Echo"));
    });
}

#[test]
fn http_api_requires_auth_token() {
    test_runtime().block_on(async {
        let tmp = tempdir().unwrap();
        let layout = Layout::new(tmp.path().to_path_buf());
        layout.ensure().unwrap();

        let router = make_router(layout.clone()).await;

        let response = router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/mcp")
                    .header("x-mcp-client", "web")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    });
}
