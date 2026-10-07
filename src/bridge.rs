//! Bridge subsystem for Nio.
//!
//! Connects and streams coding agents (agy, codex, claude, opencode) with zero-loss
//! context handoff powered by NioDB session ledgers and manifests.

use crate::Options;
use crate::agent_runner::{
    detect_agents, format_handoff_prompt, run_agent_interactive, run_agent_streaming,
};
use crate::niodb::NioDbClient;
use serde_json::{Value, json};
use std::io::{self, IsTerminal};
use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub async fn command(options: &Options) -> Result<(), String> {
    let db = NioDbClient::new();
    let _ = db.ensure_healthy().await;
    sync_local_bridge_sessions_to_db(&db).await;
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

    let mut i = if sub.is_some() && !sub.unwrap().starts_with('-') {
        1
    } else {
        0
    };
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
            run_bridge_agent(&db, &agent, model, goal, None, false).await
        }
        None => {
            let agent = flag_agent.unwrap_or_else(|| "agy".to_string());
            let model = flag_model.as_deref();
            let goal = flag_goal.as_deref();
            run_bridge_agent(&db, &agent, model, goal, None, false).await
        }

        Some("resume") => {
            let session_id = args.get(1).map(String::as_str);
            if let Some(id) = session_id {
                let agent = flag_agent.unwrap_or_else(|| "agy".to_string());
                let model = flag_model.as_deref();
                let goal = flag_goal.as_deref();
                run_bridge_agent(&db, &agent, model, goal, Some(id), false).await
            } else {
                prompt_resume_session(&db).await
            }
        }
        Some("switch") => {
            let session_id = args.get(1).map(String::as_str).ok_or_else(|| {
                "Usage: nio bridge switch <session_id> --to <agent> [--model <model>]".to_string()
            })?;
            let to_agent = flag_to.ok_or_else(|| {
                "Missing --to <agent>. Usage: nio bridge switch <session_id> --to <agent>"
                    .to_string()
            })?;
            let reason = flag_reason.or(flag_goal);
            switch_bridge_agent(
                &db,
                session_id,
                &to_agent,
                flag_model.as_deref(),
                reason.as_deref(),
            )
            .await
        }
        Some("manifest") => {
            let session_id = args
                .get(1)
                .map(String::as_str)
                .ok_or_else(|| "Usage: nio bridge manifest <session_id>".to_string())?;
            show_manifest(&db, session_id).await
        }
        Some("sessions") | Some("list") => list_bridge_sessions(&db).await,
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
        Some(other) => Err(format!(
            "Unknown bridge command '{other}'. Run 'nio bridge --help'."
        )),
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

  resume [session_id]          Resume a bridge session (select one if omitted)
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
    let sessions = list_bridge_records(db).await.unwrap_or_default();

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
            println!(
                "    \x1b[32m●\x1b[0m \x1b[1m{:<8}\x1b[0m : {} ({})",
                a.id,
                a.name,
                a.binary_path.display()
            );
        }
    }

    println!("\n  \x1b[1mNioDB Backend:\x1b[0m");
    if db_ok {
        println!(
            "    \x1b[32m●\x1b[0m Status   : Connected ({})",
            db.base_url
        );
    } else {
        println!(
            "    \x1b[33m▲\x1b[0m Status   : Local storage fallback ({})",
            db.base_url
        );
    }
    println!(
        "    ● Sessions : {} active bridge sessions (Empty State)",
        sessions.len()
    );

    println!("\n  Each coding agent uses its own configured model by default.");

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

