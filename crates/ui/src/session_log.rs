//! Operator session log — story 6-11.
//!
//! ## Why this exists
//!
//! Five defects in this repo's bug log shipped past a green suite (#65-#69). The
//! common thread is not that the tests were weak in some general way — it is that
//! **nothing independent of the tests recorded what the software actually did**. The
//! canonical case is `#66` A.4: `report::sma` emitted its `strategy:` sub-keys
//! unindented, `parse_frontmatter` filed them top-level, and `scan_one_root` therefore
//! skipped EVERY engine-written report. The Compare screen rendered perfectly. An empty
//! Compare matrix and a Compare matrix whose scanner silently rejected 40 reports look
//! identical, and for weeks nobody could tell them apart.
//!
//! So the unit of this log is not a tracing line. It is **an operator-visible action and
//! its outcome, with a denominator**: not "opened Compare", but "opened Compare, found
//! 40 candidate reports, admitted 0, rejected 40 for a missing `strategy.id`".
//!
//! ## What it is not
//!
//! No network call of any kind. No telemetry service, no analytics, no crash reporting,
//! no user identification. One append-only JSONL file per session on the operator's own
//! machine, readable with `cat`. This product's promise is offline honesty and this
//! module must not dent it.
//!
//! ## Off costs nothing
//!
//! The sink is an `Option` on the `Cockpit` (the same switch shape as
//! `lab_state_path`, ADR-0094 D3). `None` — every fixture, gallery and test — means the
//! calls compile to an `Option` check. Only the real binaries arm it, and
//! `TRADING_SESSION_LOG=0` disarms them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Why a discovered candidate was not admitted.
///
/// A CLOSED set, deliberately. An open `String` reason would let a future skip-path be
/// added without anyone deciding it deserves a name, and an unnamed skip path is how
/// `#66` A.4 stayed invisible: the scanner had seven `continue`s and no vocabulary for
/// distinguishing them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rejected {
    /// The file could not be read.
    Unreadable,
    /// Frontmatter absent or unparseable.
    MalformedFrontmatter,
    /// Parsed, but carries no `scenario`.
    MissingScenario,
    /// Parsed, but carries no `strategy.id` — **this is `#66` A.4's bucket**.
    MissingStrategyId,
    /// The `scenario` prefix maps to no known universe.
    UnknownScenario,
    /// Parsed and identified, but the body carries no KPI table to read.
    NoKpiTable,
    /// A higher-priority root already claimed this filename (ADR-0055 § D5 #2).
    DuplicateFilename,
}

/// What a scan found, admitted, and threw away — AC3's "emptiness with a denominator".
///
/// `discovered` counts every candidate the scan CONSIDERED, so `admitted == 0` is always
/// reported against a number. A tally of `0 / 0` says "there was nothing here"; a tally
/// of `0 / 40` says "there was plenty here and I used none of it", and those are
/// different sentences that this repo previously could not say apart.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanTally {
    /// Candidates considered.
    pub discovered: usize,
    /// Candidates that became usable cells.
    pub admitted: usize,
    /// The rest, by reason. Sums to `discovered - admitted`.
    pub rejected: BTreeMap<Rejected, usize>,
}

impl ScanTally {
    /// Record one candidate that was admitted.
    pub const fn admit(&mut self) {
        self.discovered += 1;
        self.admitted += 1;
    }

    /// Record one candidate that was not.
    pub fn reject(&mut self, why: Rejected) {
        self.discovered += 1;
        *self.rejected.entry(why).or_insert(0) += 1;
    }

    /// Did this scan find material and use none of it? The `#66` A.4 shape.
    #[must_use]
    pub const fn found_everything_and_used_nothing(&self) -> bool {
        self.discovered > 0 && self.admitted == 0
    }
}

/// One operator-visible action and its outcome.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// A session began.
    SessionStarted {
        /// The binary that opened it.
        app: &'static str,
    },
    /// The operator navigated to a screen.
    ScreenOpened {
        /// Screen name as the shell knows it.
        screen: String,
    },
    /// A scan completed — with its denominator.
    Scanned {
        /// What was scanned, e.g. `"compare_reports"`.
        what: &'static str,
        /// The denominator.
        tally: ScanTally,
    },
    /// A run was requested, with the inputs that define it.
    RunStarted {
        /// e.g. `"lab"` or `"bakeoff"`.
        kind: &'static str,
        /// Strategy / pair / range / source, whatever the surface actually took.
        inputs: BTreeMap<String, String>,
    },
    /// A run ended. `ok == false` carries the operator-facing message.
    RunFinished {
        /// Matches the `RunStarted` it closes.
        kind: &'static str,
        /// Did it produce a result?
        ok: bool,
        /// The error, when it did not.
        detail: Option<String>,
    },
    /// Something durable was written.
    ArtifactWritten {
        /// e.g. `"report"` or `"forward_plan"`.
        kind: &'static str,
        /// Where it landed.
        path: String,
    },
}

