//! Bridge subsystem for Nio.
//!
//! Connects and streams coding agents (agy, codex, claude, opencode) with zero-loss
//! context handoff powered by NioDB session ledgers and manifests.

use crate::agent_runner::{
    detect_agents, format_handoff_prompt, run_agent_interactive, run_agent_streaming,
};
use crate::niodb::NioDbClient;
use crate::Options;
use std::io::{self, IsTerminal, Write};

pub const DEFAULT_MODELS: &[(&str, &str)] = &[
    ("claude-3-7-sonnet", "Claude 3.7 Sonnet (Anthropic - Recommended)"),
    ("gemini-2.5-pro", "Gemini 2.5 Pro (Google DeepMind)"),
    ("gpt-4o", "GPT-4o (OpenAI)"),
    ("deepseek-r1", "DeepSeek R1 (Reasoning)"),
    ("qwen-2.5-coder", "Qwen 2.5 Coder (Open Weights)"),
];

pub async fn command(options: &Options) -> Result<(), String> {
    let db = NioDbClient::new();
    let is_tty = io::stdin().is_terminal() && io::stdout().is_terminal();

    let args = &options.prompt;
    let sub = args.first().map(String::as_str);

    // Parse options from args if any
    let mut flag_agent = None;
    let mut flag_model = options.model.clone();
    let mut flag_goal = None;
    let mut flag_to = None;
    let mut flag_reason = None;
    let mut flag_issue = None;
    let mut flag_attempt = None;
    let mut flag_why = None;

    let mut i = if sub.is_some() && !sub.unwrap().starts_with('-') { 1 } else { 0 };
    while i < args.len() {
        let arg = &args[i];
        if arg == "--agent" && i + 1 < args.len() {
            flag_agent = Some(args[i + 1].clone());
            i += 2;
        } else if (arg == "--model" || arg == "-m") && i + 1 < args.len() {
            flag_model = Some(args[i + 1].clone());
            i += 2;
        } else if (arg == "--goal" || arg == "--prompt") && i + 1 < args.len() {
            flag_goal = Some(args[i + 1].clone());
            i += 2;
        } else if arg == "--to" && i + 1 < args.len() {
            flag_to = Some(args[i + 1].clone());
            i += 2;
        } else if arg == "--reason" && i + 1 < args.len() {
            flag_reason = Some(args[i + 1].clone());
            i += 2;
        } else if arg == "--issue" && i + 1 < args.len() {
            flag_issue = Some(args[i + 1].clone());
            i += 2;
        } else if arg == "--attempt" && i + 1 < args.len() {
            flag_attempt = Some(args[i + 1].clone());
            i += 2;
        } else if arg == "--why" && i + 1 < args.len() {
            flag_why = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }

    match sub {
        None | Some("interactive") if flag_agent.is_none() && flag_goal.is_none() => {
            if is_tty {
                run_empty_state_interactive(&db, flag_model.as_deref()).await
            } else {
                run_empty_state_display(&db).await
            }
        }
        Some("run") => {
            let agent = flag_agent.unwrap_or_else(|| "agy".to_string());
            let model = flag_model.as_deref();
            let goal = flag_goal.as_deref();
            run_bridge_agent(&db, &agent, model, goal, None).await
        }
        None => {
            let agent = flag_agent.unwrap_or_else(|| "agy".to_string());
            let model = flag_model.as_deref();
            let goal = flag_goal.as_deref();
            run_bridge_agent(&db, &agent, model, goal, None).await
        }
        Some("switch") => {
            let session_id = args.get(1).map(String::as_str).ok_or_else(|| {
                "Usage: nio bridge switch <session_id> --to <agent> [--model <model>]".to_string()
            })?;
            let to_agent = flag_to.ok_or_else(|| {
                "Missing --to <agent>. Usage: nio bridge switch <session_id> --to <agent>".to_string()
            })?;
            switch_bridge_agent(&db, session_id, &to_agent, flag_model.as_deref(), flag_reason.as_deref()).await
        }
        Some("manifest") => {
            let session_id = args.get(1).map(String::as_str).ok_or_else(|| {
                "Usage: nio bridge manifest <session_id>".to_string()
            })?;
            show_manifest(&db, session_id).await
        }
        Some("sessions") | Some("list") => {
            list_bridge_sessions(&db).await
        }
        Some("dead-end") => {
            let session_id = args.get(1).map(String::as_str).ok_or_else(|| {
                "Usage: nio bridge dead-end <session_id> --issue <text> --attempt <text> --why <text>".to_string()
            })?;
            let issue = flag_issue.unwrap_or_else(|| "Unknown issue".to_string());
            let attempt = flag_attempt.unwrap_or_else(|| "Attempted solution".to_string());
            let why = flag_why.unwrap_or_else(|| "Failed".to_string());
            record_dead_end(&db, session_id, &issue, &attempt, &why).await
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_bridge_help();
            Ok(())
        }
        Some(other) => {
            Err(format!("Unknown bridge command '{other}'. Run 'nio bridge --help'."))
        }
    }
}

pub fn print_bridge_help() {
    println!(
        r#"Nio Bridge — Stream coding agents with zero-loss context handoff via NioDB

Usage:
  nio bridge [COMMAND] [OPTIONS]

Commands:
  (no command)                 Open interactive empty state and model selector
  run                          Launch an agent bridge session
    --agent <agent>              Target agent: agy, codex, claude, opencode (default: agy)
    --model <model>              Model name (e.g. claude-3-7-sonnet, gemini-2.5-pro, gpt-4o)
    --goal <prompt>              Task prompt to execute (or interactive if omitted)

  switch <session_id>          Handoff session context and switch to another agent
    --to <agent>                 Target agent to switch to
    --model <model>              Model name for the new agent
    --reason <text>              Optional reason for switching

  manifest <session_id>        Display compiled zero-loss handoff manifest
  sessions                     List active bridge sessions in NioDB
  dead-end <session_id>        Record a failed path to avoid repeating mistakes
    --issue <issue>              What problem was encountered
    --attempt <attempt>          What was tried
    --why <why>                  Why it failed

Examples:
  nio bridge
  nio bridge run --agent agy --model claude-3-7-sonnet --goal "Fix compilation error in src/api.rs"
  nio bridge switch sess_123 --to codex --model gpt-4o
"#
    );
}

async fn run_empty_state_display(db: &NioDbClient) -> Result<(), String> {
    let agents = detect_agents();
    let db_ok = db.is_healthy().await;
    let mut sessions = Vec::new();
    if db_ok {
        if let Ok(list) = db.list_sessions().await {
            sessions = list
                .into_iter()
                .filter(|s| {
                    let st = s.get("swarm_type").and_then(|v| v.as_str());
                    st != Some("assemble")
                })
                .collect();
        }
    }

    println!(
        r#"
  ╭────────────────────────────────────────────────────────────────────────╮
  │ 🌉 Nio Bridge — Multi-Agent Seamless Context Conduit                   │
  │ Stream coding agents with zero-loss context handoff via NioDB         │
  ╰────────────────────────────────────────────────────────────────────────╯"#
    );

    println!("\n  \x1b[1mDetected AI Coding Agents:\x1b[0m");
    if agents.is_empty() {
        println!("    \x1b[31m✕ No external coding agent binaries detected.\x1b[0m");
    } else {
        for a in &agents {
            println!("    \x1b[32m●\x1b[0m \x1b[1m{:<8}\x1b[0m : {} ({})", a.id, a.name, a.binary_path.display());
        }
    }

    println!("\n  \x1b[1mNioDB Backend:\x1b[0m");
    if db_ok {
        println!("    \x1b[32m●\x1b[0m Status   : Connected ({})", db.base_url);
    } else {
        println!("    \x1b[33m▲\x1b[0m Status   : Local storage fallback ({})", db.base_url);
    }
    println!("    ● Sessions : {} active bridge sessions (Empty State)", sessions.len());

    println!("\n  \x1b[1mAvailable Model Strategies:\x1b[0m");
    for (idx, (m_id, m_name)) in DEFAULT_MODELS.iter().enumerate() {
        let is_rec = if idx == 0 { " (Default / Recommended)" } else { "" };
        println!("    [{}] {:<20} - {}{}", idx + 1, m_id, m_name, is_rec);
    }

    println!(
        r#"
  \x1b[1mUsage:\x1b[0m
    Interactive : Run in a terminal with 'nio bridge'
    Non-TTY     : nio bridge run --agent <agent> --model <model> --goal <task>
    Switch      : nio bridge switch <session_id> --to <agent> --model <model>
"#
    );
    Ok(())
}

async fn run_empty_state_interactive(db: &NioDbClient, preselected_model: Option<&str>) -> Result<(), String> {
    let agents = detect_agents();
    let db_ok = db.is_healthy().await;
    let mut sessions = Vec::new();
    if db_ok {
        if let Ok(list) = db.list_sessions().await {
            sessions = list
                .into_iter()
                .filter(|s| {
                    let st = s.get("swarm_type").and_then(|v| v.as_str());
                    st != Some("assemble")
                })
                .collect();
        }
    }

    println!(
        r#"
  ╭────────────────────────────────────────────────────────────────────────╮
  │ 🌉 Nio Bridge — Multi-Agent Seamless Context Conduit                   │
  │ Stream coding agents with zero-loss context handoff via NioDB         │
  ╰────────────────────────────────────────────────────────────────────────╯"#
    );

    println!("\n  \x1b[1mDetected AI Coding Agents:\x1b[0m");
    if agents.is_empty() {
        println!("    \x1b[31m✕ No external coding agent binaries detected.\x1b[0m");
    } else {
        for a in &agents {
            println!("    \x1b[32m●\x1b[0m \x1b[1m{:<8}\x1b[0m : {} ({})", a.id, a.name, a.binary_path.display());
        }
    }

    println!("\n  \x1b[1mNioDB Backend:\x1b[0m");
    if db_ok {
        println!("    \x1b[32m●\x1b[0m Status   : Connected ({})", db.base_url);
    } else {
        println!("    \x1b[33m▲\x1b[0m Status   : Local storage fallback ({})", db.base_url);
    }
    println!("    ● Sessions : {} active bridge sessions", sessions.len());

    if sessions.is_empty() {
        println!("    \x1b[2m(Empty State: Ready to bridge your first agent)\x1b[0m");
    }

    println!("\n  \x1b[1mActions:\x1b[0m");
    println!("    [1] Start new agent bridge session");
    if !sessions.len() == 0 {
        println!("    [2] Switch agent in existing session");
        println!("    [3] Inspect bridge sessions & manifests");
    }
    println!("    [q] Quit\n");

    print!("  Select option [1]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).map_err(|e| e.to_string())?;
    let choice = choice.trim();

    if choice == "q" || choice == "Q" {
        return Ok(());
    }

    if choice == "2" && !sessions.is_empty() {
        return prompt_switch_session(db).await;
    }

    if choice == "3" && !sessions.is_empty() {
        return list_bridge_sessions(db).await;
    }

    // Default or [1]: Start new agent bridge session
    // Step 1: Agent selection
    println!("\n  \x1b[1m1. Select Coding Agent to Stream:\x1b[0m");
    let available_ids: Vec<String> = if !agents.is_empty() {
        agents.iter().map(|a| a.id.clone()).collect()
    } else {
        vec!["agy".into(), "codex".into(), "claude".into(), "opencode".into()]
    };

    for (idx, id) in available_ids.iter().enumerate() {
        let is_def = if idx == 0 { " (Default)" } else { "" };
        println!("    [{}] {}{}", idx + 1, id, is_def);
    }
    print!("  Select agent [1]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut agent_choice = String::new();
    io::stdin().read_line(&mut agent_choice).map_err(|e| e.to_string())?;
    let agent_idx = agent_choice.trim().parse::<usize>().unwrap_or(1);
    let selected_agent = available_ids
        .get(agent_idx.saturating_sub(1))
        .cloned()
        .unwrap_or_else(|| "agy".to_string());

    // Step 2: Model selection (Empty state requirement!)
    let selected_model = if let Some(m) = preselected_model {
        m.to_string()
    } else {
        println!("\n  \x1b[1m2. Select Model Strategy for {}:\x1b[0m", selected_agent);
        for (idx, (m_id, m_name)) in DEFAULT_MODELS.iter().enumerate() {
            let is_rec = if idx == 0 { " (Default)" } else { "" };
            println!("    [{}] {:<20} - {}{}", idx + 1, m_id, m_name, is_rec);
        }
        println!("    [{}] Custom model name...", DEFAULT_MODELS.len() + 1);
        print!("  Select model [1]: ");
        io::stdout().flush().map_err(|e| e.to_string())?;

        let mut model_choice = String::new();
        io::stdin().read_line(&mut model_choice).map_err(|e| e.to_string())?;
        let model_idx = model_choice.trim().parse::<usize>().unwrap_or(1);

        if model_idx == DEFAULT_MODELS.len() + 1 {
            print!("  Enter custom model: ");
            io::stdout().flush().map_err(|e| e.to_string())?;
            let mut custom = String::new();
            io::stdin().read_line(&mut custom).map_err(|e| e.to_string())?;
            let custom = custom.trim().to_string();
            if custom.is_empty() {
                DEFAULT_MODELS[0].0.to_string()
            } else {
                custom
            }
        } else {
            DEFAULT_MODELS
                .get(model_idx.saturating_sub(1))
                .map(|(id, _)| id.to_string())
                .unwrap_or_else(|| DEFAULT_MODELS[0].0.to_string())
        }
    };

    // Step 3: Objective / Goal
    println!("\n  \x1b[1m3. Task Objective:\x1b[0m");
    println!("  Enter goal for the agent (press Enter to launch full interactive session):");
    print!("  > ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut goal = String::new();
    io::stdin().read_line(&mut goal).map_err(|e| e.to_string())?;
    let goal = goal.trim();
    let goal_opt = if goal.is_empty() { None } else { Some(goal) };

    println!("\n\x1b[32m✦ Launching Bridge session with {} [{}]...\x1b[0m", selected_agent, selected_model);
    run_bridge_agent(db, &selected_agent, Some(&selected_model), goal_opt, None).await
}

async fn run_bridge_agent(
    db: &NioDbClient,
    agent: &str,
    model: Option<&str>,
    goal: Option<&str>,
    existing_session_id: Option<&str>,
) -> Result<(), String> {
    let session_id = if let Some(sid) = existing_session_id {
        sid.to_string()
    } else {
        let title = format!("Bridge {} session", agent);
        match db.create_session(&title, agent, model, Some("bridge"), goal, None).await {
            Ok(v) => v.get("id").and_then(|id| id.as_str()).unwrap_or("bridge_active").to_string(),
            Err(e) => {
                eprintln!("\x1b[33mNotice: NioDB session creation ({e}), continuing in standalone mode.\x1b[0m");
                "bridge_standalone".to_string()
            }
        }
    };

    println!("  \x1b[2mSession ID : {}\x1b[0m", session_id);
    println!("  \x1b[2mAgent      : {}\x1b[0m", agent);
    if let Some(m) = model {
        println!("  \x1b[2mModel      : {}\x1b[0m", m);
    }
    println!("  \x1b[2m════════════════════════════════════════════════════════════════\x1b[0m\n");

    if let Some(task_goal) = goal {
        // Fetch handoff manifest if session exists
        let manifest_val = db.get_manifest(&session_id).await.ok();
        let prompt_with_context = format_handoff_prompt(task_goal, manifest_val.as_ref());

        let tag = format!("{agent}");
        let (exit_code, output) = run_agent_streaming(agent, model, &prompt_with_context, Some(&tag)).await?;

        println!("\n  \x1b[2m════════════════════════════════════════════════════════════════\x1b[0m");
        if exit_code == 0 {
            println!("  \x1b[32m✓ Turn completed successfully.\x1b[0m");
            // Sync turn to NioDB
            let summary = format!("Executed task: {task_goal}");
            let touched = extract_touched_files(&output);
            if let Err(e) = db.append_turn(&session_id, agent, model, &summary, &touched).await {
                eprintln!("\x1b[33mNotice: failed to record turn in NioDB: {e}\x1b[0m");
            } else {
                println!("  \x1b[32m✓ Handoff ledger updated in NioDB. Ready for zero-loss switch.\x1b[0m");
            }
        } else {
            println!("  \x1b[31m✕ Agent exited with code {exit_code}.\x1b[0m");
        }
    } else {
        // Full interactive TTY
        let code = run_agent_interactive(agent, model, None)?;
        let summary = format!("Interactive session with {agent}");
        let _ = db.append_turn(&session_id, agent, model, &summary, &[]).await;
        println!("\n  \x1b[32m✓ Bridge session recorded ({session_id}).\x1b[0m");
        if code != 0 {
            eprintln!("  \x1b[33mAgent exited with status {code}\x1b[0m");
        }
    }

    println!("  \x1b[2mTip: Switch agents anytime with: nio bridge switch {} --to <other_agent>\x1b[0m\n", session_id);
    Ok(())
}

async fn switch_bridge_agent(
    db: &NioDbClient,
    session_id: &str,
    to_agent: &str,
    model: Option<&str>,
    reason: Option<&str>,
) -> Result<(), String> {
    println!("\n  \x1b[1m✦ Initiating Zero-Loss Context Switch to {}...\x1b[0m", to_agent);

    // 1. Fetch manifest
    let manifest = db.get_manifest(session_id).await.map_err(|e| {
        format!("Failed to retrieve context manifest for session {session_id}: {e}")
    })?;

    // 2. Perform switch in NioDB
    let _switch_res = db.switch_agent(session_id, to_agent, model, reason).await.map_err(|e| {
        format!("Failed to record agent switch in NioDB: {e}")
    })?;

    let prev_agent = manifest.get("current_agent").and_then(|v| v.as_str()).unwrap_or("previous agent");
    let turns = manifest.get("recent_turns").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let files = manifest.get("files_touched").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let dead_ends = manifest.get("known_dead_ends").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);

    println!(
        r#"
  ╭────────────────────────────────────────────────────────────────────────╮
  │ 🔄 Context Conduit Handoff Package Prepared                             │
  │ • From Agent   : {:<53} │
  │ • To Agent     : {:<53} │
  │ • Turns Kept   : {:<53} │
  │ • Files Tracked: {:<53} │
  │ • Dead-Ends    : {:<53} │
  ╰────────────────────────────────────────────────────────────────────────╯"#,
        prev_agent,
        to_agent,
        format!("{} turns preserved", turns),
        format!("{} files in context", files),
        format!("{} dead-ends avoided", dead_ends)
    );

    let goal = manifest.get("goal").and_then(|v| v.as_str()).unwrap_or("Continue session tasks");
    let handoff_prompt = format_handoff_prompt(goal, Some(&manifest));

    println!("\n  \x1b[32m⚡ Streaming {} with handoff context...\x1b[0m\n", to_agent);
    let tag = format!("{to_agent}");
    let (code, output) = run_agent_streaming(to_agent, model, &handoff_prompt, Some(&tag)).await?;

    let summary = format!("Switched from {prev_agent} to {to_agent} and continued");
    let touched = extract_touched_files(&output);
    let _ = db.append_turn(session_id, to_agent, model, &summary, &touched).await;

    if code == 0 {
        println!("\n  \x1b[32m✓ Handoff turn completed successfully.\x1b[0m\n");
    } else {
        println!("\n  \x1b[33m▲ Agent finished with code {code}.\x1b[0m\n");
    }

    Ok(())
}

async fn show_manifest(db: &NioDbClient, session_id: &str) -> Result<(), String> {
    let manifest = db.get_manifest(session_id).await?;
    println!("{}", serde_json::to_string_pretty(&manifest).unwrap_or_default());
    Ok(())
}

async fn list_bridge_sessions(db: &NioDbClient) -> Result<(), String> {
    let sessions = db.list_sessions().await?;
    println!("\n  \x1b[1mActive Nio Bridge Sessions:\x1b[0m");
    if sessions.is_empty() {
        println!("    (No bridge sessions found. Run 'nio bridge' to start one.)\n");
        return Ok(());
    }

    for s in sessions {
        let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let agent = s.get("agent").and_then(|v| v.as_str()).unwrap_or("");
        let model = s.get("model").and_then(|v| v.as_str()).unwrap_or("");
        println!("    • \x1b[1m{id}\x1b[0m : {title} [Agent: {agent}, Model: {model}]");
    }
    println!();
    Ok(())
}

async fn record_dead_end(
    db: &NioDbClient,
    session_id: &str,
    issue: &str,
    attempt: &str,
    why: &str,
) -> Result<(), String> {
    let _res = db.add_dead_end(session_id, issue, attempt, why).await?;
    println!("  \x1b[32m✓ Dead-end recorded for {session_id}.\x1b[0m");
    println!("    Issue   : {issue}");
    println!("    Attempt : {attempt}");
    println!("    Reason  : {why}");
    Ok(())
}

async fn prompt_switch_session(db: &NioDbClient) -> Result<(), String> {
    let sessions = db.list_sessions().await?;
    if sessions.is_empty() {
        println!("  No active sessions to switch.");
        return Ok(());
    }

    println!("\n  Select session to switch:");
    for (idx, s) in sessions.iter().enumerate() {
        let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let agent = s.get("agent").and_then(|v| v.as_str()).unwrap_or("");
        println!("    [{}] {} (Current agent: {})", idx + 1, id, agent);
    }
    print!("  Select session [1]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).map_err(|e| e.to_string())?;
    let idx = choice.trim().parse::<usize>().unwrap_or(1);
    let selected_session = sessions.get(idx.saturating_sub(1)).and_then(|s| s.get("id")).and_then(|v| v.as_str()).unwrap_or("");

    println!("\n  Switch to which agent?");
    println!("    [1] agy");
    println!("    [2] codex");
    println!("    [3] claude");
    println!("    [4] opencode");
    print!("  Select target agent: ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut ag_choice = String::new();
    io::stdin().read_line(&mut ag_choice).map_err(|e| e.to_string())?;
    let target_agent = match ag_choice.trim() {
        "2" => "codex",
        "3" => "claude",
        "4" => "opencode",
        _ => "agy",
    };

    switch_bridge_agent(db, selected_session, target_agent, None, None).await
}

fn extract_touched_files(output: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Created ") || trimmed.starts_with("Updated ") || trimmed.starts_with("Modified ") {
            if let Some(path) = trimmed.split_whitespace().nth(1) {
                files.push(path.trim_matches('`').to_string());
            }
        }
    }
    files.dedup();
    files
}