async fn run_empty_state_interactive(
    db: &NioDbClient,
    preselected_model: Option<&str>,
) -> Result<(), String> {
    let agents = detect_agents();
    let db_ok = db.is_healthy().await;
    let sessions = list_bridge_records(db).await.unwrap_or_default();

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
            println!(
                "    \x1b[32m●\x1b[0m \x1b[1m{:<8}\x1b[0m : {} ({})",
                a.id,
                a.name,
                a.binary_path.display()
            );
        }
    }

    println!("\n  \x1b[1mNioDB Backend:\x1b[0m");
    if db_ok {
        println!(
            "    \x1b[32m●\x1b[0m Status   : Connected ({})",
            db.base_url
        );
    } else {
        println!(
            "    \x1b[33m▲\x1b[0m Status   : Local storage fallback ({})",
            db.base_url
        );
    }
    println!("    ● Sessions : {} active bridge sessions", sessions.len());

    if sessions.is_empty() {
        println!("    \x1b[2m(Empty State: Ready to bridge your first agent)\x1b[0m");
    }

    if !sessions.is_empty() {
        let mut actions = vec!["Start new agent bridge session".to_string()];
        actions.push("Resume existing bridge session".to_string());
        actions.push("Switch agent in bridge session".to_string());
        actions.push("Inspect bridge sessions & manifests".to_string());
        actions.push("Quit".to_string());
        let menu_items: Vec<(&str, &str, bool)> = actions
            .iter()
            .map(|action| (action.as_str(), "", false))
            .collect();
        let Some(choice) = crate::select_menu_option_b("Actions", &menu_items, 0)? else {
            return Ok(());
        };
        match choice {
            1 => return prompt_resume_session(db).await,
            2 => return prompt_switch_session(db).await,
            3 => return list_bridge_sessions(db).await,
            4 => return Ok(()),
            _ => {}
        }
    }

    // Start with agent selection when there are no existing sessions.
    let available_ids: Vec<String> = if !agents.is_empty() {
        agents.iter().map(|a| a.id.clone()).collect()
    } else {
        vec![
            "agy".into(),
            "codex".into(),
            "claude".into(),
            "opencode".into(),
        ]
    };

    let agent_options: Vec<(&str, &str, bool)> = available_ids
        .iter()
        .enumerate()
        .map(|(idx, id)| (id.as_str(), "", idx == 0))
        .collect();
    let Some(agent_idx) = crate::select_menu_option_b("Coding Agent", &agent_options, 0)? else {
        return Ok(());
    };
    let selected_agent = available_ids
        .get(agent_idx)
        .cloned()
        .unwrap_or_else(|| "agy".to_string());

    let selected_model = preselected_model.map(str::to_string);

    let mut task_prompt = String::new();
    while task_prompt.trim().is_empty() {
        println!("\n  \x1b[1mSession Task\x1b[0m (Required)");
        println!(
            "  \x1b[2mThis will be used as the session title and your initial instruction to the agent.\x1b[0m"
        );
        task_prompt = crate::read_console_line("  > ")?;
        if task_prompt.trim().is_empty() {
            println!("  \x1b[31mTask is required to start a session.\x1b[0m");
        }
    }

    run_bridge_agent(
        db,
        &selected_agent,
        selected_model.as_deref(),
        Some(task_prompt.trim()),
        None,
        true,
    )
    .await
}

async fn run_bridge_agent(
    db: &NioDbClient,
    agent: &str,
    model: Option<&str>,
    goal: Option<&str>,
    existing_session_id: Option<&str>,
    interactive: bool,
) -> Result<(), String> {
    let session_id = if let Some(sid) = existing_session_id {
        sid.to_string()
    } else {
        let title_string = goal.unwrap_or("").to_string();
        let title = if title_string.is_empty() {
            format!("Bridge {} session", agent)
        } else {
            title_string
        };
        match create_bridge_session(db, &title, agent, model, goal).await {
            Ok(v) => v
                .get("id")
                .and_then(|id| id.as_str())
                .unwrap_or("bridge_active")
                .to_string(),
            Err(e) => {
                return Err(format!(
                    "Could not create bridge session in NioDB or local storage: {e}"
                ));
            }
        }
    };

    if goal.is_none() {
        println!(
            "\x1b[2mBridge session {session_id} · switch later with: nio bridge switch {session_id} --to <agent>\x1b[0m\n"
        );
    } else {
        println!("  \x1b[2mSession ID : {}\x1b[0m", session_id);
        println!("  \x1b[2mAgent      : {}\x1b[0m", agent);
        if let Some(m) = model {
            println!("  \x1b[2mModel      : {}\x1b[0m", m);
        }
        println!(
            "  \x1b[2m════════════════════════════════════════════════════════════════\x1b[0m\n"
        );
    }

    if goal.is_some() && !interactive {
        // Fetch handoff manifest if session exists
        let task_goal = goal.unwrap();
        let manifest_val = get_bridge_manifest(db, &session_id).await.ok();
        let prompt_with_context = format_handoff_prompt(task_goal, manifest_val.as_ref());

        let tag = format!("{agent}");
        let (exit_code, output) =
            run_agent_streaming(agent, model, &prompt_with_context, Some(&tag)).await?;

        println!(
            "\n  \x1b[2m════════════════════════════════════════════════════════════════\x1b[0m"
        );
        if exit_code == 0 {
            println!("  \x1b[32m✓ Turn completed successfully.\x1b[0m");
            // Sync turn to NioDB
            let summary = format!("Executed task: {task_goal}");
            let touched = extract_touched_files(&output);
            if let Err(e) =
                append_bridge_turn(db, &session_id, agent, model, &summary, None, &touched).await
            {
                eprintln!("\x1b[33mNotice: failed to record turn in NioDB: {e}\x1b[0m");
            } else {
                println!(
                    "  \x1b[32m✓ Handoff ledger updated in NioDB. Ready for zero-loss switch.\x1b[0m"
                );
            }
        } else {
            println!("  \x1b[31m✕ Agent exited with code {exit_code}.\x1b[0m");
        }
    } else {
        // Full interactive TTY
        let before_files = detect_git_modified_files();
        let start_time = SystemTime::now();
        let code = run_agent_interactive(agent, model, goal)?;
        println!("\n  \x1b[32m✓ Bridge session recorded ({session_id}).\x1b[0m");
        if code != 0 {
            eprintln!("  \x1b[33mAgent exited with status {code}\x1b[0m");
        }
        let assessment = extract_agent_session_assessment(agent, start_time);
        let summary = format!("Interactive session completed with {agent}");
        let after_files = detect_git_modified_files();
        let touched: Vec<String> = after_files
            .into_iter()
            .filter(|f| !before_files.contains(f))
            .collect();
        let _ = append_bridge_turn(
            db,
            &session_id,
            agent,
            model,
            &summary,
            assessment.as_deref(),
            &touched,
        )
        .await;
    }

    println!(
        "  \x1b[2mTip: Switch agents anytime with: nio bridge switch {} --to <other_agent>\x1b[0m\n",
        session_id
    );
    Ok(())
}

