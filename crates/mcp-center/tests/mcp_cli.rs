use assert_cmd::{Command, cargo::cargo_bin};
use mcp_center::{Layout, ServerConfig};
use std::{fs, path::Path, process::Command as StdCommand, time::Duration};
use tempfile::tempdir;

fn cli_with_root(root: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("mcp-center").expect("binary exists");
    cmd.args(["--root", root.to_str().unwrap()]);
    cmd.args(args);
    // Force English output for consistent test assertions
    cmd.env("MCP_CENTER_LANG", "en");
    cmd.assert()
}

fn load_server_configs(root: &Path) -> Vec<ServerConfig> {
    let layout = Layout::new(root.to_path_buf());
    let servers_dir = layout.servers_dir();
    if !servers_dir.exists() {
        return Vec::new();
    }
    fs::read_dir(servers_dir)
        .expect("servers dir")
        .map(|entry| {
            let entry = entry.expect("dir entry");
            ServerConfig::from_file(entry.path()).expect("config parses")
        })
        .collect()
}

#[test]
fn mcp_flow_manage_server_configs() {
    let tmp = tempdir().expect("temp dir");
    let root = tmp.path().to_path_buf();
    let test_server_name = "ExampleHttp";
    let test_endpoint = "https://example.com/mcp";

    // add HTTP server inline
    cli_with_root(
        &root,
        &["mcp", "add", test_server_name, "--protocol", "http", "--url", test_endpoint],
    )
    .success();

    let configs = load_server_configs(&root);
    assert!(!configs.is_empty(), "expected at least one config after add command");

    let added = configs
        .iter()
        .find(|cfg| cfg.definition().name.as_deref() == Some(test_server_name))
        .expect("added server present");
    let server_id = added.definition().id.clone();
    assert!(!server_id.is_empty(), "server id should be generated on add");
    assert_eq!(
        added.definition().endpoint.as_deref(),
        Some(test_endpoint),
        "endpoint should match input"
    );
    assert!(!added.definition().enabled, "server should be disabled by default");

    // mcp list should include the server name
    let list_output = cli_with_root(&root, &["mcp", "list"]).success();
    let list_stdout = String::from_utf8(list_output.get_output().stdout.clone()).unwrap();
    assert!(
        list_stdout.contains(test_server_name),
        "server name should appear in list output"
    );

    // info returns JSON with matching id
    let info_output = cli_with_root(&root, &["mcp", "info", test_server_name])
        .success()
        .get_output()
        .stdout
        .clone();
    let info: serde_json::Value =
        serde_json::from_slice(&info_output).expect("info output is valid JSON");
    assert_eq!(info.get("name").and_then(|v| v.as_str()), Some(test_server_name));
    assert_eq!(info.get("id").and_then(|v| v.as_str()), Some(server_id.as_str()));

    // enable and verify config toggled
    cli_with_root(&root, &["mcp", "enable", test_server_name]).success();
    let enabled_cfg = load_server_configs(&root)
        .into_iter()
        .find(|cfg| cfg.definition().id == server_id)
        .expect("server still present");
    assert!(enabled_cfg.definition().enabled, "server should be enabled");

    // disable again
    cli_with_root(&root, &["mcp", "disable", test_server_name]).success();
    let disabled_cfg = load_server_configs(&root)
        .into_iter()
        .find(|cfg| cfg.definition().id == server_id)
        .expect("server still present");
    assert!(!disabled_cfg.definition().enabled, "server should be disabled");

    // start daemon in background to exercise list-tools (even if no tools)
    let mut daemon = StdCommand::new(cargo_bin("mcp-center"))
        .args(["--root", root.to_str().unwrap(), "serve"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn daemon");

    // wait for rpc socket to appear
    let layout = Layout::new(root.clone());
    let rpc_socket = layout.daemon_rpc_socket_path();
    for _ in 0..50 {
        if rpc_socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(rpc_socket.exists(), "rpc socket should exist after daemon start");

    cli_with_root(&root, &["mcp", "list-tools"]).success();

    let _ = daemon.kill();
    let _ = daemon.wait();

    // remove server config
    cli_with_root(&root, &["mcp", "remove", test_server_name, "--yes"]).success();
    let configs_after = load_server_configs(&root);
    let still_exists = configs_after.iter().any(|cfg| cfg.definition().id == server_id);
    assert!(!still_exists, "server config should be removed");
}