/// Where events go. A trait so the writer is external I/O behind a seam (project rule)
/// and so the acceptance test can read back exactly what was recorded.
pub trait SessionSink: std::fmt::Debug + Send + Sync {
    /// Record one event. Must not panic and must not block the render loop.
    fn record(&self, event: &Event);
}

/// The default sink: one append-only JSONL file per session.
#[derive(Debug)]
pub struct JsonlSink {
    file: Mutex<std::fs::File>,
    path: PathBuf,
}

impl JsonlSink {
    /// Open a session file under `dir`, named by the session's start time.
    ///
    /// # Errors
    /// Returns the `io::Error` if the directory or file cannot be created.
    pub fn create(dir: &Path, started_unix_secs: u64) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        write_readme_once(dir);
        let path = dir.join(format!("session-{started_unix_secs}.jsonl"));
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            file: Mutex::new(file),
            path,
        })
    }

    /// The file this sink appends to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SessionSink for JsonlSink {
    fn record(&self, event: &Event) {
        use std::io::Write;
        let Ok(line) = serde_json::to_string(event) else {
            return;
        };
        let Ok(mut f) = self.file.lock() else {
            // A poisoned lock means another thread panicked mid-write. Losing a log
            // line is the correct trade against propagating that panic into the UI.
            return;
        };
        let _ = writeln!(f, "{line}");
    }
}

/// `$XDG_STATE_HOME/trading/sessions`, defaulting to `~/.local/state/trading/sessions`.
///
/// State, not config: these are records of what happened, not settings, and XDG puts
/// them in different places. The directory is git-ignored.
#[must_use]
pub fn session_dir(override_path: Option<&Path>) -> PathBuf {
    if let Some(p) = override_path {
        return p.to_path_buf();
    }
    let base = std::env::var("XDG_STATE_HOME").map_or_else(
        |_| {
            std::env::var("HOME").map_or_else(
                |_| PathBuf::from(".local/state"),
                |h| PathBuf::from(h).join(".local").join("state"),
            )
        },
        PathBuf::from,
    );
    base.join("trading").join("sessions")
}

/// How many session files are kept. AC5: retention is bounded and stated.
///
/// Thirty sessions is a few weeks of ordinary use at a few KB each. The cap exists so
/// the directory cannot grow without bound on a machine nobody prunes; it is not a
/// privacy control, because nothing leaves the machine in the first place.
pub const KEEP_SESSIONS: usize = 30;

/// Delete all but the newest [`KEEP_SESSIONS`] session files in `dir`.
///
/// Best-effort and silent: a log that cannot tidy itself must not break a boot.
pub fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "jsonl")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("session-"))
        })
        .collect();
    if files.len() <= KEEP_SESSIONS {
        return;
    }
    // Name-sorted is time-sorted: the stem is a zero-padded-free unix second count, so
    // sort by the parsed number rather than lexically (10 < 9 lexically).
    files.sort_by_key(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix("session-"))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    });
    for old in &files[..files.len() - KEEP_SESSIONS] {
        let _ = std::fs::remove_file(old);
    }
}

/// What the operator finds if they open the session directory (AC5).
///
/// Written beside the logs rather than only in a runbook, because the person most likely
/// to wonder what these files are is the person looking at them. Written once — a
/// subsequent boot leaves an edited copy alone.
fn write_readme_once(dir: &Path) {
    let readme = dir.join("README.md");
    if readme.exists() {
        return;
    }
    let _ = std::fs::write(
        &readme,
        format!(
            "# Cockpit session logs\n\
             \n\
             One JSONL file per run of the cockpit, on this machine only.\n\
             \n\
             **Recorded**: which screens were opened; which runs were started and with\n\
             what strategy / pair / range / source; whether each run produced a result or\n\
             an error; what each scan found vs. admitted; which artifacts were written.\n\
             \n\
             **Not recorded**: anything about you, anything typed into a field, any price\n\
             or money figure, and any network destination \u{2014} because nothing here\n\
             touches the network. No telemetry, no analytics, no crash reporting, no\n\
             identifier of any kind.\n\
             \n\
             **Retention**: the newest {KEEP_SESSIONS} sessions are kept; older files are\n\
             deleted at the next boot. Delete any of them at any time \u{2014} nothing reads\n\
             them back.\n\
             \n\
             **Off switch**: start the cockpit with `TRADING_SESSION_LOG=0`.\n\
             \n\
             Read one with `cat`, or `jq -c . session-*.jsonl` if you have jq.\n\
             \n\
             Why this exists: `docs/runbooks/operator-session-log.md` in the repo.\n"
        ),
    );
}

/// Is the log disabled by the environment? AC4's trivial off-switch.
#[must_use]
pub fn disabled_by_env() -> bool {
    std::env::var("TRADING_SESSION_LOG").is_ok_and(|v| v == "0" || v.eq_ignore_ascii_case("off"))
}