async fn switch_bridge_agent(
    db: &NioDbClient,
    session_id: &str,
    to_agent: &str,
    model: Option<&str>,
    reason: Option<&str>,
) -> Result<(), String> {
    println!(
        "\n  \x1b[1m✦ Initiating Zero-Loss Context Switch to {}...\x1b[0m",
        to_agent
    );
    let _ = db.ensure_healthy().await;

    // Read the old session state before switching its current agent. The manifest
    // supplies the source agent, objective, and turn history for the handoff.
    let manifest = get_bridge_manifest(db, session_id).await.map_err(|e| {
        format!("Failed to retrieve context manifest for session {session_id}: {e}")
    })?;

    let prev_agent = manifest
        .get("current_agent")
        .and_then(|v| v.as_str())
        .unwrap_or("previous agent");
    let turns = manifest
        .get("recent_turns")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let files = manifest
        .get("files_touched")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let dead_ends = manifest
        .get("known_dead_ends")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    println!(
        r#"
  ╭────────────────────────────────────────────────────────────────────────╮
  │ 🔄 Context Conduit Handoff Package Prepared                            │
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

    // Persist the target agent only after preserving the source manifest.
    switch_bridge_record(db, session_id, to_agent, model, reason)
        .await
        .map_err(|e| format!("Failed to record agent switch in NioDB: {e}"))?;

    println!(
        "\n  \x1b[32m✓ Handoff package prepared for {}. Ready for next run.\x1b[0m\n",
        to_agent
    );

    Ok(())
}

async fn show_manifest(db: &NioDbClient, session_id: &str) -> Result<(), String> {
    let manifest = get_bridge_manifest(db, session_id).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&manifest).unwrap_or_default()
    );
    Ok(())
}

fn normalize_session_value(mut s: Value) -> Value {
    if let Some(data) = s.get("data").and_then(Value::as_object).cloned() {
        if let Some(obj) = s.as_object_mut() {
            for (k, v) in data {
                if !obj.contains_key(&k) || obj.get(&k).map_or(true, |val| val.is_null()) {
                    obj.insert(k, v);
                }
            }
        }
    }
    s
}

fn extract_session_timestamp(session: &Value) -> u64 {
    let parse_iso = |iso: &str| -> Option<u64> {
        let parts: Vec<&str> = iso
            .split(|c| c == 'T' || c == 'Z' || c == '-' || c == ':' || c == '.')
            .collect();
        if parts.len() < 6 {
            return None;
        }
        let y: u64 = parts[0].parse().ok()?;
        let m: u64 = parts[1].parse().ok()?;
        let d: u64 = parts[2].parse().ok()?;
        let h: u64 = parts[3].parse().ok()?;
        let min: u64 = parts[4].parse().ok()?;
        let s: u64 = parts[5].parse().ok()?;
        let ms: u64 = if parts.len() > 6 && !parts[6].is_empty() {
            let ms_str = &parts[6][0..std::cmp::min(3, parts[6].len())];
            let mut val: u64 = ms_str.parse().ok()?;
            if ms_str.len() == 1 {
                val *= 100;
            } else if ms_str.len() == 2 {
                val *= 10;
            }
            val
        } else {
            0
        };

        let mut days = 0;
        for year in 1970..y {
            days += if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                366
            } else {
                365
            };
        }
        let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let is_leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        for month in 1..m {
            days += month_days[(month - 1) as usize];
            if month == 2 && is_leap {
                days += 1;
            }
        }
        days += d - 1;

        let total_s = days * 86400 + h * 3600 + min * 60 + s;
        Some(total_s * 1000 + ms)
    };

    if let Some(updated) = session.get("updated_at").and_then(Value::as_str) {
        if let Some(ts) = parse_iso(updated) {
            return ts;
        }
    }
    if let Some(created) = session.get("created_at").and_then(Value::as_str) {
        if let Some(ts) = parse_iso(created) {
            return ts;
        }
    }
    if let Some(id) = session.get("id").and_then(Value::as_str) {
        if id.starts_with("bridge_local_") {
            return id["bridge_local_".len()..].parse().unwrap_or(0);
        }
    }
    0
}

