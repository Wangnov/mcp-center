use assert_cmd::Command;
use mcp_center::{ServerConfig, ServerDefinition};
use std::fs;
use tempfile::tempdir;

fn run_cli(args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("mcp-center").expect("binary exists");
    cmd.args(args);
    // Force English output for consistent test assertions
    cmd.env("MCP_CENTER_LANG", "en");
    cmd.assert()
}

#[test]
fn init_creates_sample_server_configs() {
    let tmp = tempdir().expect("temp dir");
    let root = tmp.path().to_str().unwrap();

    run_cli(&["--root", root, "init"]).success();

    let servers_dir = tmp.path().join("config/servers");
    assert!(servers_dir.is_dir(), "expected servers directory at {}", servers_dir.display());

    let mut configs: Vec<ServerConfig> = fs::read_dir(&servers_dir)
        .expect("servers dir readable")
        .map(|entry| {
            let entry = entry.expect("dir entry");
            ServerConfig::from_file(entry.path()).expect("config parses")
        })
        .collect();

    configs.sort_by(|a, b| {
        a.definition()
            .name
            .as_deref()
            .unwrap_or_default()
            .cmp(b.definition().name.as_deref().unwrap_or_default())
    });

    assert!(configs.len() >= 2, "expected sample configs, found {}", configs.len());

    for config in configs {
        let definition: &ServerDefinition = config.definition();
        assert!(!definition.id.is_empty(), "server definition should have generated id");
        assert!(
            !definition.name.as_deref().unwrap_or_default().is_empty(),
            "sample server should have name"
        );
        // sample configs are disabled by default
        assert!(!definition.enabled, "sample server {} should be disabled", definition.id);
    }

    // running init again should be idempotent and mention already initialized
    let rerun = run_cli(&["--root", root, "init"]).success();
    let rerun_stdout = String::from_utf8(rerun.get_output().stdout.clone()).expect("stdout utf8");
    assert!(
        rerun_stdout.contains("Added sample config")
            || rerun_stdout.contains("already initialized"),
        "unexpected init rerun output: {rerun_stdout}"
    );
}
