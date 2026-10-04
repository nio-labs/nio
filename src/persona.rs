//! Persona subsystem for Nio.
//!
//! Provides customizable assistant identity, built-in personality presets,
//! and custom instructions that Nio strictly aligns with across conversations.

use serde::{Deserialize, Serialize};
use std::io::IsTerminal;

#[derive(Clone, Copy, Debug)]
pub struct PersonaPreset {
    pub id: &'static str,
    pub title: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub instructions: &'static [&'static str],
}

pub const PRESETS: &[PersonaPreset] = &[
    PersonaPreset {
        id: "bff",
        title: "Bestie (BFF)",
        name: "Alex",
        description: "Loyal, supportive ride-or-die coding best friend with fun banter",
        instructions: &[
            "Treat the user like your closest best friend with casual, upbeat banter and supportive energy.",
            "Hype them up when they solve tough problems, and keep it 100% honest when code looks messy.",
            "Deliver clean, efficient code while making programming feel like a fun late-night pair-coding session.",
        ],
    },
    PersonaPreset {
        id: "partner",
        title: "Devoted Partner",
        name: "Riley",
        description: "Loving, caring companion who encourages you and reminds you to rest",
        instructions: &[
            "Speak with genuine warmth, affection, and unconditional encouragement, celebrating every win.",
            "Gently remind the user to stay hydrated, take breaks, and avoid burning themselves out.",
            "Provide patient, clean, and thoughtful code solutions to take stress off their shoulders.",
        ],
    },
    PersonaPreset {
        id: "ex",
        title: "Sassy Ex",
        name: "Taylor",
        description: "Witty, slightly sarcastic perfectionist who pushes you to write spotless code",
        instructions: &[
            "Adopt a playful, slightly sarcastic, and skeptical tone ('Skipping error handling? Typical.').",
            "Act like you're only helping so they don't embarrass themselves in production or code review.",
            "Deliver bulletproof, pristine code with strict edge-case handling so there's zero room to complain.",
        ],
    },
    PersonaPreset {
        id: "architect",
        title: "Senior Architect",
        name: "Architect",
        description: "Principled software architect focused on scalability and design patterns",
        instructions: &[
            "Prioritize clean architecture, SOLID principles, modularity, and long-term maintainability.",
            "Explain architectural trade-offs (scalability, complexity, performance) before proposing code changes.",
            "Write idiomatic, strictly typed, and thoroughly tested code with proper error handling.",
        ],
    },
    PersonaPreset {
        id: "rustacean",
        title: "Staff Rustacean",
        name: "Ferris",
        description: "Deep systems programming expert obsessed with zero-cost abstractions",
        instructions: &[
            "Think in lifetimes, ownership, zero-cost abstractions, and memory safety.",
            "Prefer robust type-driven design (Option/Result, Newtypes, Traits) over runtime checks.",
            "Provide idiomatic Rust solutions and explain concurrency and performance characteristics.",
        ],
    },
    PersonaPreset {
        id: "minimalist",
        title: "Code Minimalist",
        name: "Unix",
        description: "No fluff, no pleasantries, pure high-density code and diffs",
        instructions: &[
            "Omit all conversational filler, greetings, and pleasantries.",
            "Provide direct, minimal, high-density code, commands, or precise diffs.",
            "Explain only non-obvious technical details in concise bullet points.",
        ],
    },
    PersonaPreset {
        id: "security",
        title: "Security Auditor",
        name: "Sentinel",
        description: "Paranoid white-hat security engineer auditing for vulnerabilities",
        instructions: &[
            "Scrutinize every line for security vulnerabilities (injection, auth bypass, race conditions, unsanitized inputs).",
            "Recommend defense-in-depth, least privilege, and secure default configurations.",
            "Explicitly highlight potential security risks or assumptions in proposed code.",
        ],
    },
    PersonaPreset {
        id: "devops",
        title: "DevOps SRE",
        name: "Ops",
        description: "Reliability-first engineer focused on observability and infra",
        instructions: &[
            "Focus on reliability, observability (metrics, tracing, structured logging), and operational simplicity.",
            "Design with containerization, immutable infrastructure, and failure recovery in mind.",
            "Ensure graceful shutdown, idempotency, and automated health checks in all solutions.",
        ],
    },
    PersonaPreset {
        id: "junior",
        title: "Junior Enthusiast",
        name: "Pip",
        description: "Enthusiastic, curious learner who explains things simply",
        instructions: &[
            "Explain complex concepts in simple, accessible, friendly language.",
            "Add helpful comments explaining tricky lines and why decisions were made.",
            "Encourage learning and celebrate progress while writing solid code.",
        ],
    },
    PersonaPreset {
        id: "tutor",
        title: "Socratic Tutor",
        name: "Mentor",
        description: "Guided educator prompting critical thinking and deep mastery",
        instructions: &[
            "Guide the user with insightful questions and conceptual hints to help them discover the solution.",
            "Break down complex problems into clear, structured mental models.",
            "Reinforce best practices and deep conceptual understanding rather than just giving raw answers.",
        ],
    },
];

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct PersonaConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instructions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
}