fn session_display_info(session: &Value) -> (String, String) {
    let s = normalize_session_value(session.clone());

    let goal = s.get("goal").and_then(Value::as_str).unwrap_or("").trim();
    let title = s.get("title").and_then(Value::as_str).unwrap_or("").trim();

    let generic_goal =
        goal.eq_ignore_ascii_case("switched agent") || goal.eq_ignore_ascii_case("unknown");
    let normalized_title = title.to_ascii_lowercase();
    let generic_title = normalized_title == "bridge session"
        || (normalized_title.starts_with("bridge ") && normalized_title.ends_with(" session"));

    let id = s.get("id").and_then(Value::as_str).unwrap_or("unknown_id");

    let primary_title = if !goal.is_empty() && !generic_goal {
        goal.to_string()
    } else if !title.is_empty() && !generic_title {
        title.to_string()
    } else {
        id.to_string()
    };

    let agent = s.get("agent").and_then(Value::as_str).unwrap_or("unknown");
    let model = s.get("model").and_then(Value::as_str).unwrap_or("");

    let agent_model = if !model.is_empty() && model != agent {
        format!("{agent} · {model}")
    } else {
        agent.to_string()
    };

    let ts_ms = extract_session_timestamp(&s);
    let time_str = if ts_ms == 0 {
        "".to_string()
    } else {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if ts_ms > now {
            " · just now".to_string()
        } else {
            let diff_s = (now - ts_ms) / 1000;
            if diff_s < 60 {
                " · just now".to_string()
            } else if diff_s < 3600 {
                format!(" · {}m ago", diff_s / 60)
            } else if diff_s < 86400 {
                format!(" · {}h ago", diff_s / 3600)
            } else {
                format!(" · {}d ago", diff_s / 86400)
            }
        }
    };

    let meta = format!("[{}{}]", agent_model, time_str);
    (primary_title, meta)
}

fn is_generic_turn_summary(summary: &str) -> bool {
    let summary = summary.trim().to_ascii_lowercase();
    summary == "switched agent"
        || summary.starts_with("switched from ")
        || summary.starts_with("interactive session completed with ")
        || summary.starts_with("handoff session completed with ")
        || summary.starts_with("handoff session executed with ")
        || summary.starts_with("handoff note:")
}

