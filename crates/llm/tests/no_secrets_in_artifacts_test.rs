//! T1926 — V9 secrets-in-artifacts gate.
//!
//! Composes the smoke harness against fixture API keys that look
//! real (`sk-ant-V9-secretkey-12345678`,
//! `sk-V9-OpenAI-secretkey-87654321`), drives one round-trip per
//! provider, then scans every artifact the smoke run could have
//! written. The gate asserts **zero substrings** of the two fixture
//! keys land in any scanned artifact.
//!
//! ## Why the pattern list still lives in the shell script
//!
//! The grep gate also fires standalone
//! (`bash scripts/check_no_secrets_in_llm_artifacts.sh`) against CI
//! artifacts that don't come from a Rust test run — presenter
//! screenshots, archived backtests, dev logs. The pattern list must
//! therefore have exactly one home, and that home is the script.
//!
//! This test does **not** duplicate it: [`SecretGate::parse`] READS
//! `PATTERNS=(…)` and `SK_RE=…` out of the script at runtime, so the
//! test and CI cannot drift by construction. If someone edits the
//! script's `SK_RE` without updating the matcher here, the test fails
//! loudly with both spellings rather than scanning for the wrong thing.
//!
//! ## Why the scan is Rust and not `bash` (2026-09-24, story 6-9)
//!
//! Until now this test shelled out via
//! `std::process::Command::new("bash")`. That is unrunnable on the
//! `windows-latest` CI leg: `bash` resolution from a native process is
//! not guaranteed there, and the script's two workhorses — `strings(1)`
//! (binutils, absent from Git-for-Windows) and GNU `find` (shadowed by
//! `C:\Windows\System32\find.exe` when Git's `usr/bin` is off PATH) —
//! are not guaranteed either. A `strings`-less run is worse than a
//! failure: every `strings … | grep -q` pipeline then reports "no
//! match", so the gate would go GREEN having scanned nothing.
//!
//! The Rust scan is a strict SUPERSET of the shell one: it reads raw
//! bytes, so it also catches a pattern that `strings(1)` would have
//! dropped (printable runs shorter than 4) or that `grep`'s
//! line-orientation would have split. [`SecretGate::self_check`] proves
//! on every run that the scanner actually detects both fixture keys —
//! a positive control the shell gate never had.
//!
//! On unix the script itself is ALSO executed, unchanged, so its own
//! plumbing (`find` / `grep` / `strings` wiring) stays exercised.

use async_trait::async_trait;
use cost::{AgentRole, LlmTier};
use llm::{
    AnthropicProvider, ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmError, LlmProvider,
    MessageRole, ModelId, OpenAiProvider, ProviderKind, RecordingProvider,
};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ANT_KEY: &str = "sk-ant-V9-secretkey-12345678";
const OAI_KEY: &str = "sk-V9-OpenAI-secretkey-87654321";

/// In-memory canned-response provider for the local-recording leg.
struct CannedLeaf(ChatResponse, ProviderKind, &'static str);
#[async_trait]
impl LlmProvider for CannedLeaf {
    fn name(&self) -> &str {
        self.2
    }
    fn provider_kind(&self) -> ProviderKind {
        self.1.clone()
    }
    async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
        Ok(self.0.clone())
    }
}

fn canned() -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::Text("OK".to_string())],
        stop_reason: llm::StopReason::EndTurn,
        usage: llm::TokenUsage {
            tokens_in: 5,
            tokens_out: 1,
            tokens_cached_in: 0,
        },
        model: ModelId::new("claude-opus-4-7"),
        correlation_id: Uuid::nil(),
    }
}

/// Spawn an Anthropic-shaped mock that REQUIRES `x-api-key: <ANT_KEY>`
/// — proving the key crosses the wire — and returns canned `OK`.
async fn spawn_ant_mock() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_v9",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-7",
            "content": [{"type": "text", "text": "OK"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 5,
                "output_tokens": 1,
                "cache_read_input_tokens": 0
            }
        })))
        .mount(&s)
        .await;
    s
}

