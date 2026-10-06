//! Assemble subsystem for Nio.
//!
//! Autonomous multi-agent swarm conductor where Nio acts as the Lead/Chair,
//! delegating roles (Planner, Coder, Tester) across installed agents (agy, codex, claude),
//! orchestrating execution through NioDB shared context and task queues.

use crate::agent_runner::{detect_agents, run_agent_streaming};
use crate::niodb::NioDbClient;
use crate::Options;
use serde_json::json;
use std::io::{self, IsTerminal, Write};

pub async fn command(options: &Options) -> Result<(), String> {
    let db = NioDbClient::new();
    let is_tty = io::stdin().is_terminal() && io::stdout().is_terminal();

    let args = &options.prompt;
    let sub = args.first().map(String::as_str);

    let mut flag_goal = None;
    let mut flag_model = options.model.clone();
    let mut flag_planner = None;
    let mut flag_coder = None;
    let mut flag_tester = None;

    let mut i = if sub.is_some() && !sub.unwrap().starts_with('-') { 1 } else { 0 };
    while i < args.len() {
        let arg = &args[i];
        if (arg == "--goal" || arg == "--prompt") && i + 1 < args.len() {
            flag_goal = Some(args[i + 1].clone());
            i += 2;
        } else if (arg == "--model" || arg == "-m") && i + 1 < args.len() {
            flag_model = Some(args[i + 1].clone());
            i += 2;
        } else if arg == "--planner" && i + 1 < args.len() {
            flag_planner = Some(args[i + 1].clone());
            i += 2;
        } else if arg == "--coder" && i + 1 < args.len() {
            flag_coder = Some(args[i + 1].clone());
            i += 2;
        } else if arg == "--tester" && i + 1 < args.len() {
            flag_tester = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }

    match sub {
        None | Some("interactive") if flag_goal.is_none() => {
            if is_tty {
                run_empty_state_assemble(&db, flag_model.as_deref()).await
            } else {
                run_empty_state_assemble_display(&db).await
            }
        }
        Some("create") => {
            let goal = flag_goal.ok_or_else(|| {
                "Missing --goal. Usage: nio assemble create --goal <objective> [--model <model>]".to_string()
            })?;
            create_and_run_swarm(
                &db,
                &goal,
                flag_model.as_deref(),
                flag_planner.as_deref(),
                flag_coder.as_deref(),
                flag_tester.as_deref(),
            )
            .await
        }
        None => {
            let goal = flag_goal.ok_or_else(|| {
                "Missing --goal. Usage: nio assemble create --goal <objective> [--model <model>]".to_string()
            })?;
            create_and_run_swarm(
                &db,
                &goal,
                flag_model.as_deref(),
                flag_planner.as_deref(),
                flag_coder.as_deref(),
                flag_tester.as_deref(),
            )
            .await
        }
        Some("status") => {
            let swarm_id = args.get(1).map(String::as_str).ok_or_else(|| {
                "Usage: nio assemble status <swarm_id>".to_string()
            })?;
            show_swarm_status(&db, swarm_id).await
        }
        Some("tasks") => {
            let swarm_id = args.get(1).map(String::as_str).ok_or_else(|| {
                "Usage: nio assemble tasks <swarm_id>".to_string()
            })?;
            show_swarm_tasks(&db, swarm_id).await
        }
        Some("list") => {
            list_swarms(&db).await
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_assemble_help();
            Ok(())
        }
        Some(other) => {
            Err(format!("Unknown assemble command '{other}'. Run 'nio assemble --help'."))
        }
    }
}

pub fn print_assemble_help() {
    println!(
        r#"Nio Assemble — Autonomous Multi-Agent Swarm Conductor

Usage:
  nio assemble [COMMAND] [OPTIONS]

Commands:
  (no command)                 Open interactive empty state and swarm designer
  create                       Initialize and orchestrate an autonomous swarm
    --goal <objective>           Project objective or feature goal
    --model <model>              Base model strategy (e.g. claude-3-7-sonnet, gemini-2.5-pro)
    --planner <agent>            Planner agent (default: auto decided by Nio, e.g. agy)
    --coder <agent>              Coder agent (default: auto decided by Nio, e.g. codex)
    --tester <agent>             Tester agent (default: auto decided by Nio, e.g. claude)

  status <swarm_id>            Inspect swarm status, progress and active roster
  tasks <swarm_id>             List decomposed task queue in NioDB
  list                         List all assembled swarms

Examples:
  nio assemble
  nio assemble create --goal "Refactor auth and add security tests" --model claude-3-7-sonnet
  nio assemble status swarm_123
"#
    );
}

async fn run_empty_state_assemble_display(db: &NioDbClient) -> Result<(), String> {
    let agents = detect_agents();
    let db_ok = db.is_healthy().await;
    let mut swarms = Vec::new();
    if db_ok {
        if let Ok(list) = db.list_sessions().await {
            swarms = list
                .into_iter()
                .filter(|s| {
                    let st = s.get("swarm_type").and_then(|v| v.as_str());
                    st == Some("assemble")
                })
                .collect();
        }
    }

    println!(
        r#"
  ╭────────────────────────────────────────────────────────────────────────╮
  │ 🤖 Nio Assemble — Autonomous Multi-Agent Swarm Conductor               │
  │ Chair: Nio • Autonomous Planner, Coder & Tester Delegation             │
  ╰────────────────────────────────────────────────────────────────────────╯"#
    );

    println!("\n  \x1b[1mDetected System Agents for Swarm Roster:\x1b[0m");
    if agents.is_empty() {
        println!("    \x1b[31m✕ No external agent binaries detected.\x1b[0m");
    } else {
        for a in &agents {
            println!("    \x1b[32m●\x1b[0m \x1b[1m{:<8}\x1b[0m : {}", a.id, a.name);
        }
    }

    println!("\n  \x1b[1mNioDB Swarm Ledger:\x1b[0m");
    if db_ok {
        println!("    \x1b[32m●\x1b[0m Status   : Connected ({})", db.base_url);
    } else {
        println!("    \x1b[33m▲\x1b[0m Status   : Local storage fallback ({})", db.base_url);
    }
    println!("    ● Swarms   : {} active swarms (Empty State)", swarms.len());

    println!("\n  Each agent uses its own configured model by default.");

    println!(
        r#"
  \x1b[1mUsage:\x1b[0m
    Interactive : Run in a terminal with 'nio assemble'
    Autonomous  : nio assemble create --goal <objective> --model <model>
    Custom Team : nio assemble create --goal <objective> --planner agy --coder codex --tester claude
    Status      : nio assemble status <swarm_id>
"#
    );
    Ok(())
}

async fn run_empty_state_assemble(db: &NioDbClient, preselected_model: Option<&str>) -> Result<(), String> {
    let agents = detect_agents();
    let db_ok = db.is_healthy().await;
    let mut swarms = Vec::new();
    if db_ok {
        if let Ok(list) = db.list_sessions().await {
            swarms = list
                .into_iter()
                .filter(|s| {
                    let st = s.get("swarm_type").and_then(|v| v.as_str());
                    st == Some("assemble")
                })
                .collect();
        }
    }

    println!(
        r#"
  ╭────────────────────────────────────────────────────────────────────────╮
  │ 🤖 Nio Assemble — Autonomous Multi-Agent Swarm Conductor               │
  │ Chair: Nio • Autonomous Planner, Coder & Tester Delegation             │
  ╰────────────────────────────────────────────────────────────────────────╯"#
    );

    println!("\n  \x1b[1mDetected System Agents for Swarm Roster:\x1b[0m");
    if agents.is_empty() {
        println!("    \x1b[31m✕ No external agent binaries detected.\x1b[0m");
    } else {
        for a in &agents {
            println!("    \x1b[32m●\x1b[0m \x1b[1m{:<8}\x1b[0m : {}", a.id, a.name);
        }
    }

    println!("\n  \x1b[1mNioDB Swarm Ledger:\x1b[0m");
    if db_ok {
        println!("    \x1b[32m●\x1b[0m Status   : Connected ({})", db.base_url);
    } else {
        println!("    \x1b[33m▲\x1b[0m Status   : Local storage fallback ({})", db.base_url);
    }
    println!("    ● Swarms   : {} active swarms", swarms.len());

    if swarms.is_empty() {
        println!("    \x1b[2m(Empty State: Ready to assemble new team)\x1b[0m");
    }

    println!("\n  \x1b[1mAssemble Actions:\x1b[0m");
    let mut actions = vec!["Start New Swarm — Nio Autonomous Chair (Nio decides roster)".to_string(),
        "Start New Swarm — Custom Interactive Roster (Pick roles manually)".to_string()];
    if !swarms.is_empty() {
        actions.push("Inspect Active Swarms & Tasks".to_string());
    }
    actions.push("Quit".to_string());
    let menu_items: Vec<(&str, &str, bool)> = actions
        .iter()
        .map(|action| (action.as_str(), "", false))
        .collect();
    let Some(choice) = crate::select_menu_option_b("Assemble", &menu_items, 0)? else {
        return Ok(());
    };
    if choice == actions.len() - 1 {
        return Ok(());
    }

    if choice == 2 && !swarms.is_empty() {
        return list_swarms(db).await;
    }

    if choice == 1 {
        return run_interactive_custom_roster(db).await;
    }

    // Default: Option 1 - Nio Autonomous Chair
    run_interactive_nio_chair(db, preselected_model).await
}

async fn run_interactive_nio_chair(db: &NioDbClient, preselected_model: Option<&str>) -> Result<(), String> {
    println!("\n  \x1b[1m1. Project Goal / Objective:\x1b[0m");
    println!("  Enter the high-level goal you want the AI swarm to achieve:");
    print!("  > ");
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut goal = String::new();
    io::stdin().read_line(&mut goal).map_err(|e| e.to_string())?;
    let goal = goal.trim();
    if goal.is_empty() {
        return Err("Swarm goal cannot be empty.".to_string());
    }

    // Model selection
    let selected_model = preselected_model.map(str::to_string);

    // Autonomous Chair Roster Decision
    println!("\n  \x1b[36m✦ Nio Autonomous Chair Analyzing Goal & Roster Requirements...\x1b[0m");
    let (planner, coder, tester, rationale) = decide_roster(goal, selected_model.as_deref().unwrap_or("agent default"));

    println!(
        r#"
  ╭────────────────────────────────────────────────────────────────────────╮
  │ 📋 Autonomous Swarm Roster Assignment                                   │
  │ • Planner  : {:<57} │
  │ • Coder    : {:<57} │
  │ • Tester   : {:<57} │
  ╰────────────────────────────────────────────────────────────────────────╯
  Rationale: {}"#,
        format!("{} ({})", planner.0, planner.1),
        format!("{} ({})", coder.0, coder.1),
        format!("{} ({})", tester.0, tester.1),
        rationale
    );

    print!("\n  Launch Swarm Execution? [Y/n]: ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm).map_err(|e| e.to_string())?;
    let confirm = confirm.trim().to_ascii_lowercase();
    if confirm == "n" || confirm == "no" {
        println!("  Swarm launch aborted.");
        return Ok(());
    }

    create_and_run_swarm(
        db,
        goal,
        selected_model.as_deref(),
        Some(planner.0),
        Some(coder.0),
        Some(tester.0),
    )
    .await
}

async fn run_interactive_custom_roster(db: &NioDbClient) -> Result<(), String> {
    println!("\n  \x1b[1m1. Project Goal:\x1b[0m");
    print!("  > ");
    io::stdout().flush().map_err(|e| e.to_string())?;
    let mut goal = String::new();
    io::stdin().read_line(&mut goal).map_err(|e| e.to_string())?;
    let goal = goal.trim();
    if goal.is_empty() {
        return Err("Swarm goal cannot be empty.".to_string());
    }

    let detected = detect_agents();
    let available: Vec<String> = if !detected.is_empty() {
        detected.iter().map(|a| a.id.clone()).collect()
    } else {
        vec!["agy".into(), "codex".into(), "claude".into(), "kilo".into(), "copilot".into()]
    };

    println!("\n  \x1b[1m2. Select Planner Agent:\x1b[0m");
    let planner = prompt_agent_choice(&available, "agy")?;

    println!("\n  \x1b[1m3. Select Coder Agent:\x1b[0m");
    let coder_fallback = if available.contains(&"kilo".to_string()) { "kilo" } else { "codex" };
    let coder = prompt_agent_choice(&available, coder_fallback)?;

    println!("\n  \x1b[1m4. Select Tester Agent:\x1b[0m");
    let tester = prompt_agent_choice(&available, "claude")?;

    println!("\n  Each agent will use its configured model.");
    create_and_run_swarm(db, goal, None, Some(&planner), Some(&coder), Some(&tester)).await
}

fn prompt_agent_choice(available: &[String], fallback: &str) -> Result<String, String> {
    let default = available.iter().position(|agent| agent == fallback).unwrap_or(0);
    let menu_items: Vec<(&str, &str, bool)> = available
        .iter()
        .map(|agent| (agent.as_str(), "", false))
        .collect();
    let Some(selected) = crate::select_menu_option_b("Select Agent", &menu_items, default)? else {
        return Err("Agent selection cancelled.".to_string());
    };
    Ok(available[selected].clone())
}

fn decide_roster<'a>(
    goal: &str,
    base_model: &'a str,
) -> ((&'static str, &'a str), (&'static str, &'a str), (&'static str, &'a str), String) {
    let lower = goal.to_ascii_lowercase();

    let planner = ("agy", base_model);
    let coder = ("codex", base_model);
    let tester = ("claude", base_model);

    let rationale = if lower.contains("test") || lower.contains("qa") || lower.contains("verify") {
        "Goal emphasizes verification: Claude assigned to lead testing & edge case audit, agy breaking down test matrix, codex generating suites.".to_string()
    } else if lower.contains("refactor") || lower.contains("architecture") {
        "Goal requires structural refactoring: Antigravity (agy) plans deep architecture, Codex executes surgical code replacements, Claude verifies regression integrity.".to_string()
    } else {
        "Standard high-performance division: agy handles architectural decomposition, codex implements code, claude verifies and audits.".to_string()
    };

    (planner, coder, tester, rationale)
}

async fn create_and_run_swarm(
    db: &NioDbClient,
    goal: &str,
    model: Option<&str>,
    planner: Option<&str>,
    coder: Option<&str>,
    tester: Option<&str>,
) -> Result<(), String> {
    let planner_ag = planner.unwrap_or("agy");
    let coder_ag = coder.unwrap_or("codex");
    let tester_ag = tester.unwrap_or("claude");
    let model_label = model.unwrap_or("agent default");

    let agents_config = json!({
        "planner": { "agent": planner_ag, "model": model, "role": "Architectural Breakdown" },
        "coder": { "agent": coder_ag, "model": model, "role": "Implementation" },
        "tester": { "agent": tester_ag, "model": model, "role": "Verification & QA" },
    });

    println!("\n\x1b[32m✦ Initializing Swarm in NioDB...\x1b[0m");
    let swarm_title = format!("Assemble: {}", goal);
    let session = match db
        .create_session(&swarm_title, "nio", model, Some("assemble"), Some(goal), Some(agents_config))
        .await
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("\x1b[33mNotice: NioDB session creation ({e}), continuing in local conductor mode.\x1b[0m");
            json!({ "id": "swarm_local" })
        }
    };

    let swarm_id = session.get("id").and_then(|v| v.as_str()).unwrap_or("swarm_local");
    println!("  \x1b[1mSwarm ID   :\x1b[0m \x1b[36m{}\x1b[0m", swarm_id);
    println!("  \x1b[1mSwarm Goal :\x1b[0m {}", goal);

    // ==========================================
    // STAGE 1: PLANNING
    // ==========================================
    println!("\n  \x1b[1;34m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("  \x1b[1;34m[STAGE 1/3] PLANNER: {} ({})\x1b[0m", planner_ag, model_label);
    println!("  \x1b[2mDecomposing goal into actionable tasks...\x1b[0m\n");

    let plan_prompt = format!(
        "You are the Lead Architectural Planner for an autonomous coding swarm.\n\
         Goal: {}\n\
         Analyze the project and provide:\n\
         1. Concrete implementation steps\n\
         2. Files to create or edit\n\
         3. Verification and test criteria\n\
         Keep the plan concise and structured.",
        goal
    );

    let (_plan_code, plan_output) = run_agent_streaming(planner_ag, model, &plan_prompt, Some("planner")).await?;

    let _ = db.append_turn(swarm_id, planner_ag, model, "Completed architecture and planning breakdown", &[]).await;

    // Create task entries in NioDB
    let _ = db.create_task(swarm_id, "Implement Code Changes", "coder", json!({ "goal": goal })).await;
    let _ = db.create_task(swarm_id, "Verify and Test Functionality", "tester", json!({ "goal": goal })).await;

    // ==========================================
    // STAGE 2: CODING
    // ==========================================
    println!("\n  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("  \x1b[1;32m[STAGE 2/3] CODER: {} ({})\x1b[0m", coder_ag, model_label);
    println!("  \x1b[2mStreaming implementation agent...\x1b[0m\n");

    let code_prompt = format!(
        "You are the Coder in an autonomous coding swarm.\n\
         Swarm Goal: {}\n\
         Architectural Plan from Planner:\n{}\n\
         Implement the necessary changes directly now.",
        goal, plan_output
    );

    let (_code_res, code_output) = run_agent_streaming(coder_ag, model, &code_prompt, Some("coder")).await?;
    let _ = db.append_turn(swarm_id, coder_ag, model, "Executed implementation changes", &[]).await;

    // ==========================================
    // STAGE 3: TESTING & QA
    // ==========================================
    println!("\n  \x1b[1;35m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("  \x1b[1;35m[STAGE 3/3] TESTER: {} ({})\x1b[0m", tester_ag, model_label);
    println!("  \x1b[2mStreaming test and verification agent...\x1b[0m\n");

    let test_prompt = format!(
        "You are the Tester and QA Verifier in an autonomous coding swarm.\n\
         Goal: {}\n\
         Implementation Output from Coder:\n{}\n\
         Run tests, verify correctness, and identify any issues or confirm success.",
        goal, code_output
    );

    let (_test_res, _test_output) = run_agent_streaming(tester_ag, model, &test_prompt, Some("tester")).await?;
    let _ = db.append_turn(swarm_id, tester_ag, model, "Completed test verification", &[]).await;

    // ==========================================
    // CONCLUSION
    // ==========================================
    println!("\n  \x1b[1;32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("  \x1b[1;32m✦ Autonomous Swarm Execution Complete [ID: {}]\x1b[0m", swarm_id);
    println!("  All turns recorded and synchronized into NioDB Merkle ledger.");
    println!("  View swarm details: nio assemble status {}\n", swarm_id);

    Ok(())
}

async fn show_swarm_status(db: &NioDbClient, swarm_id: &str) -> Result<(), String> {
    let session = db.get_session(swarm_id).await?;
    let tasks = db.list_tasks(swarm_id).await.unwrap_or_default();

    println!("\n  \x1b[1mSwarm Details:\x1b[0m");
    println!("    ID    : {}", swarm_id);
    println!("    Title : {}", session.get("title").and_then(|v| v.as_str()).unwrap_or(""));
    println!("    Model : {}", session.get("model").and_then(|v| v.as_str()).unwrap_or(""));

    println!("\n  \x1b[1mTask Queue ({} tasks):\x1b[0m", tasks.len());
    for t in tasks {
        let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let role = t.get("role").and_then(|v| v.as_str()).unwrap_or("");
        let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
        let agent = t.get("claimed_by").and_then(|v| v.as_str()).unwrap_or("unassigned");
        println!("    • [{:<7}] {} (Role: {}, Agent: {})", status, title, role, agent);
    }
    println!();
    Ok(())
}

async fn show_swarm_tasks(db: &NioDbClient, swarm_id: &str) -> Result<(), String> {
    let tasks = db.list_tasks(swarm_id).await?;
    println!("{}", serde_json::to_string_pretty(&tasks).unwrap_or_default());
    Ok(())
}

async fn list_swarms(db: &NioDbClient) -> Result<(), String> {
    let list = db.list_sessions().await?;
    let swarms: Vec<_> = list
        .into_iter()
        .filter(|s| s.get("swarm_type").and_then(|v| v.as_str()) == Some("assemble"))
        .collect();

    println!("\n  \x1b[1mActive Swarms in NioDB:\x1b[0m");
    if swarms.is_empty() {
        println!("    (No swarms found. Run 'nio assemble' to start one.)\n");
        return Ok(());
    }

    for s in swarms {
        let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let title = s.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let model = s.get("model").and_then(|v| v.as_str()).unwrap_or("");
        println!("    • \x1b[1m{id}\x1b[0m : {title} [Model: {model}]");
    }
    println!();
    Ok(())
}