async fn list_bridge_sessions(db: &NioDbClient) -> Result<(), String> {
    let sessions = list_bridge_records(db).await?;
    println!("\n  \x1b[1mActive Nio Bridge Sessions:\x1b[0m");
    if sessions.is_empty() {
        println!("    (No bridge sessions found. Run 'nio bridge' to start one.)\n");
        return Ok(());
    }

    for s in sessions {
        let normalized = normalize_session_value(s);
        let id = normalized
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let (task, meta) = session_display_info(&normalized);
        println!("    • \x1b[1;36m{id}\x1b[0m : {task} \x1b[2m{meta}\x1b[0m");
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

async fn prompt_resume_session(db: &NioDbClient) -> Result<(), String> {
    let raw_sessions = list_bridge_records(db).await?;
    if raw_sessions.is_empty() {
        println!("  No active sessions to resume.");
        return Ok(());
    }

    let sessions: Vec<Value> = raw_sessions
        .into_iter()
        .map(normalize_session_value)
        .collect();
    let display_pairs: Vec<(String, String)> = sessions.iter().map(session_display_info).collect();
    let session_items: Vec<(&str, &str, bool)> = display_pairs
        .iter()
        .map(|(task, meta)| (task.as_str(), meta.as_str(), false))
        .collect();

    let Some(idx) =
        crate::select_menu_option_b("Select Bridge Session to Resume", &session_items, 0)?
    else {
        return Ok(());
    };
    let session = &sessions[idx];
    let selected_session = session.get("id").and_then(Value::as_str).unwrap_or("");
    let agent = session
        .get("agent")
        .and_then(Value::as_str)
        .unwrap_or("agy");
    let prev_model = session.get("model").and_then(Value::as_str);

    let detected = crate::agent_runner::detect_agents();
    let target_agents: Vec<String> = if detected.is_empty() {
        vec![
            "agy".into(),
            "codex".into(),
            "claude".into(),
            "opencode".into(),
        ]
    } else {
        detected.iter().map(|a| a.id.clone()).collect()
    };

    let initial_idx = target_agents.iter().position(|a| a == agent).unwrap_or(0);
    let target_items: Vec<(&str, &str, bool)> = target_agents
        .iter()
        .map(|a| (a.as_str(), "", false))
        .collect();

    let Some(agent_idx) =
        crate::select_menu_option_b("Resume with Agent", &target_items, initial_idx)?
    else {
        return Ok(());
    };
    let selected_agent = &target_agents[agent_idx];
    let model = if selected_agent == agent {
        prev_model
    } else {
        None
    };

    // Re-fetch manifest to build the handoff prompt if needed.
    let manifest = get_bridge_manifest(db, selected_session)
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
    let prev_goal = manifest
        .get("goal")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|g| {
            !g.is_empty() && !is_generic_turn_summary(g) && !g.eq_ignore_ascii_case("unknown")
        })
        .unwrap_or("");

    let mut handoff_prompt = crate::agent_runner::format_handoff_prompt(prev_goal, Some(&manifest));

    handoff_prompt.push_str("\n\n[SYSTEM INSTRUCTION]\n");
    handoff_prompt.push_str("This is a resumed session. Please briefly acknowledge this context in 1 sentence and wait for the user's next command. DO NOT begin working on the objective yet.");

    println!(
        "\n  \x1b[32m⚡ Resuming session with {} interactively...\x1b[0m\n",
        selected_agent
    );
    let before_files = detect_git_modified_files();
    let start_time = SystemTime::now();
    // Auto-switch if the user chose a different agent than the one active in the session
    if selected_agent != agent {
        let _ = db
            .switch_agent_full(
                selected_session,
                selected_agent,
                model,
                Some(&format!(
                    "Resumed session and switched to {}",
                    selected_agent
                )),
                None,
            )
            .await;
    }
    let code = crate::agent_runner::run_agent_interactive(
        selected_agent,
        model,
        Some(handoff_prompt.as_str()),
    )?;

    let assessment = extract_agent_session_assessment(selected_agent, start_time);
    let summary = format!("Interactive session resumed with {selected_agent}");
    let after_files = detect_git_modified_files();
    let touched: Vec<String> = after_files
        .into_iter()
        .filter(|f| !before_files.contains(f))
        .collect();
    let _ = append_bridge_turn(
        db,
        selected_session,
        selected_agent,
        model,
        &summary,
        assessment.as_deref(),
        &touched,
    )
    .await;

    if code != 0 {
        eprintln!("  \x1b[33mAgent exited with status {code}\x1b[0m");
    }

    Ok(())
}

async fn prompt_switch_session(db: &NioDbClient) -> Result<(), String> {
    let raw_sessions = list_bridge_records(db).await?;
    if raw_sessions.is_empty() {
        println!("  No active sessions to switch.");
        return Ok(());
    }

    let sessions: Vec<Value> = raw_sessions
        .into_iter()
        .map(normalize_session_value)
        .collect();
    let display_pairs: Vec<(String, String)> = sessions.iter().map(session_display_info).collect();
    let session_items: Vec<(&str, &str, bool)> = display_pairs
        .iter()
        .map(|(task, meta)| (task.as_str(), meta.as_str(), false))
        .collect();

    let Some(idx) =
        crate::select_menu_option_b("Select Bridge Session to Switch", &session_items, 0)?
    else {
        return Ok(());
    };
    let selected_session = sessions[idx]
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("");

    let detected = detect_agents();
    let target_agents: Vec<String> = if detected.is_empty() {
        vec![
            "agy".into(),
            "codex".into(),
            "claude".into(),
            "opencode".into(),
        ]
    } else {
        detected.iter().map(|agent| agent.id.clone()).collect()
    };
    let target_items: Vec<(&str, &str, bool)> = target_agents
        .iter()
        .map(|agent| (agent.as_str(), "", false))
        .collect();
    let Some(agent_idx) = crate::select_menu_option_b("Switch To Agent", &target_items, 0)? else {
        return Ok(());
    };
    switch_bridge_agent(db, selected_session, &target_agents[agent_idx], None, None).await
}

fn extract_touched_files(output: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Created ")
            || trimmed.starts_with("Updated ")
            || trimmed.starts_with("Modified ")
        {
            if let Some(path) = trimmed.split_whitespace().nth(1) {
                files.push(path.trim_matches('`').to_string());
            }
        }
    }
    files.dedup();
    files
}

fn local_bridge_sessions_path() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".nio")
        .join("bridge-sessions.json"))
}

fn read_local_bridge_sessions() -> Result<Vec<Value>, String> {
    let path = local_bridge_sessions_path()?;
    match fs::read(&path) {
        Ok(data) => {
            serde_json::from_slice(&data).map_err(|e| format!("reading local bridge sessions: {e}"))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("reading {}: {e}", path.display())),
    }
}

fn write_local_bridge_sessions(sessions: &[Value]) -> Result<(), String> {
    let path = local_bridge_sessions_path()?;
    let parent = path.parent().ok_or("invalid local bridge session path")?;
    fs::create_dir_all(parent).map_err(|e| format!("creating {}: {e}", parent.display()))?;
    let data = serde_json::to_vec_pretty(sessions).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| format!("writing {}: {e}", path.display()))
}

fn put_local_bridge_session(mut session: Value) -> Result<Value, String> {
    let mut sessions = read_local_bridge_sessions()?;
    let id = session
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if id.is_empty() {
        return Err("session response did not contain an id".into());
    }
    if session.get("swarm_type").is_none() {
        session["swarm_type"] = json!("bridge");
    }
    if session.get("turns").is_none() {
        session["turns"] = json!([]);
    }
    if let Some(existing) = sessions
        .iter_mut()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(&id))
    {
        *existing = session.clone();
    } else {
        sessions.push(session.clone());
    }
    write_local_bridge_sessions(&sessions)?;
    Ok(session)
}