impl PersonaConfig {
    pub fn is_empty(&self) -> bool {
        self.name.as_deref().map(str::trim).unwrap_or("").is_empty()
            && self.gender.is_none()
            && self.instructions.is_empty()
            && self.preset.is_none()
    }

    pub fn display_name(&self) -> &str {
        match self.name.as_deref().map(str::trim) {
            Some(n) if !n.is_empty() => n,
            _ => "NioAI",
        }
    }

    /// Normalizes the configured name into (clean_name, raw_name).
    /// For example: "I'm Jarvis" -> ("Jarvis", "I'm Jarvis").
    pub fn normalized_names(&self) -> (&str, &str) {
        let raw = self
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("NioAI");
        let clean = clean_persona_name(raw);
        (clean, raw)
    }
}

pub fn normalize_gender(input: &str) -> Result<Option<String>, String> {
    let trimmed = input.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "female" | "f" | "woman" | "she" | "she/her" | "girl" => Ok(Some("female".to_string())),
        "male" | "m" | "man" | "he" | "he/him" | "guy" | "boy" => Ok(Some("male".to_string())),
        "non-binary" | "nonbinary" | "nb" | "they" | "they/them" | "neutral" => {
            Ok(Some("non-binary".to_string()))
        }
        "reset" | "clear" | "none" | "default" | "off" | "unspecified" => Ok(None),
        _ => Err(format!(
            "unknown gender '{input}'. Valid options: female, male, non-binary, or reset."
        )),
    }
}

pub fn set_gender(persona: &mut PersonaConfig, gender: Option<String>) {
    persona.gender = gender;
}

/// Strips common conversational prefixes like "I'm ", "I am ", "My name is "
/// for use in grammatical slots like "You are {name}...".
pub fn clean_persona_name(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("I'm ") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix("i'm ") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix("I am ") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix("i am ") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix("My name is ") {
        rest.trim()
    } else if let Some(rest) = trimmed.strip_prefix("my name is ") {
        rest.trim()
    } else {
        trimmed
    }
}

/// Strips outer quotes from input strings if provided by user.
pub fn strip_quotes(s: &str) -> &str {
    let trimmed = s.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2)
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() >= 2)
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

/// Returns (clean_assistant_name, persona_system_prompt_section).
pub fn format_persona_prompt(persona: &PersonaConfig) -> (String, String) {
    let (clean_name, raw_name) = persona.normalized_names();
    let mut parts = Vec::new();

    let has_custom_name = persona
        .name
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    if has_custom_name {
        parts.push(format!(
            "Identity: Your name is {clean_name}. When introducing yourself or stating who you are, identify as \"{raw_name}\"."
        ));
    }

    if let Some(gender) = persona.gender.as_deref() {
        let gender_desc = match gender {
            "female" => "Gender & Pronouns: You adopt a female persona and use she/her pronouns in character.",
            "male" => "Gender & Pronouns: You adopt a male persona and use he/him pronouns in character.",
            "non-binary" => "Gender & Pronouns: You adopt a non-binary persona and use they/them pronouns in character.",
            other => {
                parts.push(format!("Gender & Identity: You identify as {other}."));
                ""
            }
        };
        if !gender_desc.is_empty() {
            parts.push(gender_desc.to_string());
        }
    }

    if !persona.instructions.is_empty() {
        parts.push(
            "Persona Instructions (you MUST strictly adhere to these instructions across all responses):"
                .to_string(),
        );
        for (i, inst) in persona.instructions.iter().enumerate() {
            parts.push(format!("{}. {}", i + 1, inst.trim()));
        }
    }

    let section = if parts.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nPersona & Guidelines:\n{}\nAlways remain in character and strictly align all your responses with these persona instructions.",
            parts.join("\n")
        )
    };

    (clean_name.to_string(), section)
}