async fn spawn_oai_mock() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-v9",
            "object": "chat.completion",
            "model": "gpt-5",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "OK"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 1, "total_tokens": 6}
        })))
        .mount(&s)
        .await;
    s
}

#[tokio::test]
async fn t1926_no_secrets_in_artifacts() {
    let td = tempfile::tempdir().expect("tempdir");
    let db_path = td.path().join("replay-v9.db");

    // 1. Stand up two wiremock servers that the leaf providers talk to
    //    with the fixture keys. Even if the key crosses the wire,
    //    nothing inside this test ever writes the key to a persistent
    //    artifact.
    let ant_srv = spawn_ant_mock().await;
    let oai_srv = spawn_oai_mock().await;
    let ant =
        AnthropicProvider::with_base_url(ant_srv.uri(), ANT_KEY, ModelId::new("claude-opus-4-7"));
    let oai = OpenAiProvider::new_with_base_url(oai_srv.uri(), OAI_KEY, ModelId::new("gpt-5"));

    // 2. Wrap each in a RecordingProvider that writes to the same
    //    tempfile cache.
    let ant_rec = RecordingProvider::open(ant, &db_path).await.unwrap();
    let mut req = ChatRequest::new(
        ModelId::new("claude-opus-4-7"),
        LlmTier::DeepThink,
        AgentRole::Trader,
    );
    req.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("smoke-v9".into())],
    });
    ant_rec.complete(req).await.expect("ant record");
    drop(ant_rec);

    let oai_rec = RecordingProvider::open(oai, &db_path).await.unwrap();
    let mut req = ChatRequest::new(ModelId::new("gpt-5"), LlmTier::DeepThink, AgentRole::Trader);
    req.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("smoke-v9-oai".into())],
    });
    oai_rec.complete(req).await.expect("oai record");
    drop(oai_rec);

    // Use the alternate canned leaf to seed a recording row whose
    // wire body never carried a key — covers the "in-process leaf"
    // path that bypasses HTTP entirely.
    let leaf = CannedLeaf(canned(), ProviderKind::Anthropic, "canned");
    let rec = RecordingProvider::open(leaf, &db_path).await.unwrap();
    let mut req = ChatRequest::new(
        ModelId::new("claude-opus-4-7"),
        LlmTier::QuickThink,
        AgentRole::SentimentAnalyst,
    );
    req.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("smoke-v9-canned".into())],
    });
    rec.complete(req).await.expect("canned record");
    drop(rec);

    // 3. Scan the tempdir's artifacts with the V9 pattern list, read
    //    out of the shell gate. Every artifact path is the per-test
    //    scratch so this test never reads the operator's live DBs.
    let script = v9_script_path();
    assert!(
        script.exists(),
        "V9 grep script missing at {}",
        script.display()
    );
    let gate = SecretGate::parse(
        &std::fs::read_to_string(&script).expect("read V9 grep script"),
        &script,
    );

    // Positive + negative control. A gate that cannot fail is
    // indistinguishable from a gate that passed: prove on THIS run
    // that the scanner detects both fixture keys and stays quiet on
    // innocuous bytes, before trusting its verdict on the artifacts.
    gate.self_check(ANT_KEY, OAI_KEY);

    // The tempdir is simultaneously the replay DB's directory, the
    // `--log-dir` and the `--fixtures-dir` of the shell invocation
    // below, so one recursive walk covers all three legs — plus the
    // `-wal` / `-shm` sidecars, which the shell's `-name '*.db'`
    // fixtures glob never looked at. No audit DB is written by this
    // test (the shell leg passes `--audit-db /dev/null`), so there is
    // nothing else to scan.
    let mut hits = Vec::new();
    let scanned = gate.scan_tree(td.path(), &mut hits);
    assert!(
        hits.is_empty(),
        "V9 FAIL: {} secret pattern(s) found in artifacts:\n  {}",
        hits.len(),
        hits.join("\n  ")
    );
    // "No hits" must mean "scanned and clean", never "scanned nothing".
    assert!(
        db_path.is_file(),
        "replay DB vanished before the scan: {}",
        db_path.display()
    );
    assert!(
        scanned > 0,
        "the V9 walk visited 0 files under {} — an empty-set result, not a clean one",
        td.path().display()
    );

    // 3b. End-to-end control for the WALK, not just the matcher: plant
    //     a leaked key in a throwaway tree and require the same code
    //     path to find it. Deliberately a SEPARATE tempdir so the
    //     artifact scan above and the shell leg below never see it.
    {
        let decoy_dir = tempfile::tempdir().expect("decoy tempdir");
        let nested = decoy_dir.path().join("logs").join("run-1");
        std::fs::create_dir_all(&nested).expect("decoy dirs");
        std::fs::write(nested.join("agent.log"), format!("auth={ANT_KEY}\n")).expect("write decoy");
        let mut decoy_hits = Vec::new();
        let decoy_scanned = gate.scan_tree(decoy_dir.path(), &mut decoy_hits);
        assert_eq!(
            decoy_scanned, 1,
            "decoy walk did not reach the planted file"
        );
        assert!(
            !decoy_hits.is_empty(),
            "the V9 walk did not flag a planted `{ANT_KEY}` — the gate cannot fail"
        );
    }

    // 4. Unix only: run the shell gate verbatim too, so the script's
    //    own `find` / `grep` / `strings` plumbing stays exercised.
    //    This is an ADDITIONAL assertion, not the gate — the Rust scan
    //    above is what runs on all three CI legs. What is therefore
    //    unproven on windows-latest is the SCRIPT's plumbing, never the
    //    pattern list (read from the script above) nor the artifacts.
    #[cfg(unix)]
    {
        let status = std::process::Command::new("bash")
            .arg(&script)
            .arg("--db")
            .arg(&db_path)
            .arg("--log-dir")
            .arg(td.path()) // tempdir itself — no log files inside
            .arg("--audit-db")
            .arg("/dev/null")
            .arg("--fixtures-dir")
            .arg(td.path())
            .current_dir(repo_root())
            .status()
            .expect("invoke V9 grep script");

        assert!(
            status.success(),
            "V9 grep script failed (exit code {:?}); see stderr for hits",
            status.code()
        );
    }
}