async fn create_bridge_session(
    db: &NioDbClient,
    title: &str,
    agent: &str,
    model: Option<&str>,
    goal: Option<&str>,
) -> Result<Value, String> {
    match db
        .create_session(title, agent, model, Some("bridge"), goal, None)
        .await
    {
        Ok(session) => put_local_bridge_session(session),
        Err(remote_error) => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let session = json!({
                "id": format!("bridge_local_{now}"), "title": title, "agent": agent,
                "model": model.unwrap_or(agent), "swarm_type": "bridge", "goal": goal,
                "turns": [], "files_touched": [], "known_dead_ends": [],
                "local_only": true, "remote_error": remote_error,
            });
            put_local_bridge_session(session)
        }
    }
}

async fn append_bridge_turn(
    db: &NioDbClient,
    id: &str,
    agent: &str,
    model: Option<&str>,
    summary: &str,
    handoff_summary: Option<&str>,
    files: &[String],
) -> Result<Value, String> {
    let remote = db
        .append_turn_full(id, agent, model, summary, handoff_summary, files)
        .await;
    let mut sessions = read_local_bridge_sessions()?;
    let Some(session) = sessions
        .iter_mut()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(id))
    else {
        return remote.map_err(|e| e);
    };
    let mut turn = json!({ "agent": agent, "model": model.unwrap_or(agent), "summary": summary, "files_touched": files });
    if let Some(hs) = handoff_summary {
        turn["handoff_summary"] = json!(hs);
    }
    push_local_turn(session, turn);
    let mut all_files = session
        .get("files_touched")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for file in files {
        if !all_files.iter().any(|v| v.as_str() == Some(file)) {
            all_files.push(json!(file));
        }
    }
    session["files_touched"] = Value::Array(all_files);
    write_local_bridge_sessions(&sessions)?;
    remote.or_else(|_| Ok(json!({"id": id})))
}

async fn list_bridge_records(db: &NioDbClient) -> Result<Vec<Value>, String> {
    match db.list_sessions().await {
        Ok(remote_sessions) => {
            let mut sessions: Vec<Value> = remote_sessions
                .into_iter()
                .map(normalize_session_value)
                .collect();
            // The list endpoint may omit turn history. Load each manifest so the
            // picker can use the latest recorded turn as its fallback title.
            for session in &mut sessions {
                let has_turns = session
                    .get("turns")
                    .and_then(Value::as_array)
                    .is_some_and(|turns| !turns.is_empty());
                if has_turns {
                    continue;
                }
                let Some(id) = session.get("id").and_then(Value::as_str) else {
                    continue;
                };
                if let Ok(manifest) = db.get_manifest(id).await {
                    if let Some(turns) = manifest.get("recent_turns").and_then(Value::as_array) {
                        session["turns"] = Value::Array(turns.clone());
                    }
                    for (source_key, target_key) in [
                        ("goal", "goal"),
                        ("current_agent", "agent"),
                        ("model", "model"),
                    ] {
                        let missing = session
                            .get(target_key)
                            .and_then(Value::as_str)
                            .map_or(true, |value| value.trim().is_empty());
                        if missing {
                            if let Some(value) = manifest.get(source_key) {
                                session[target_key] = value.clone();
                            }
                        }
                    }
                }
            }
            if let Ok(local) = read_local_bridge_sessions() {
                for session in local {
                    let norm = normalize_session_value(session);
                    let id = norm.get("id").and_then(Value::as_str);
                    if !sessions
                        .iter()
                        .any(|s| s.get("id").and_then(Value::as_str) == id)
                    {
                        sessions.push(norm);
                    }
                }
            }
            let mut result: Vec<Value> = sessions
                .into_iter()
                .filter(|s| s.get("swarm_type").and_then(Value::as_str) != Some("assemble"))
                .collect();
            result.sort_by_key(|s| std::cmp::Reverse(extract_session_timestamp(s)));
            Ok(result)
        }
        Err(_) => {
            let local = read_local_bridge_sessions()?;
            let mut result: Vec<Value> = local.into_iter().map(normalize_session_value).collect();
            result.sort_by_key(|s| std::cmp::Reverse(extract_session_timestamp(s)));
            Ok(result)
        }
    }
}