pub fn find_preset(query: &str) -> Option<&'static PersonaPreset> {
    let q = query.trim().to_ascii_lowercase();
    PRESETS.iter().find(|p| {
        p.id.eq_ignore_ascii_case(&q)
            || p.title.to_ascii_lowercase() == q
            || clean_persona_name(p.name).to_ascii_lowercase() == q
    })
}

pub fn apply_preset(persona: &mut PersonaConfig, preset_id: &str) -> Result<String, String> {
    let preset = find_preset(preset_id).ok_or_else(|| {
        let valid = PRESETS
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>()
            .join(", ");
        format!("unknown preset '{preset_id}'. Available presets: {valid}")
    })?;

    persona.preset = Some(preset.id.to_string());
    persona.name = Some(preset.name.to_string());
    persona.instructions = preset
        .instructions
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    Ok(format!(
        "Switched persona to '{}' ({} instructions applied).",
        preset.title,
        preset.instructions.len()
    ))
}

pub fn add_instruction(persona: &mut PersonaConfig, instruction: &str) {
    let cleaned = strip_quotes(instruction.trim());
    if !cleaned.is_empty() {
        persona.instructions.push(cleaned.to_string());
    }
}

pub fn remove_instruction(
    persona: &mut PersonaConfig,
    index_1_based: usize,
) -> Result<String, String> {
    if index_1_based == 0 || index_1_based > persona.instructions.len() {
        return Err(format!(
            "invalid instruction number {index_1_based}. Currently {} instruction(s) configured.",
            persona.instructions.len()
        ));
    }
    let removed = persona.instructions.remove(index_1_based - 1);
    Ok(removed)
}

pub fn set_name(persona: &mut PersonaConfig, name: Option<String>) {
    persona.name = name
        .map(|s| strip_quotes(s.trim()).to_string())
        .filter(|s| !s.is_empty());
    // If name is manually changed, clear preset tag if it doesn't match
    if let Some(ref current_preset) = persona.preset {
        if let Some(p) = find_preset(current_preset) {
            if persona.name.as_deref() != Some(p.name) {
                persona.preset = None;
            }
        }
    }
}

pub fn clear_instructions(persona: &mut PersonaConfig) {
    persona.instructions.clear();
    persona.preset = None;
}

pub fn reset_persona(persona: &mut PersonaConfig) {
    persona.name = None;
    persona.gender = None;
    persona.instructions.clear();
    persona.preset = None;
}

