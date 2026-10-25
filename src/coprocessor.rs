use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroAuditResult {
    pub model: String,
    pub action: String,
    pub cluster: Option<String>,
    pub verdict: String,
    pub risk_score: f64,
    pub confidence: f64,
    pub is_safe: bool,
    pub latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniDefectResult {
    pub model: String,
    pub will_break: bool,
    pub category: String,
    pub break_probability: f64,
    pub risk_score: f64,
    pub reason: String,
    pub confidence: f64,
    pub latency_ms: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandAudit {
    pub is_destructive: bool,
    pub requires_confirmation: bool,
    pub reason: String,
    pub risk_score: f64,
    pub latency_ms: f64,
}

/// Audit terminal commands using Viper Destructive Safety Clutch and Micro Policy Arbiter
pub fn audit_command(command: &str) -> CommandAudit {
    let t0 = Instant::now();
    let lower = command.trim().to_lowercase();

    // 1. First-pass ultra-fast heuristic check for known destructive patterns
    let mut is_destructive = false;
    let mut reason = String::new();
    let mut risk_score = 0.05;

    if lower.contains("rm ") && (lower.contains("-rf") || lower.contains("-fr") || lower.contains(" -r ") || lower.contains(" -f ")) {
        is_destructive = true;
        reason = "Recursive or forced file deletion".into();
        risk_score = 0.95;
    } else if lower.contains("drop database") || lower.contains("delete database") || lower.contains("truncate table") {
        is_destructive = true;
        reason = "Irreversible database mutation".into();
        risk_score = 0.99;
    } else if lower.contains("mkfs") || lower.contains("dd if=") || lower.contains(":(){ :|:& };:") {
        is_destructive = true;
        reason = "Filesystem corruption or fork-bomb pattern".into();
        risk_score = 1.0;
    } else if lower.contains("fuser -k") || lower.contains("kill -9") || lower.contains("pkill -9") || lower.contains("taskkill /f") {
        is_destructive = true;
        reason = "Forceful termination of system processes".into();
        risk_score = 0.85;
    } else if lower.contains("docker system prune") || lower.contains("docker volume rm") {
        is_destructive = true;
        reason = "Destructive pruning of Docker containers or persistent volumes".into();
        risk_score = 0.88;
    } else if lower.contains("git reset --hard") || lower.contains("git clean -fd") {
        is_destructive = true;
        reason = "Destructive git operation discarding uncommitted work".into();
        risk_score = 0.80;
    }

    // 2. Query nioxs micro if available for deep policy arbitration
    if let Some(nioxs_bin) = crate::find_nioxs_binary() {
        if let Ok(output) = Command::new(&nioxs_bin)
            .args(["micro", command])
            .output()
        {
            if output.status.success() {
                if let Ok(res) = serde_json::from_slice::<MicroAuditResult>(&output.stdout) {
                    if !res.is_safe || res.verdict == "REJECT" || res.risk_score > 0.60 {
                        is_destructive = true;
                        if reason.is_empty() {
                            reason = format!("Micro policy arbiter flagged risk (score: {:.2})", res.risk_score);
                        }
                        risk_score = res.risk_score.max(risk_score);
                    }
                }
            }
        }
    }

    let latency_ms = t0.elapsed().as_secs_f64() * 1000.0;
    CommandAudit {
        is_destructive,
        requires_confirmation: is_destructive,
        reason,
        risk_score,
        latency_ms,
    }
}

/// Sanitize input strings using Mask Real-Time Redactor
pub fn sanitize_prompt(prompt: &str) -> (String, usize) {
    if let Some(nioxs_bin) = crate::find_nioxs_binary() {
        if let Ok(output) = Command::new(&nioxs_bin)
            .args(["mask", prompt])
            .output()
        {
            if output.status.success() {
                let sanitized = String::from_utf8_lossy(&output.stdout).trim_end().to_string();
                let secret_markers = sanitized.matches("[REDACTED_SECRET]").count();
                if secret_markers > 0 {
                    return (sanitized, secret_markers);
                }
            }
        }
    }

    // Fast built-in token scrubber fallback
    let mut scrubbed = prompt.to_string();
    let mut count = 0;
    // AWS Access Key ID
    if scrubbed.contains("AKIA") {
        let re = regex_lite::Regex::new(r"AKIA[0-9A-Z]{16}").unwrap();
        if re.is_match(&scrubbed) {
            scrubbed = re.replace_all(&scrubbed, "[REDACTED_SECRET]").to_string();
            count += 1;
        }
    }
    // Generic Bearer / JWT
    if scrubbed.contains("eyJ") {
        let re = regex_lite::Regex::new(r"eyJ[a-zA-Z0-9_-]{10,}\.eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}").unwrap();
        if re.is_match(&scrubbed) {
            scrubbed = re.replace_all(&scrubbed, "[REDACTED_SECRET]").to_string();
            count += 1;
        }
    }

    (scrubbed, count)
}

/// Run pre-flight defect prediction on diffs and code patches using Mini Defect Forecaster
pub fn predict_diff_defects(diff: &str) -> Option<MiniDefectResult> {
    if let Some(nioxs_bin) = crate::find_nioxs_binary() {
        if let Ok(output) = Command::new(&nioxs_bin)
            .args(["mini", diff])
            .output()
        {
            if output.status.success() {
                if let Ok(res) = serde_json::from_slice::<MiniDefectResult>(&output.stdout) {
                    return Some(res);
                }
            }
        }
    }
    None
}
