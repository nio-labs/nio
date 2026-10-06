//! Agent subprocess runner and stream manager for Nio.
//!
//! Wraps and auto-detects 16+ real installed coding agents (agy, codex, claude, kilo,
//! kilocode, copilot, gemini, opencode, cline, goose, aider, cursor, continue, plandex,
//! devin, amazon-q), passes context handoffs from NioDB, and streams execution live.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Clone, Debug)]
pub struct DetectedAgent {
    pub id: String,
    pub name: String,
    pub binary_path: PathBuf,
    #[allow(dead_code)]
    pub available: bool,
}

pub fn detect_agents() -> Vec<DetectedAgent> {
    let agent_defs = [
        ("agy", "Google Antigravity (agy)"),
        ("codex", "OpenAI Codex CLI"),
        ("claude", "Anthropic Claude Code"),
        ("kilo", "Kilo Code CLI (kilo)"),
        ("kilocode", "Kilo Code Engine (kilocode)"),
        ("copilot", "GitHub Copilot CLI"),
        ("gemini", "Google Gemini CLI"),
        ("opencode", "OpenCode Assistant"),
        ("cline", "Cline Autonomous Agent"),
        ("goose", "Block Goose Agent"),
        ("aider", "Aider AI Pair Programmer"),
        ("cursor", "Cursor Agent CLI"),
        ("continue", "Continue Dev CLI"),
        ("plandex", "Plandex AI Engine"),
        ("devin", "Devin CLI"),
        ("amazon-q", "Amazon Q Developer"),
    ];

    let mut result = Vec::new();
    for (id, name) in agent_defs {
        if let Some(path) = find_binary(id) {
            result.push(DetectedAgent {
                id: id.to_string(),
                name: name.to_string(),
                binary_path: path,
                available: true,
            });
        }
    }
    result
}

pub fn find_binary(name: &str) -> Option<PathBuf> {
    // 1. Check PATH environment variable
    if let Ok(path_var) = env::var("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // 2. Check standard system directories
    let mut fallbacks = Vec::new();
    if let Ok(home) = env::var("HOME") {
        let home_path = PathBuf::from(&home);
        fallbacks.push(home_path.join(".local/bin").join(name));
        fallbacks.push(home_path.join(".cargo/bin").join(name));

        // Scan NVM node versions directory (e.g. ~/.nvm/versions/node/v*/bin)
        let nvm_node_dir = home_path.join(".nvm/versions/node");
        if let Ok(entries) = fs::read_dir(&nvm_node_dir) {
            for entry in entries.flatten() {
                let bin_candidate = entry.path().join("bin").join(name);
                if bin_candidate.is_file() {
                    return Some(bin_candidate);
                }
            }
        }
    }
    fallbacks.push(PathBuf::from("/usr/local/bin").join(name));
    fallbacks.push(PathBuf::from("/usr/bin").join(name));

    for candidate in fallbacks {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // Special alias check for amazon-q ("q")
    if name == "amazon-q" {
        return find_binary("q");
    }

    None
}

/// Prepares full prompt by injecting NioDB context handoff manifest if available.
pub fn format_handoff_prompt(goal: &str, manifest_opt: Option<&serde_json::Value>) -> String {
    let Some(manifest) = manifest_opt else {
        return goal.to_string();
    };

    let session_id = manifest.get("session_id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let prev_agent = manifest.get("current_agent").and_then(|v| v.as_str()).unwrap_or("none");
    let model = manifest.get("model").and_then(|v| v.as_str()).unwrap_or("default");

    let mut text = format!(
        "[NIO AGENT CONTEXT HANDOFF]\n\
         Session: {session_id} | From: {prev_agent} | Model: {model}\n"
    );

    if let Some(turns) = manifest.get("recent_turns").and_then(|v| v.as_array()) {
        if !turns.is_empty() {
            text.push_str("Recent Completed Turns:\n");
            for turn in turns {
                let ag = turn.get("agent").and_then(|v| v.as_str()).unwrap_or("agent");
                let sm = turn.get("summary").and_then(|v| v.as_str()).unwrap_or("");
                text.push_str(&format!("  • [{ag}]: {sm}\n"));
            }
        }
    }

    if let Some(files) = manifest.get("files_touched").and_then(|v| v.as_array()) {
        if !files.is_empty() {
            text.push_str("Files Touched So Far:\n");
            for f in files {
                if let Some(path) = f.as_str() {
                    text.push_str(&format!("  • {path}\n"));
                }
            }
        }
    }

    if let Some(dead_ends) = manifest.get("known_dead_ends").and_then(|v| v.as_array()) {
        if !dead_ends.is_empty() {
            text.push_str("Known Dead-Ends (DO NOT REPEAT):\n");
            for de in dead_ends {
                let iss = de.get("issue").and_then(|v| v.as_str()).unwrap_or("");
                let att = de.get("attempt").and_then(|v| v.as_str()).unwrap_or("");
                let why = de.get("why").and_then(|v| v.as_str()).unwrap_or("");
                text.push_str(&format!("  • Issue: {iss} | Attempted: {att} | Why Failed: {why}\n"));
            }
        }
    }

    text.push_str("\n[CURRENT OBJECTIVE]\n");
    text.push_str(goal);
    text
}

/// Runs the agent interactively with full TTY inheritance.
pub fn run_agent_interactive(
    agent_id: &str,
    model: Option<&str>,
    initial_prompt: Option<&str>,
) -> Result<i32, String> {
    let binary = find_binary(agent_id)
        .ok_or_else(|| format!("Binary for '{agent_id}' not found in PATH or standard agent locations"))?;

    let mut cmd = std::process::Command::new(binary);
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match agent_id {
        "agy" => {
            if let Some(m) = model {
                cmd.arg("--model").arg(m);
            }
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg("-i").arg(prompt);
                }
            }
        }
        "codex" => {
            if let Some(m) = model {
                cmd.arg("-m").arg(m);
            }
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg(prompt);
                }
            }
        }
        "claude" => {
            if let Some(m) = model {
                cmd.arg("--model").arg(m);
            }
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg(prompt);
                }
            }
        }
        "kilo" | "kilocode" => {
            if let Some(m) = model {
                cmd.arg("-m").arg(m);
            }
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg(prompt);
                }
            }
        }
        "copilot" => {
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg("-p").arg(prompt);
                }
            }
        }
        "gemini" => {
            if let Some(m) = model {
                cmd.arg("-m").arg(m);
            }
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg("-i").arg(prompt);
                }
            }
        }
        "cline" => {
            cmd.arg("-i");
        }
        "goose" => {
            cmd.arg("session");
        }
        "aider" => {
            if let Some(m) = model {
                cmd.arg("--model").arg(m);
            }
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg("--message").arg(prompt);
                }
            }
        }
        "cursor" | "continue" | "opencode" | "plandex" | "devin" | "amazon-q" => {
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg(prompt);
                }
            }
        }
        _other => {
            if let Some(prompt) = initial_prompt {
                if !prompt.trim().is_empty() {
                    cmd.arg(prompt);
                }
            }
        }
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run interactive agent {agent_id}: {e}"))?;

    Ok(status.code().unwrap_or(0))
}