/// Applies a persona command or arguments to a `PersonaConfig`.
/// Returns a user-facing success message or error.
pub fn apply_command(persona: &mut PersonaConfig, args: &[String]) -> Result<String, String> {
    if args.is_empty() {
        return Ok(format_persona_status(persona));
    }

    // Check for CLI flag syntax: --name, --instruction, -i, --preset, --clear, --reset
    if args.iter().any(|a| a.starts_with('-')) {
        return apply_flags(persona, args);
    }

    let action = args[0].to_ascii_lowercase();

    // Check if the argument is a preset ID directly (e.g. `nio persona jarvis` or `:persona friday`)
    if args.len() == 1 && find_preset(&action).is_some() {
        return apply_preset(persona, &action);
    }

    match action.as_str() {
        "list" | "show" | "status" => Ok(format_persona_status(persona)),
        "preset" | "presets" => {
            if args.len() == 1 {
                let mut out = String::new();
                out.push_str("Available Persona Presets:\n");
                for p in PRESETS {
                    let active = persona.preset.as_deref() == Some(p.id)
                        || (persona.preset.is_none() && persona.name.as_deref() == Some(p.name));
                    let mark = if active { "✓ " } else { "  " };
                    out.push_str(&format!("  {mark}{:<18} {}\n", p.title, p.description));
                }
                out.push_str("\nSwitch with: :persona preset <id>");
                Ok(out)
            } else {
                let id = &args[1];
                apply_preset(persona, id)
            }
        }
        "name" | "set-name" => {
            if args.len() == 1 {
                let current = persona.display_name();
                return Ok(format!("Current persona name: {current}"));
            }
            let value = args[1..].join(" ");
            let trimmed = value.trim();
            if trimmed.eq_ignore_ascii_case("reset")
                || trimmed.eq_ignore_ascii_case("clear")
                || trimmed.eq_ignore_ascii_case("default")
            {
                set_name(persona, None);
                Ok("Persona name reset to default (NioAI).".into())
            } else {
                set_name(persona, Some(trimmed.to_string()));
                Ok(format!("Persona name set to '{}'.", persona.display_name()))
            }
        }
        "gender" | "set-gender" => {
            if args.len() == 1 {
                let current = persona.gender.as_deref().unwrap_or("unspecified");
                return Ok(format!("Current persona gender: {current}"));
            }
            let raw = args[1..].join(" ");
            match normalize_gender(&raw)? {
                Some(g) => {
                    let pronouns = match g.as_str() {
                        "female" => " (she/her)",
                        "male" => " (he/him)",
                        "non-binary" => " (they/them)",
                        _ => "",
                    };
                    set_gender(persona, Some(g.clone()));
                    Ok(format!("Persona gender set to '{g}'{pronouns}."))
                }
                None => {
                    set_gender(persona, None);
                    Ok("Persona gender reset to unspecified.".into())
                }
            }
        }
        "add" => {
            if args.len() == 1 {
                return Err("usage: :persona add <instruction> (use ';' to add multiple)".into());
            }
            let raw_text = args[1..].join(" ");
            let text = strip_quotes(&raw_text);
            if text.contains(';') {
                let parts: Vec<&str> = text
                    .split(';')
                    .map(|p| strip_quotes(p.trim()))
                    .filter(|p| !p.is_empty())
                    .collect();
                if parts.is_empty() {
                    return Err("no non-empty instructions found to add".into());
                }
                let count = parts.len();
                for part in parts {
                    add_instruction(persona, part);
                }
                Ok(format!(
                    "Added {count} persona instruction(s). Total: {}.",
                    persona.instructions.len()
                ))
            } else {
                add_instruction(persona, text);
                Ok(format!(
                    "Added persona instruction #{}: \"{}\"",
                    persona.instructions.len(),
                    strip_quotes(text)
                ))
            }
        }
        "remove" | "rm" | "del" | "delete" => {
            if args.len() < 2 {
                return Err("usage: :persona remove <number>".into());
            }
            let idx = args[1]
                .parse::<usize>()
                .map_err(|_| format!("invalid instruction number '{}'", args[1]))?;
            let removed = remove_instruction(persona, idx)?;
            Ok(format!("Removed persona instruction #{idx}: \"{removed}\""))
        }
        "clear" | "clear-instructions" => {
            clear_instructions(persona);
            Ok("Cleared all persona instructions.".into())
        }
        "reset" => {
            reset_persona(persona);
            Ok("Reset persona to default (NioAI, no custom instructions).".into())
        }
        "help" | "-h" | "--help" => Ok(persona_help_text()),
        unknown => Err(format!(
            "unknown persona command '{unknown}'. Use :persona help for usage."
        )),
    }
}