async fn get_bridge_manifest(db: &NioDbClient, id: &str) -> Result<Value, String> {
    if let Ok(mut manifest) = db.get_manifest(id).await {
        remove_placeholder_turns(&mut manifest);
        if manifest.get("current_agent").is_none()
            || manifest.get("current_agent").and_then(Value::as_str) == Some("unknown")
        {
            if let Ok(sess) = db.get_session(id).await {
                if let Some(ag) = sess.get("agent") {
                    manifest["current_agent"] = ag.clone();
                }
                if let Some(mo) = sess.get("model") {
                    manifest["model"] = mo.clone();
                }
                if manifest
                    .get("goal")
                    .and_then(Value::as_str)
                    .map_or(true, |g| g.is_empty())
                {
                    if let Some(g) = sess.get("goal") {
                        manifest["goal"] = g.clone();
                    }
                }
            }
        }
        return Ok(manifest);
    }
    let sessions = read_local_bridge_sessions()?;
    let session = sessions
        .iter()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("no local bridge session found for {id}"))?;
    let mut manifest = json!({
        "session_id": id,
        "goal": session.get("goal").and_then(Value::as_str).unwrap_or(""),
        "current_agent": session.get("agent").and_then(Value::as_str).unwrap_or("unknown"),
        "model": session.get("model").and_then(Value::as_str).unwrap_or("agent default"),
        "recent_turns": session.get("turns").cloned().unwrap_or_else(|| json!([])),
        "files_touched": session.get("files_touched").cloned().unwrap_or_else(|| json!([])),
        "known_dead_ends": session.get("known_dead_ends").cloned().unwrap_or_else(|| json!([])),
    });
    remove_placeholder_turns(&mut manifest);
    Ok(manifest)
}

async fn switch_bridge_record(
    db: &NioDbClient,
    id: &str,
    agent: &str,
    model: Option<&str>,
    reason: Option<&str>,
) -> Result<Value, String> {
    let remote = db.switch_agent(id, agent, model, reason).await;
    let mut sessions = read_local_bridge_sessions()?;
    if let Some(session) = sessions
        .iter_mut()
        .find(|s| s.get("id").and_then(Value::as_str) == Some(id))
    {
        let previous_agent = session
            .get("agent")
            .and_then(Value::as_str)
            .unwrap_or("previous agent")
            .to_string();
        if let Some(reason) = reason.filter(|value| !value.trim().is_empty()) {
            push_local_turn(
                session,
                json!({
                    "agent": previous_agent,
                    "model": session.get("model").and_then(Value::as_str).unwrap_or("agent default"),
                    "summary": format!("Handoff note: {reason}"),
                    "files_touched": [],
                }),
            );
            session["goal"] = json!(reason);
        }
        session["agent"] = json!(agent);
        session["model"] = json!(model.unwrap_or(agent));
        write_local_bridge_sessions(&sessions)?;
        return remote.or_else(|_| Ok(json!({"id": id, "agent": agent})));
    }
    remote.or_else(|_| Ok(json!({"id": id, "agent": agent})))
}

fn detect_git_modified_files() -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.len() > 3 {
            let path = trimmed[3..].trim();
            if !path.is_empty() {
                files.push(path.to_string());
            }
        }
    }
    files
}

fn push_local_turn(session: &mut Value, turn: Value) {
    if !session.get("turns").is_some_and(Value::is_array) {
        session["turns"] = json!([]);
    }
    if let Some(turns) = session["turns"].as_array_mut() {
        turns.push(turn);
    }
}

fn remove_placeholder_turns(manifest: &mut Value) {
    if let Some(turns) = manifest
        .get_mut("recent_turns")
        .and_then(Value::as_array_mut)
    {
        turns.retain(|turn| {
            let summary = turn.get("summary").and_then(Value::as_str).unwrap_or("");
            summary != "Switched agent" && !summary.starts_with("Switched from ")
        });
    }
}