/// Streams agent execution in real-time to stdout/stderr while capturing combined output.
pub async fn run_agent_streaming(
    agent_id: &str,
    model: Option<&str>,
    prompt: &str,
    tag: Option<&str>,
) -> Result<(i32, String), String> {
    let binary = find_binary(agent_id)
        .ok_or_else(|| format!("Binary for '{agent_id}' not found in PATH or standard agent locations"))?;

    let mut cmd = Command::new(&binary);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    match agent_id {
        "agy" => {
            if let Some(m) = model {
                cmd.arg("--model").arg(m);
            }
            cmd.arg("-p").arg(prompt);
        }
        "codex" => {
            cmd.arg("exec");
            if let Some(m) = model {
                cmd.arg("-m").arg(m);
            }
            cmd.arg(prompt);
        }
        "claude" => {
            cmd.arg("-p");
            if let Some(m) = model {
                cmd.arg("--model").arg(m);
            }
            cmd.arg(prompt);
        }
        "kilo" | "kilocode" => {
            cmd.arg("run");
            if let Some(m) = model {
                cmd.arg("-m").arg(m);
            }
            cmd.arg(prompt);
        }
        "copilot" => {
            cmd.arg("-p").arg(prompt);
        }
        "gemini" => {
            cmd.arg("-p").arg(prompt);
            if let Some(m) = model {
                cmd.arg("-m").arg(m);
            }
        }
        "cline" => {
            cmd.arg(prompt);
        }
        "goose" => {
            cmd.arg("run").arg(prompt);
        }
        "aider" => {
            cmd.arg("--message").arg(prompt);
            if let Some(m) = model {
                cmd.arg("--model").arg(m);
            }
        }
        "cursor" | "continue" | "opencode" | "plandex" | "devin" | "amazon-q" => {
            cmd.arg(prompt);
        }
        _ => {
            cmd.arg(prompt);
        }
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn {agent_id}: {e}"))?;

    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let prefix = tag.map(|t| format!("\x1b[36m[{t}]\x1b[0m ")).unwrap_or_default();
    let mut accumulated = String::new();

    loop {
        tokio::select! {
            res = stdout_reader.next_line() => {
                match res {
                    Ok(Some(line)) => {
                        println!("{prefix}{line}");
                        let _ = io::stdout().flush();
                        accumulated.push_str(&line);
                        accumulated.push('\n');
                    }
                    Ok(None) => break,
                    Err(e) => {
                        eprintln!("Error streaming stdout: {e}");
                        break;
                    }
                }
            }
            res = stderr_reader.next_line() => {
                match res {
                    Ok(Some(line)) => {
                        eprintln!("\x1b[33m{prefix}{line}\x1b[0m");
                        let _ = io::stderr().flush();
                        accumulated.push_str(&line);
                        accumulated.push('\n');
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Wait failed on {agent_id}: {e}"))?;

    Ok((status.code().unwrap_or(0), accumulated))
}