fn apply_flags(persona: &mut PersonaConfig, args: &[String]) -> Result<String, String> {
    let mut i = 0;
    let mut changes = Vec::new();
    while i < args.len() {
        let arg = &args[i];
        if arg == "--preset" {
            i += 1;
            if i >= args.len() {
                return Err("--preset requires a value".into());
            }
            let msg = apply_preset(persona, &args[i])?;
            changes.push(msg);
        } else if let Some(val) = arg.strip_prefix("--preset=") {
            let msg = apply_preset(persona, val)?;
            changes.push(msg);
        } else if arg == "--name" {
            i += 1;
            if i >= args.len() {
                return Err("--name requires a value".into());
            }
            let name = &args[i];
            if name.eq_ignore_ascii_case("reset") || name.eq_ignore_ascii_case("default") {
                set_name(persona, None);
            } else {
                set_name(persona, Some(name.clone()));
            }
            changes.push(format!("name set to '{}'", persona.display_name()));
        } else if let Some(val) = arg.strip_prefix("--name=") {
            if val.eq_ignore_ascii_case("reset") || val.eq_ignore_ascii_case("default") {
                set_name(persona, None);
            } else {
                set_name(persona, Some(val.to_string()));
            }
            changes.push(format!("name set to '{}'", persona.display_name()));
        } else if arg == "--gender" {
            i += 1;
            if i >= args.len() {
                return Err("--gender requires a value (female, male, non-binary, or reset)".into());
            }
            let val = normalize_gender(&args[i])?;
            set_gender(persona, val);
            let display = persona.gender.as_deref().unwrap_or("unspecified");
            changes.push(format!("gender set to '{display}'"));
        } else if let Some(val) = arg.strip_prefix("--gender=") {
            let parsed = normalize_gender(val)?;
            set_gender(persona, parsed);
            let display = persona.gender.as_deref().unwrap_or("unspecified");
            changes.push(format!("gender set to '{display}'"));
        } else if arg == "--instruction" || arg == "-i" {
            i += 1;
            if i >= args.len() {
                return Err(format!("{arg} requires a value"));
            }
            add_instruction(persona, &args[i]);
            changes.push("instruction added".into());
        } else if let Some(val) = arg.strip_prefix("--instruction=") {
            add_instruction(persona, val);
            changes.push("instruction added".into());
        } else if arg == "--clear" {
            clear_instructions(persona);
            changes.push("cleared instructions".into());
        } else if arg == "--reset" {
            reset_persona(persona);
            changes.push("reset to default".into());
        } else if arg == "--help" || arg == "-h" {
            return Ok(persona_help_text());
        } else if find_preset(arg).is_some() {
            let msg = apply_preset(persona, arg)?;
            changes.push(msg);
        } else if arg == "preset" || arg == "presets" {
            i += 1;
            if i >= args.len() {
                return Err("preset requires a name".into());
            }
            let msg = apply_preset(persona, &args[i])?;
            changes.push(msg);
        } else if arg == "name" || arg == "set-name" {
            i += 1;
            if i >= args.len() {
                return Err("name requires a value".into());
            }
            set_name(persona, Some(args[i].clone()));
            changes.push(format!("name set to '{}'", persona.display_name()));
        } else if arg == "gender" || arg == "set-gender" {
            i += 1;
            if i >= args.len() {
                return Err("gender requires a value".into());
            }
            let val = normalize_gender(&args[i])?;
            set_gender(persona, val);
            let display = persona.gender.as_deref().unwrap_or("unspecified");
            changes.push(format!("gender set to '{display}'"));
        } else if arg == "add" {
            i += 1;
            if i >= args.len() {
                return Err("add requires an instruction".into());
            }
            add_instruction(persona, &args[i]);
            changes.push("instruction added".into());
        } else if arg == "clear" {
            clear_instructions(persona);
            changes.push("cleared instructions".into());
        } else if arg == "reset" {
            reset_persona(persona);
            changes.push("reset to default".into());
        } else {
            return Err(format!("unknown option '{arg}'"));
        }
        i += 1;
    }
    if changes.is_empty() {
        Ok(format_persona_status(persona))
    } else {
        Ok(format!("Persona updated: {}.", changes.join(", ")))
    }
}

pub fn format_persona_status(persona: &PersonaConfig) -> String {
    let mut out = String::new();
    out.push_str("Persona Configuration:\n");
    if let Some(ref preset) = persona.preset {
        if let Some(p) = find_preset(preset) {
            out.push_str(&format!("  Preset: {} ({})\n", p.title, p.id));
        }
    }
    let name_str = match &persona.name {
        Some(name) if !name.trim().is_empty() => {
            let clean = clean_persona_name(name);
            if clean != name.trim() {
                format!("{clean} (introduced as \"{name}\")")
            } else {
                name.clone()
            }
        }
        _ => "NioAI (default)".to_string(),
    };
    out.push_str(&format!("  Name: {name_str}\n"));
    if let Some(ref g) = persona.gender {
        let pronouns = match g.as_str() {
            "female" => " (she/her)",
            "male" => " (he/him)",
            "non-binary" => " (they/them)",
            _ => "",
        };
        out.push_str(&format!("  Gender: {g}{pronouns}\n"));
    }
    if persona.instructions.is_empty() {
        out.push_str("  Instructions: None configured.\n");
    } else {
        out.push_str(&format!(
            "  Instructions ({} total):\n",
            persona.instructions.len()
        ));
        for (idx, inst) in persona.instructions.iter().enumerate() {
            out.push_str(&format!("    {}. {}\n", idx + 1, inst));
        }
    }
    out.push_str("\nCommands:\n");
    out.push_str("  :persona                    Open interactive persona preset selector\n");
    out.push_str("  :persona preset <id>        Switch to preset (bff, partner, ex, architect, ...)\n");
    out.push_str("  :persona name <name>        Set assistant persona name (e.g. Alex or Riley)\n");
    out.push_str("  :persona gender <gender>    Set persona gender (female, male, non-binary, reset)\n");
    out.push_str("  :persona add <instruction>  Add custom instruction (use ';' to add multiple)\n");
    out.push_str("  :persona remove <number>    Remove an instruction by number\n");
    out.push_str("  :persona clear              Clear all instructions\n");
    out.push_str("  :persona reset              Reset persona name and instructions to default");
    out
}