pub(crate) fn extract_agent_session_assessment(
    agent: &str,
    start_time: SystemTime,
) -> Option<String> {
    let home = env::var("HOME").ok()?;
    let home_path = PathBuf::from(home);

    match agent {
        "agy" => {
            let brain_dir = home_path.join(".gemini/antigravity-cli/brain");
            if !brain_dir.is_dir() {
                return None;
            }
            let entries = fs::read_dir(&brain_dir).ok()?;
            let mut dirs: Vec<PathBuf> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.path())
                .collect();
            dirs.sort_by_key(|d| {
                fs::metadata(d)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH)
            });
            dirs.reverse();

            // Candidate dirs within reasonable time window or most recent
            for dir in dirs {
                let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.starts_with('.') {
                    continue;
                }
                let modified = fs::metadata(&dir)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let duration = start_time.duration_since(modified).unwrap_or_default();
                // Allow up to 10 minutes prior to session start or anything after start_time
                if modified < start_time && duration.as_secs() > 600 {
                    continue;
                }

                let log_full = dir.join(".system_generated/logs/transcript_full.jsonl");
                let log_compact = dir.join(".system_generated/logs/transcript.jsonl");
                let target = if log_full.is_file() {
                    log_full
                } else if log_compact.is_file() {
                    log_compact
                } else {
                    continue;
                };

                if let Ok(content) = fs::read_to_string(&target) {
                    let mut latest_response: Option<String> = None;
                    for line in content.lines() {
                        if let Ok(val) = serde_json::from_str::<Value>(line) {
                            if val.get("source").and_then(Value::as_str) == Some("MODEL")
                                && val.get("type").and_then(Value::as_str)
                                    == Some("PLANNER_RESPONSE")
                            {
                                if let Some(text) = val.get("content").and_then(Value::as_str) {
                                    let trimmed = text.trim();
                                    if !trimmed.is_empty() {
                                        latest_response = Some(trimmed.to_string());
                                    }
                                }
                            }
                        }
                    }
                    if let Some(resp) = latest_response {
                        return Some(resp);
                    }
                }
            }
        }
        "codex" => {
            let history_path = home_path.join(".codex/history.jsonl");
            if history_path.is_file() {
                if let Ok(content) = fs::read_to_string(&history_path) {
                    let mut lines: Vec<&str> =
                        content.lines().filter(|l| !l.trim().is_empty()).collect();
                    lines.reverse();
                    for line in lines.iter().take(5) {
                        if let Ok(val) = serde_json::from_str::<Value>(line) {
                            if let Some(text) = val.get("text").and_then(Value::as_str) {
                                let trimmed = text.trim();
                                if trimmed.len() > 20 && !trimmed.starts_with('[') {
                                    return Some(trimmed.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        "claude" => {
            let projects_dir = home_path.join(".claude/projects");
            if projects_dir.is_dir() {
                if let Ok(entries) = fs::read_dir(&projects_dir) {
                    let mut files: Vec<PathBuf> = Vec::new();
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            if let Ok(sub) = fs::read_dir(entry.path()) {
                                for s in sub.flatten() {
                                    if s.path().extension().and_then(|x| x.to_str())
                                        == Some("jsonl")
                                    {
                                        files.push(s.path());
                                    }
                                }
                            }
                        }
                    }
                    files.sort_by_key(|f| {
                        fs::metadata(f)
                            .and_then(|m| m.modified())
                            .unwrap_or(SystemTime::UNIX_EPOCH)
                    });
                    files.reverse();

                    for file in files.into_iter().take(3) {
                        if let Ok(content) = fs::read_to_string(&file) {
                            let mut latest: Option<String> = None;
                            for line in content.lines() {
                                if let Ok(val) = serde_json::from_str::<Value>(line) {
                                    if val.get("type").and_then(Value::as_str) == Some("assistant")
                                    {
                                        if let Some(msg) = val.get("message") {
                                            if let Some(arr) =
                                                msg.get("content").and_then(Value::as_array)
                                            {
                                                for item in arr {
                                                    if item.get("type").and_then(Value::as_str)
                                                        == Some("text")
                                                    {
                                                        if let Some(txt) =
                                                            item.get("text").and_then(Value::as_str)
                                                        {
                                                            let trimmed = txt.trim();
                                                            if !trimmed.is_empty() {
                                                                latest = Some(trimmed.to_string());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(l) = latest {
                                return Some(l);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    None
}

pub(crate) async fn sync_local_bridge_sessions_to_db(db: &NioDbClient) {
    if !db.is_healthy().await {
        return;
    }
    let Ok(mut local_sessions) = read_local_bridge_sessions() else {
        return;
    };
    let mut modified = false;

    for s in &mut local_sessions {
        if s.get("local_only").and_then(Value::as_bool) == Some(true) {
            let title = s
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Bridge session")
                .to_string();
            let agent = s
                .get("agent")
                .and_then(Value::as_str)
                .unwrap_or("agy")
                .to_string();
            let model = s
                .get("model")
                .and_then(Value::as_str)
                .map(|s| s.to_string());
            let goal = s.get("goal").and_then(Value::as_str).map(|s| s.to_string());

            if let Ok(remote_session) = db
                .create_session(
                    &title,
                    &agent,
                    model.as_deref(),
                    Some("bridge"),
                    goal.as_deref(),
                    None,
                )
                .await
            {
                if let Some(remote_id) = remote_session.get("id").and_then(Value::as_str) {
                    let old_id = s
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    s["id"] = json!(remote_id);
                    s["local_only"] = json!(false);
                    s["remote_error"] = json!(null);
                    modified = true;

                    // Sync turns
                    if let Some(turns) = s.get("turns").and_then(Value::as_array).cloned() {
                        for turn in turns {
                            let tagent =
                                turn.get("agent").and_then(Value::as_str).unwrap_or(&agent);
                            let tmodel = turn.get("model").and_then(Value::as_str);
                            let tsummary =
                                turn.get("summary").and_then(Value::as_str).unwrap_or("");
                            let hs = turn.get("handoff_summary").and_then(Value::as_str);
                            let summary_to_post = tsummary.to_string();
                            let tfiles: Vec<String> = turn
                                .get("files_touched")
                                .and_then(Value::as_array)
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                            let _ = db
                                .append_turn_full(
                                    remote_id,
                                    tagent,
                                    tmodel,
                                    &summary_to_post,
                                    hs,
                                    &tfiles,
                                )
                                .await;
                        }
                    }
                    eprintln!(
                        "  \x1b[32m✓ Local bridge session {old_id} migrated to NioDB as {remote_id}\x1b[0m"
                    );
                }
            }
        }
    }

    if modified {
        let _ = write_local_bridge_sessions(&local_sessions);
    }
}