/// Resolve the repo root from `CARGO_MANIFEST_DIR` (= `crates/llm`) by
/// going up two parents.
fn repo_root() -> std::path::PathBuf {
    let manifest =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repo root")
        .to_path_buf()
}

/// Path to the single source of truth for the V9 pattern list.
fn v9_script_path() -> std::path::PathBuf {
    repo_root()
        .join("scripts")
        .join("check_no_secrets_in_llm_artifacts.sh")
}

// ── The V9 gate, sourced from the shell script ───────────────────────────────

/// The `SK_RE` spelling this matcher implements. It is NOT the source
/// of truth — the script is — but the matcher is hand-rolled, so a
/// change to the script's regex must be mirrored here deliberately.
/// [`SecretGate::parse`] refuses to run on any other spelling.
const SUPPORTED_SK_RE: &str = "sk-[A-Za-z0-9_-]{12,}";

/// The V9 secret patterns, parsed out of
/// `scripts/check_no_secrets_in_llm_artifacts.sh` at runtime.
struct SecretGate {
    /// `PATTERNS=(…)` — matched case-INSENSITIVELY, mirroring the
    /// script's `grep -i -F`. Stored lowercased.
    literals: Vec<String>,
    /// The raw `SK_RE` spelling, kept for error messages.
    sk_re: String,
}

impl SecretGate {
    /// Extract `PATTERNS=(…)` and `SK_RE=…` from the script source.
    ///
    /// Deliberately strict: an unparseable script is a hard failure,
    /// never a silently empty pattern list.
    fn parse(src: &str, script: &std::path::Path) -> Self {
        let mut literals = Vec::new();
        let mut in_array = false;
        let mut sk_re: Option<String> = None;

        for line in src.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            if in_array {
                if trimmed.starts_with(')') {
                    in_array = false;
                } else if let Some(v) = unquote(trimmed, '"') {
                    literals.push(v.to_ascii_lowercase());
                }
                continue;
            }
            if trimmed == "PATTERNS=(" {
                in_array = true;
            } else if let Some(rest) = trimmed.strip_prefix("SK_RE=")
                && let Some(v) = unquote(rest, '\'')
            {
                sk_re = Some(v.to_string());
            }
        }