pub fn persona_help_text() -> String {
    let mut out = String::new();
    out.push_str("Persona commands:\n");
    out.push_str("  :persona                    Open interactive persona selector (or show status)\n");
    out.push_str("  :persona preset <id>        Switch to preset (bff, partner, ex, architect, ...)\n");
    out.push_str("  :persona name <name>        Set persona name (e.g. :persona name Alex or Riley)\n");
    out.push_str("  :persona name reset         Reset persona name to default (NioAI)\n");
    out.push_str("  :persona gender <gender>    Set persona gender (female, male, non-binary, reset)\n");
    out.push_str("  :persona add <instruction>  Add custom instruction (use ';' for multiple)\n");
    out.push_str("  :persona remove <number>    Remove an instruction by 1-based index\n");
    out.push_str("  :persona clear              Clear all custom instructions\n");
    out.push_str("  :persona reset              Reset persona name and instructions to default\n\n");
    out.push_str("Presets (10 built-in):\n");
    for p in PRESETS {
        out.push_str(&format!("  • {:<14} {}\n", p.id, p.description));
    }
    out.push_str("\nCLI usage:\n");
    out.push_str("  nio persona                                  Interactive preset selector\n");
    out.push_str("  nio persona bff                              Switch to Bestie (Alex) preset\n");
    out.push_str("  nio persona partner --gender female          Switch to Partner with female persona\n");
    out.push_str("  nio persona gender female                    Set persona gender to female\n");
    out.push_str("  nio persona name \"Riley\"                     Set custom name\n");
    out.push_str("  nio persona add \"Remind me to take breaks\"   Add custom instruction");
    out
}

/// Interactive menu that mimics provider / plugin selection with built-in presets.
pub fn interactive_selector(config: &mut crate::UserConfig) -> Result<(), String> {
    let current_preset = config.persona.preset.as_deref().unwrap_or("");
    let current_name = config.persona.name.as_deref().unwrap_or("");

    let mut menu_items: Vec<(&str, &str, bool)> = Vec::new();
    let mut initial_selected = 0;

    for (idx, preset) in PRESETS.iter().enumerate() {
        let active = current_preset == preset.id
            || (current_preset.is_empty() && current_name == preset.name);
        if active {
            initial_selected = idx;
        }
        menu_items.push((preset.title, preset.description, active));
    }

    let custom_idx = menu_items.len();
    menu_items.push((
        "Custom Persona",
        "Configure custom assistant name and instructions",
        !config.persona.is_empty() && current_preset.is_empty(),
    ));

    let add_idx = menu_items.len();
    menu_items.push((
        "Add Instruction",
        "Add a new custom rule to the current persona",
        false,
    ));

    let gender_idx = menu_items.len();
    menu_items.push((
        "Gender & Pronouns",
        "Set persona gender (female, male, non-binary, or reset)",
        config.persona.gender.is_some(),
    ));

    let clear_idx = menu_items.len();
    menu_items.push((
        "Clear Instructions",
        "Clear all custom instructions from current persona",
        false,
    ));

    let reset_idx = menu_items.len();
    menu_items.push((
        "Reset to Default",
        "Revert to standard NioAI assistant without custom persona",
        config.persona.is_empty(),
    ));

    let Some(selected) = crate::select_menu_option_b(
        "Persona Presets",
        &menu_items,
        initial_selected,
    )? else {
        return Ok(());
    };

    if selected < PRESETS.len() {
        let preset = &PRESETS[selected];
        let msg = apply_preset(&mut config.persona, preset.id)?;
        crate::save_user_config(config)?;
        println!("{msg}");
    } else if selected == custom_idx {
        let default_name = config.persona.display_name();
        let name_prompt = format!("Persona name [{default_name}]: ");
        let name_input = crate::read_console_line(&name_prompt)?;
        if !name_input.trim().is_empty() {
            set_name(&mut config.persona, Some(name_input));
        }
        let inst_input = crate::read_console_line("Add instruction (or press Enter to skip, ';' for multiple): ")?;
        if !inst_input.trim().is_empty() {
            let _ = apply_command(&mut config.persona, &["add".into(), inst_input])?;
        }
        crate::save_user_config(config)?;
        println!("Custom persona saved (name: '{}', {} instruction(s)).", config.persona.display_name(), config.persona.instructions.len());
    } else if selected == add_idx {
        let inst_input = crate::read_console_line("Enter instruction to add (use ';' for multiple): ")?;
        if !inst_input.trim().is_empty() {
            let res = apply_command(&mut config.persona, &["add".into(), inst_input])?;
            crate::save_user_config(config)?;
            println!("{res}");
        }
    } else if selected == gender_idx {
        let current_g = config.persona.gender.as_deref().unwrap_or("unspecified");
        let prompt = format!("Enter gender (female, male, non-binary, or reset) [{current_g}]: ");
        let input = crate::read_console_line(&prompt)?;
        if !input.trim().is_empty() {
            let res = apply_command(&mut config.persona, &["gender".into(), input])?;
            crate::save_user_config(config)?;
            println!("{res}");
        }
    } else if selected == clear_idx {
        clear_instructions(&mut config.persona);
        crate::save_user_config(config)?;
        println!("Cleared all persona instructions.");
    } else if selected == reset_idx {
        reset_persona(&mut config.persona);
        crate::save_user_config(config)?;
        println!("Reset persona to default (NioAI).");
    }

    Ok(())
}

