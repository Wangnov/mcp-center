use assert_cmd::Command;
use mcp_center::{
    Layout, ProjectId, ProjectRegistry,
    project::{ToolCustomization, ToolPermission},
};
use std::{fs, path::Path};
use tempfile::tempdir;

fn cli_with_root(root: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("mcp-center").expect("binary exists");
    cmd.args(["--root", root.to_str().unwrap()]);
    cmd.args(args);
    // Force English output for consistent test assertions
    cmd.env("MCP_CENTER_LANG", "en");
    cmd.assert()
}

#[test]
fn project_cli_manages_permissions_and_customizations() {
    let tmp = tempdir().expect("temp dir");
    let root = tmp.path().to_path_buf();
    let layout = Layout::new(root.clone());
    let project_dir = tmp.path().join("workspace");
    fs::create_dir(&project_dir).expect("create project dir");
    let canonical = fs::canonicalize(&project_dir).unwrap_or(project_dir.clone());
    let project_arg = canonical.to_str().unwrap().to_string();

    // add a dummy server definition so project allow can validate server name
    cli_with_root(
        &root,
        &[
            "mcp",
            "add",
            "ExampleHttp",
            "--protocol",
            "http",
            "--url",
            "https://example.test/mcp",
        ],
    )
    .success();

    // add project
    cli_with_root(&root, &["project", "add", &project_arg]).success();
    let registry = ProjectRegistry::new(&layout);
    registry.ensure().expect("registry exists");
    let project_id = ProjectId::from_path(&canonical);

    let mut record = registry.load(&project_id).expect("record exists after add");
    assert_eq!(record.allowed_server_ids.len(), 0);

    let server_config =
        layout.load_server_config_by_name("ExampleHttp").expect("server config present");
    let server_id = server_config.definition().id.clone();

    // allow server for project
    cli_with_root(&root, &["project", "allow", &project_arg, &server_id]).success();
    record = registry.load(&project_id).expect("record after allow");
    assert_eq!(record.allowed_server_ids, vec![server_id.clone()]);

    // allow specific tools
    cli_with_root(
        &root,
        &[
            "project",
            "allow-tools",
            &project_arg,
            &format!("{server_id}::resolve-library-id"),
            &format!("{server_id}::get-library-docs"),
        ],
    )
    .success();
    record = registry.load(&project_id).expect("record after allow-tools");
    let permission = record.allowed_server_tools.get(&server_id).expect("tool permission exists");
    match permission {
        ToolPermission::AllowList { tools } => {
            assert_eq!(
                tools,
                &vec!["resolve-library-id".to_string(), "get-library-docs".to_string()]
            );
        }
        other => panic!("unexpected permission {other:?}"),
    }

    // deny tools overwrites permission
    cli_with_root(
        &root,
        &["project", "deny-tools", &project_arg, &format!("{server_id}::ask_question")],
    )
    .success();
    record = registry.load(&project_id).expect("record after deny-tools");
    let permission = record.allowed_server_tools.get(&server_id).expect("tool permission exists");
    match permission {
        ToolPermission::DenyList { tools } => {
            assert_eq!(tools, &vec!["ask_question".to_string()]);
        }
        other => panic!("unexpected permission {other:?}"),
    }

    // set custom description
    let custom_desc = "[TEST] custom prompt";
    cli_with_root(
        &root,
        &["project", "set-tool-desc", &project_arg, "resolve-library-id", custom_desc],
    )
    .success();
    record = registry.load(&project_id).expect("record after set desc");
    assert!(
        record
            .tool_customizations
            .iter()
            .any(|ToolCustomization { tool_name, description }| tool_name == "resolve-library-id"
                && description.as_deref() == Some(custom_desc)),
        "custom description stored"
    );

    // reset custom description
    cli_with_root(&root, &["project", "reset-tool-desc", &project_arg, "resolve-library-id"])
        .success();
    record = registry.load(&project_id).expect("record after reset desc");
    assert!(
        record.tool_customizations.iter().all(|c| c.tool_name != "resolve-library-id"),
        "custom description cleared"
    );

    // deny server access removes from allowed list
    cli_with_root(&root, &["project", "deny", &project_arg, &server_id]).success();
    record = registry.load(&project_id).expect("record after deny");
    assert!(record.allowed_server_ids.is_empty(), "server removed from allow list");

    // remove project
    cli_with_root(&root, &["project", "remove", &project_arg, "--yes"]).success();
    assert!(registry.load(&project_id).is_err(), "project record removed");
}