        assert!(
            !in_array,
            "unterminated PATTERNS=( array in {}",
            script.display()
        );
        let sk_re = sk_re.unwrap_or_else(|| panic!("no SK_RE=… line in {}", script.display()));
        assert_eq!(
            sk_re,
            SUPPORTED_SK_RE,
            "{} changed SK_RE to `{sk_re}` but this test's hand-rolled matcher \
             implements `{SUPPORTED_SK_RE}`. Update SUPPORTED_SK_RE and \
             `sk_shape_hit` together, or the gate scans for the wrong thing.",
            script.display()
        );
        assert!(
            literals.len() >= 8,
            "parsed only {} V9 patterns from {} — expected the full list; \
             a silently-shrunk pattern set is a vacuous gate",
            literals.len(),
            script.display()
        );

        Self { literals, sk_re }
    }

    /// Every hit `bytes` would produce, described for the failure message.
    fn hits(&self, bytes: &[u8], label: &str) -> Vec<String> {
        let mut out = Vec::new();
        let lower = bytes.to_ascii_lowercase();
        for pat in &self.literals {
            if contains(&lower, pat.as_bytes()) {
                out.push(format!("{label} contains literal '{pat}'"));
            }
        }
        // `grep -E` without `-i` in the script — case-SENSITIVE.
        if sk_shape_hit(bytes) {
            out.push(format!("{label} contains sk-key-shape ({})", self.sk_re));
        }
        out
    }

    /// Recursively scan every file under `root`; returns the number of
    /// files actually read, so callers can prove the walk was not empty.
    fn scan_tree(&self, root: &std::path::Path, out: &mut Vec<String>) -> usize {
        let mut scanned = 0usize;
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                match entry.file_type() {
                    Ok(ft) if ft.is_dir() => stack.push(path),
                    Ok(ft) if ft.is_file() => {
                        let bytes = std::fs::read(&path)
                            .unwrap_or_else(|e| panic!("read artifact {}: {e}", path.display()));
                        scanned += 1;
                        out.extend(self.hits(&bytes, &path.display().to_string()));
                    }
                    _ => {}
                }
            }
        }
        scanned
    }

    /// Positive + negative control for the scanner itself.
    fn self_check(&self, ant_key: &str, oai_key: &str) {
        for key in [ant_key, oai_key] {
            assert!(
                !self.hits(key.as_bytes(), "self-check").is_empty(),
                "V9 scanner does not detect the fixture key `{key}` — the gate \
                 cannot fail, so a green run proves nothing"
            );
            // Casing variations must match too (`grep -i` in the script).
            assert!(
                !self
                    .hits(key.to_ascii_uppercase().as_bytes(), "self-check")
                    .is_empty(),
                "V9 scanner is case-sensitive on `{key}` — the script greps -i"
            );
        }
        assert!(
            self.hits(b"nothing to see here, just a bar and a key", "self-check")
                .is_empty(),
            "V9 scanner fires on innocuous bytes — it would mask a real hit in noise"
        );
    }
}

/// Strip a matching leading/trailing `q` from `s`, if present.
fn unquote(s: &str, q: char) -> Option<&str> {
    s.strip_prefix(q)?.split(q).next()
}

/// Naive substring search over bytes (artifacts here are ≤ a few hundred KB).
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// `sk-[A-Za-z0-9_-]{12,}` — the script's `SK_RE`, case-sensitive.
fn sk_shape_hit(bytes: &[u8]) -> bool {
    bytes.windows(3).enumerate().any(|(i, w)| {
        w == b"sk-"
            && bytes[i + 3..]
                .iter()
                .take_while(|b| b.is_ascii_alphanumeric() || **b == b'_' || **b == b'-')
                .count()
                >= 12
    })
}
