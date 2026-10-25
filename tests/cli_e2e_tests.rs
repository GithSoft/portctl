use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_version_flag() {
    let mut cmd = Command::cargo_bin("portctl").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("portctl 2.1.0"));
}

#[test]
fn test_cli_help_flag() {
    let mut cmd = Command::cargo_bin("portctl").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Port Manager"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn test_cli_list_json() {
    let tmp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("portctl").unwrap();
    cmd.env("PORTCTL_CONFIG_DIR", tmp.path().join("config"))
        .env("PORTCTL_REGISTRY_DIR", tmp.path().join("registry"))
        .arg("list")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("["));
}

#[test]
fn test_cli_assign_and_get() {
    let tmp = tempdir().unwrap();
    let config_dir = tmp.path().join("config");
    let registry_dir = tmp.path().join("registry");

    // Assign port 4567 to my-test-svc
    let mut assign_cmd = Command::cargo_bin("portctl").unwrap();
    assign_cmd
        .env("PORTCTL_CONFIG_DIR", &config_dir)
        .env("PORTCTL_REGISTRY_DIR", &registry_dir)
        .args(["assign", "my-test-svc", "-f", "4567"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-test-svc"))
        .stdout(predicate::str::contains("4567"));

    // Get port for my-test-svc
    let mut get_cmd = Command::cargo_bin("portctl").unwrap();
    get_cmd
        .env("PORTCTL_CONFIG_DIR", &config_dir)
        .env("PORTCTL_REGISTRY_DIR", &registry_dir)
        .args(["get", "my-test-svc"])
        .assert()
        .success()
        .stdout(predicate::str::contains("4567"));
}

#[test]
fn test_cli_check_port() {
    let tmp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("portctl").unwrap();
    cmd.env("PORTCTL_CONFIG_DIR", tmp.path().join("config"))
        .env("PORTCTL_REGISTRY_DIR", tmp.path().join("registry"))
        .args(["check", "59999"])
        .assert()
        .success()
        .stdout(predicate::str::contains("59999"))
        .stdout(predicate::str::contains("Registry: Not registered"));
}

#[test]
fn test_cli_env_export() {
    let tmp = tempdir().unwrap();
    let config_dir = tmp.path().join("config");
    let registry_dir = tmp.path().join("registry");

    // Assign port first
    let mut assign_cmd = Command::cargo_bin("portctl").unwrap();
    assign_cmd
        .env("PORTCTL_CONFIG_DIR", &config_dir)
        .env("PORTCTL_REGISTRY_DIR", &registry_dir)
        .args(["assign", "cli-env-test", "-f", "4888"])
        .assert()
        .success();

    // In a directory that detects cli-env-test
    let project_dir = tmp.path().join("cli-env-test");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(
        project_dir.join("package.json"),
        r#"{"name": "cli-env-test"}"#,
    )
    .unwrap();

    let mut env_cmd = Command::cargo_bin("portctl").unwrap();
    env_cmd
        .current_dir(&project_dir)
        .env("PORTCTL_CONFIG_DIR", &config_dir)
        .env("PORTCTL_REGISTRY_DIR", &registry_dir)
        .args(["env", "-e"])
        .assert()
        .success()
        .stdout(predicate::str::contains("export PORT=4888"));
}

#[test]
fn test_cli_doctor_json_and_fix() {
    let tmp = tempdir().unwrap();
    let config_dir = tmp.path().join("config");
    let registry_dir = tmp.path().join("registry");

    let mut doc_cmd = Command::cargo_bin("portctl").unwrap();
    doc_cmd
        .env("PORTCTL_CONFIG_DIR", &config_dir)
        .env("PORTCTL_REGISTRY_DIR", &registry_dir)
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_issues\""))
        .stdout(predicate::str::contains("\"is_healthy\""));

    let mut fix_cmd = Command::cargo_bin("portctl").unwrap();
    fix_cmd
        .env("PORTCTL_CONFIG_DIR", &config_dir)
        .env("PORTCTL_REGISTRY_DIR", &registry_dir)
        .args(["doctor", "--fix"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SYSTEM DIAGNOSTIC REPORT"));
}

#[test]
fn test_cli_kill_idle_and_guardrail() {
    let tmp = tempdir().unwrap();
    let config_dir = tmp.path().join("config");
    let registry_dir = tmp.path().join("registry");

    // 1. Idle port kill test
    let mut kill_idle_cmd = Command::cargo_bin("portctl").unwrap();
    kill_idle_cmd
        .env("PORTCTL_CONFIG_DIR", &config_dir)
        .env("PORTCTL_REGISTRY_DIR", &registry_dir)
        .args(["kill", "59123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already idle"));

    // 2. Sensitive port blocked by guardrail
    let mut kill_sens_cmd = Command::cargo_bin("portctl").unwrap();
    kill_sens_cmd
        .env("PORTCTL_CONFIG_DIR", &config_dir)
        .env("PORTCTL_REGISTRY_DIR", &registry_dir)
        .args(["kill", "22"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SECURITY GUARDRAIL"));
}

#[test]
fn test_cli_shell_completions() {
    let mut bash_cmd = Command::cargo_bin("portctl").unwrap();
    bash_cmd
        .args(["completion", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_portctl"));

    let mut zsh_cmd = Command::cargo_bin("portctl").unwrap();
    zsh_cmd
        .args(["completion", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef portctl"));
}

#[test]
fn test_cli_unexpose_unregistered() {
    let tmp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("portctl").unwrap();
    cmd.env("PORTCTL_CONFIG_DIR", tmp.path().join("config"))
        .env("PORTCTL_REGISTRY_DIR", tmp.path().join("registry"))
        .args(["unexpose", "nonexistent-app"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not registered"));
}
