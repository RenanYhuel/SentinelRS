use anyhow::Result;

use crate::client;
use crate::output::{print_json, theme, OutputMode};
use crate::store;

pub async fn execute(mode: OutputMode, server: Option<String>) -> Result<()> {
    if mode == OutputMode::Human {
        theme::print_header("System Diagnostics");
    }

    let checks = run_checks(server.as_deref()).await;

    match mode {
        OutputMode::Json => {
            let items: Vec<_> = checks
                .iter()
                .map(|(name, ok, detail)| {
                    serde_json::json!({ "check": name, "ok": ok, "detail": detail })
                })
                .collect();
            print_json(&items)?;
        }
        OutputMode::Human => {
            for (name, ok, detail) in &checks {
                theme::print_kv_colored(name, detail, *ok);
            }
            println!();
            let passed = checks.iter().filter(|(_, ok, _)| *ok).count();
            let total = checks.len();
            theme::print_kv("Result", &format!("{passed}/{total} checks passed"));
            println!();
        }
    }

    Ok(())
}

async fn run_checks(server_flag: Option<&str>) -> Vec<(String, bool, String)> {
    let mut results = Vec::new();

    let config_ok = store::exists();
    results.push((
        "CLI Config".into(),
        config_ok,
        if config_ok {
            store::config_path().display().to_string()
        } else {
            "not found — run `sentinel init`".into()
        },
    ));

    let (server_reachable, server_detail) = check_server(server_flag).await;
    results.push(("Server Health".into(), server_reachable, server_detail));

    let (ready, ready_detail) = check_ready(server_flag).await;
    results.push(("Server Ready".into(), ready, ready_detail));

    let agent_cfg = check_agent_config();
    results.push(agent_cfg);

    let wal = check_wal_dir();
    results.push(wal);

    results
}

async fn check_server(server_flag: Option<&str>) -> (bool, String) {
    match client::build_client(server_flag) {
        Ok(c) => {
            let ok = c.health_ok().await;
            (
                ok,
                if ok {
                    "reachable".into()
                } else {
                    "unreachable".into()
                },
            )
        }
        Err(e) => (false, format!("config error: {e}")),
    }
}

async fn check_ready(server_flag: Option<&str>) -> (bool, String) {
    match client::build_client(server_flag) {
        Ok(c) => match c.get_text("/ready").await {
            Ok(_) => (true, "ready".into()),
            Err(_) => (false, "not ready".into()),
        },
        Err(_) => (false, "skipped".into()),
    }
}

fn check_agent_config() -> (String, bool, String) {
    let paths = [
        dirs::config_dir()
            .unwrap_or_default()
            .join("sentinel")
            .join("agent.yml"),
        std::path::PathBuf::from("/etc/sentinel/agent.yml"),
    ];
    for p in &paths {
        if p.exists() {
            return ("Agent Config".into(), true, p.display().to_string());
        }
    }
    ("Agent Config".into(), false, "not found".into())
}

fn check_wal_dir() -> (String, bool, String) {
    let dirs_to_check = [
        dirs::data_dir()
            .unwrap_or_default()
            .join("sentinel")
            .join("wal"),
        std::path::PathBuf::from("/var/lib/sentinel/wal"),
    ];
    for d in &dirs_to_check {
        if d.exists() {
            return ("WAL Directory".into(), true, d.display().to_string());
        }
    }
    ("WAL Directory".into(), false, "not found".into())
}
