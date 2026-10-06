//! NioDB integration client for Nio.
//!
//! Provides communication with the local or remote NioDB instance (default: http://127.0.0.1:7432),
//! handling authentication tokens, session synchronization, context handoff manifests,
//! dead-ends logging, and swarm task queues.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct NioDbClient {
    pub base_url: String,
    pub token: Option<String>,
    client: reqwest::Client,
}

impl NioDbClient {
    pub fn new() -> Self {
        let base_url = env::var("NIODB_URL")
            .or_else(|_| env::var("NIODB_LISTEN").map(|l| {
                if l.starts_with("http://") || l.starts_with("https://") {
                    l
                } else {
                    format!("http://{l}")
                }
            }))
            .unwrap_or_else(|_| "http://127.0.0.1:7432".to_string());

        let token = find_token();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            base_url,
            token,
            client,
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(token) = &self.token {
            if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
                headers.insert(AUTHORIZATION, val);
            }
        }
        headers
    }

    pub async fn is_healthy(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    pub async fn create_session(
        &self,
        title: &str,
        agent: &str,
        model: Option<&str>,
        swarm_type: Option<&str>,
        goal: Option<&str>,
        agents_config: Option<Value>,
    ) -> Result<Value, String> {
        let url = format!("{}/api/v1/sessions", self.base_url);
        let mut body = json!({
            "title": title,
            "agent": agent,
            "model": model.unwrap_or(agent),
        });

        if let Some(st) = swarm_type {
            body["swarm_type"] = json!(st);
        }
        if let Some(g) = goal {
            body["goal"] = json!(g);
        }
        if let Some(ac) = agents_config {
            body["agents"] = ac;
        }

        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to create session: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))
    }

    pub async fn list_sessions(&self) -> Result<Vec<Value>, String> {
        let url = format!("{}/api/v1/sessions", self.base_url);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Failed to list sessions: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        let val: Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {e}"))?;

        if let Some(items) = val.get("items").and_then(|i| i.as_array()) {
            Ok(items.clone())
        } else {
            Ok(vec![])
        }
    }

    pub async fn get_session(&self, id: &str) -> Result<Value, String> {
        let url = format!("{}/api/v1/sessions/{}", self.base_url, id);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Failed to fetch session: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse session: {e}"))
    }

    pub async fn switch_agent(
        &self,
        id: &str,
        to_agent: &str,
        model: Option<&str>,
        reason: Option<&str>,
    ) -> Result<Value, String> {
        let url = format!("{}/api/v1/sessions/{}/switch", self.base_url, id);
        let mut body = json!({
            "to_agent": to_agent,
            "model": model.unwrap_or(to_agent),
        });
        if let Some(r) = reason {
            body["reason"] = json!(r);
        }

        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to switch agent: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse switch response: {e}"))
    }

    pub async fn append_turn(
        &self,
        id: &str,
        agent: &str,
        model: Option<&str>,
        summary: &str,
        files_touched: &[String],
    ) -> Result<Value, String> {
        let url = format!("{}/api/v1/sessions/{}/turns", self.base_url, id);
        let body = json!({
            "agent": agent,
            "model": model.unwrap_or(agent),
            "summary": summary,
            "files_touched": files_touched,
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to append turn: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse append response: {e}"))
    }

    pub async fn get_manifest(&self, id: &str) -> Result<Value, String> {
        let url = format!("{}/api/v1/sessions/{}/manifest", self.base_url, id);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Failed to get manifest: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse manifest: {e}"))
    }

    pub async fn add_dead_end(
        &self,
        id: &str,
        issue: &str,
        attempt: &str,
        why: &str,
    ) -> Result<Value, String> {
        let url = format!("{}/api/v1/sessions/{}/dead-ends", self.base_url, id);
        let body = json!({
            "issue": issue,
            "attempt": attempt,
            "why": why,
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to record dead-end: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse dead-end response: {e}"))
    }

    pub async fn create_task(
        &self,
        session_id: &str,
        title: &str,
        role: &str,
        payload: Value,
    ) -> Result<Value, String> {
        let url = format!("{}/api/v1/tasks", self.base_url);
        let body = json!({
            "session_id": session_id,
            "title": title,
            "role": role,
            "payload": payload,
        });

        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to create task: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse create task response: {e}"))
    }

    pub async fn list_tasks(&self, session_id: &str) -> Result<Vec<Value>, String> {
        let url = format!("{}/api/v1/tasks?session_id={}", self.base_url, session_id);
        let resp = self
            .client
            .get(&url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|e| format!("Failed to list tasks: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        let val: Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse tasks response: {e}"))?;

        if let Some(items) = val.get("items").and_then(|i| i.as_array()) {
            Ok(items.clone())
        } else {
            Ok(vec![])
        }
    }

    #[allow(dead_code)]
    pub async fn claim_task(&self, task_id: &str, agent: &str) -> Result<Value, String> {
        let url = format!("{}/api/v1/tasks/{}/claim", self.base_url, task_id);
        let body = json!({ "agent": agent });

        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to claim task: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse claim task response: {e}"))
    }

    #[allow(dead_code)]
    pub async fn complete_task(&self, task_id: &str, result: Value) -> Result<Value, String> {
        let url = format!("{}/api/v1/tasks/{}/complete", self.base_url, task_id);
        let body = json!({ "result": result });

        let resp = self
            .client
            .post(&url)
            .headers(self.headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Failed to complete task: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("NioDB error ({status}): {text}"));
        }

        resp.json::<Value>()
            .await
            .map_err(|e| format!("Failed to parse complete task response: {e}"))
    }
}

fn find_token() -> Option<String> {
    if let Ok(t) = env::var("NIODB_TOKEN") {
        let t = t.trim().to_string();
        if !t.is_empty() {
            return Some(t);
        }
    }

    let mut candidate_paths = vec![
        PathBuf::from("./nio-db/client-token"),
        PathBuf::from("./nio-db/secret-token"),
    ];

    if let Ok(home) = env::var("HOME") {
        candidate_paths.push(PathBuf::from(&home).join(".nio-db/client-token"));
        candidate_paths.push(PathBuf::from(&home).join(".nio-db/secret-token"));
        candidate_paths.push(PathBuf::from(&home).join("nio-labs/nio-db/nio-db/client-token"));
        candidate_paths.push(PathBuf::from(&home).join("nio-labs/nio-db/nio-db/secret-token"));
    }

    for path in candidate_paths {
        if let Ok(content) = fs::read_to_string(&path) {
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }

    None
}