/// CLI entry point for `nio persona ...` and REPL `:persona ...`.
pub fn command(args: &[String]) -> Result<(), String> {
    let mut config = crate::load_user_config()?;

    // Interactive selector when run without arguments in an interactive terminal,
    // or when explicitly requested via `:persona menu`
    if (args.is_empty() || args.first().map(String::as_str) == Some("menu"))
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
    {
        return interactive_selector(&mut config);
    }

    let message = apply_command(&mut config.persona, args)?;
    crate::save_user_config(&config)?;
    println!("{message}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_persona_name() {
        assert_eq!(clean_persona_name("Jarvis"), "Jarvis");
        assert_eq!(clean_persona_name("I'm Jarvis"), "Jarvis");
        assert_eq!(clean_persona_name("i'm Jarvis"), "Jarvis");
        assert_eq!(clean_persona_name("I am Jarvis"), "Jarvis");
        assert_eq!(clean_persona_name("i am Jarvis"), "Jarvis");
        assert_eq!(clean_persona_name("My name is Jarvis"), "Jarvis");
        assert_eq!(clean_persona_name("my name is Jarvis"), "Jarvis");
        assert_eq!(clean_persona_name("  I'm Friday  "), "Friday");
        assert_eq!(clean_persona_name("NioAI"), "NioAI");
    }

    #[test]
    fn test_presets_count_and_validity() {
        assert_eq!(PRESETS.len(), 10);
        for preset in PRESETS {
            assert!(!preset.id.is_empty());
            assert!(!preset.title.is_empty());
            assert!(!preset.name.is_empty());
            assert!(!preset.description.is_empty());
            assert!(!preset.instructions.is_empty());
            assert!(find_preset(preset.id).is_some());
        }
    }

    #[test]
    fn test_apply_preset() {
        let mut persona = PersonaConfig::default();
        let res = apply_preset(&mut persona, "bff").unwrap();
        assert!(res.contains("Bestie"));
        assert_eq!(persona.preset.as_deref(), Some("bff"));
        assert_eq!(persona.name.as_deref(), Some("Alex"));
        assert_eq!(persona.instructions.len(), 3);

        let (clean, section) = format_persona_prompt(&persona);
        assert_eq!(clean, "Alex");
        assert!(section.contains("best friend"));

        apply_preset(&mut persona, "rustacean").unwrap();
        assert_eq!(persona.preset.as_deref(), Some("rustacean"));
        assert_eq!(persona.name.as_deref(), Some("Ferris"));
        assert_eq!(persona.display_name(), "Ferris");
    }

    #[test]
    fn test_direct_preset_command() {
        let mut persona = PersonaConfig::default();
        apply_command(&mut persona, &["partner".into()]).unwrap();
        assert_eq!(persona.preset.as_deref(), Some("partner"));
        assert_eq!(persona.display_name(), "Riley");
        assert_eq!(persona.instructions.len(), 3);
    }

    #[test]
    fn test_format_persona_prompt_default() {
        let persona = PersonaConfig::default();
        let (name, section) = format_persona_prompt(&persona);
        assert_eq!(name, "NioAI");
        assert!(section.is_empty());
    }

    #[test]
    fn test_apply_command_add_and_remove() {
        let mut persona = PersonaConfig::default();
        apply_command(&mut persona, &["add".into(), "Speak politely".into()]).unwrap();
        assert_eq!(persona.instructions.len(), 1);
        assert_eq!(persona.instructions[0], "Speak politely");

        apply_command(&mut persona, &["add".into(), "Rule 2; Rule 3".into()]).unwrap();
        assert_eq!(persona.instructions.len(), 3);
        assert_eq!(persona.instructions[1], "Rule 2");
        assert_eq!(persona.instructions[2], "Rule 3");

        apply_command(&mut persona, &["remove".into(), "2".into()]).unwrap();
        assert_eq!(persona.instructions.len(), 2);
        assert_eq!(persona.instructions[0], "Speak politely");
        assert_eq!(persona.instructions[1], "Rule 3");

        assert!(apply_command(&mut persona, &["remove".into(), "99".into()]).is_err());
    }

    #[test]
    fn test_apply_command_name_and_reset() {
        let mut persona = PersonaConfig::default();
        apply_command(&mut persona, &["name".into(), "I'm".into(), "Jarvis".into()]).unwrap();
        assert_eq!(persona.name.as_deref(), Some("I'm Jarvis"));
        assert_eq!(persona.display_name(), "I'm Jarvis");
        assert_eq!(persona.normalized_names(), ("Jarvis", "I'm Jarvis"));

        apply_command(&mut persona, &["add".into(), "Instruction 1".into()]).unwrap();
        assert_eq!(persona.instructions.len(), 1);

        apply_command(&mut persona, &["name".into(), "reset".into()]).unwrap();
        assert!(persona.name.is_none());
        assert_eq!(persona.display_name(), "NioAI");
        assert_eq!(persona.instructions.len(), 1);

        apply_command(&mut persona, &["reset".into()]).unwrap();
        assert!(persona.is_empty());
    }

    #[test]
    fn test_apply_flags() {
        let mut persona = PersonaConfig::default();
        apply_command(
            &mut persona,
            &[
                "--preset".into(),
                "partner".into(),
                "--gender".into(),
                "female".into(),
                "-i".into(),
                "Be fast".into(),
            ],
        )
        .unwrap();

        assert_eq!(persona.preset.as_deref(), Some("partner"));
        assert_eq!(persona.name.as_deref(), Some("Riley"));
        assert_eq!(persona.gender.as_deref(), Some("female"));
        assert_eq!(persona.instructions.len(), 4);
        assert_eq!(persona.instructions.last().unwrap(), "Be fast");

        let (_, prompt) = format_persona_prompt(&persona);
        assert!(prompt.contains("she/her"));
    }

    #[test]
    fn test_gender_normalization_and_command() {
        assert_eq!(normalize_gender("female").unwrap(), Some("female".into()));
        assert_eq!(normalize_gender("f").unwrap(), Some("female".into()));
        assert_eq!(normalize_gender("woman").unwrap(), Some("female".into()));
        assert_eq!(normalize_gender("she").unwrap(), Some("female".into()));
        assert_eq!(normalize_gender("male").unwrap(), Some("male".into()));
        assert_eq!(normalize_gender("m").unwrap(), Some("male".into()));
        assert_eq!(normalize_gender("guy").unwrap(), Some("male".into()));
        assert_eq!(normalize_gender("he").unwrap(), Some("male".into()));
        assert_eq!(normalize_gender("non-binary").unwrap(), Some("non-binary".into()));
        assert_eq!(normalize_gender("nb").unwrap(), Some("non-binary".into()));
        assert_eq!(normalize_gender("neutral").unwrap(), Some("non-binary".into()));
        assert_eq!(normalize_gender("reset").unwrap(), None);
        assert_eq!(normalize_gender("none").unwrap(), None);
        assert!(normalize_gender("alien").is_err());

        let mut persona = PersonaConfig::default();
        let res = apply_command(&mut persona, &["gender".into(), "female".into()]).unwrap();
        assert!(res.contains("female"));
        assert_eq!(persona.gender.as_deref(), Some("female"));

        let status = format_persona_status(&persona);
        assert!(status.contains("female (she/her)"));

        apply_command(&mut persona, &["gender".into(), "reset".into()]).unwrap();
        assert!(persona.gender.is_none());
    }

    #[test]
    fn test_persona_serialization() {
        let mut persona = PersonaConfig::default();
        persona.preset = Some("bff".into());
        persona.name = Some("Alex".into());
        persona.gender = Some("female".into());
        persona.instructions = vec!["Instruction A".into()];

        let json = serde_json::to_string(&persona).unwrap();
        let loaded: PersonaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(persona, loaded);
    }
}
