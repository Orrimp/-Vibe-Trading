//! bug-log #205 — the ONE report-root layout contract (ADR-0055 amendment D8).
//!
//! The `ui` readers named below walk a report root (`evidence/`, `lab-runs/`,
//! a test tempdir) through this module, so the depth rule lives in one place:
//!
//! ```text
//! <root>[/<container>]/<slug>/reports/backtest-*.md
//! ```
//!
//! - `<root>/<slug>/reports/` — container depth 0 (`lab-runs/<slug>/reports`,
//!   and the committed dirs directly under `evidence/`, e.g.
//!   `evidence/v5-*/reports`).
//! - `<root>/<container>/<slug>/reports/` — container depth 1 (the committed
//!   layout since the `spec/v1` reorganisation `1395520629dc`, e.g.
//!   `evidence/v1/<slug>/reports`).
//! - Nothing deeper is read: [`MAX_CONTAINER_DEPTH`] is the bound.
//! - A directory named [`REPORTS_DIR`] is never a container and never a slug,
//!   so the walk never descends into `reports/artifacts/…`.
//! - The slug is the LEAF directory name directly above `reports/`; the full
//!   path (which carries the container) stays in the caller's `PathBuf`.
//! - **Order (the D8 tie rule).** Report dirs come in (container depth, path)
//!   order — every container-depth-0 dir before any container-depth-1 dir, then
//!   by path — and the files of one dir by path. A reader that keeps the FIRST
//!   of equal candidates — the Lab on an equal score; Compare on an identical
//!   filename or an equal `generated:` (bug-log #206) — therefore keeps a
//!   container-depth-0 report on a tie within one root, whether the container
//!   name sorts before or after the slug name.
//! - **Symlinks are followed** (`Path::is_dir` follows them): a symlinked slug
//!   dir or container is walked like a real one, so a report outside the root
//!   can be listed through it. A symlink loop (`loop -> <root>`) is walked at
//!   most [`MAX_CONTAINER_DEPTH`] levels deep, so the walk terminates; it can
//!   list one real dir under two paths. Pinned by LY5 (unix).
//!
//! Readers: the Reports picker (`reports::loader::discover_reports_in`), the
//! Lab's report lookup (`lab::equity_loader`) and, since bug-log #206,
//! Compare's scan (`compare::cache::scan_one_root`).
//!
//! **K2 never-panic.** An absent or unreadable directory contributes nothing
//! (a `tracing` breadcrumb, never an error, never a panic).

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use smol_str::SmolStr;

/// Container levels walked above `<slug>/reports`: 0 = `<root>/<slug>/reports`;
/// 1 = `<root>/<container>/<slug>/reports` (the committed layout since
/// `1395520629dc`). Raising it amends ADR-0055 D8.
pub const MAX_CONTAINER_DEPTH: usize = 1;

/// The directory name that holds reports; never a container, never a slug.
pub const REPORTS_DIR: &str = "reports";

/// One in-contract `reports/` directory found under a root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportDir {
    /// The leaf directory name directly above `reports/` (the slug).
    pub slug: SmolStr,
    /// Container depth: 0 = `<root>/<slug>/reports`, 1 =
    /// `<root>/<container>/<slug>/reports`. The FIRST sort key of
    /// [`report_dirs`] (the D8 tie rule).
    pub container_depth: usize,
    /// The full path of the `reports/` directory itself.
    pub dir: PathBuf,
}

/// Sorted child directories of `dir`, never including one named
/// [`REPORTS_DIR`]. An absent or unreadable `dir` yields an empty list.
fn child_dirs(dir: &Path) -> Vec<PathBuf> {
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            // An absent root (a fresh checkout has no `lab-runs/`) is the
            // normal case and the Lab reaches this from `view`, so it stays at
            // trace; anything else is worth a debug breadcrumb.
            if e.kind() == std::io::ErrorKind::NotFound {
                tracing::trace!(path = %dir.display(), "report layout: dir absent — skipped");
            } else {
                tracing::debug!(
                    path = %dir.display(),
                    error = %e,
                    "report layout: dir unreadable — skipped"
                );
            }
            return Vec::new();
        }
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.file_name() != Some(OsStr::new(REPORTS_DIR)))
        .collect();
    out.sort();
    out
}

/// `(container depth, container)` for `root` (depth 0) plus every directory up
/// to [`MAX_CONTAINER_DEPTH`] levels below it (never through a
/// [`REPORTS_DIR`]), in breadth-first, per-level sorted order.
fn containers(root: &Path) -> Vec<(usize, PathBuf)> {
    let mut all = vec![(0, root.to_path_buf())];
    let mut level = vec![root.to_path_buf()];
    for depth in 1..=MAX_CONTAINER_DEPTH {
        level = level.iter().flat_map(|d| child_dirs(d.as_path())).collect();
        all.extend(level.iter().map(|d| (depth, d.clone())));
    }
    all
}

/// Every in-contract `reports/` directory under `root`, in (container depth,
/// path) order (the D8 tie rule — see the module doc).
///
/// A `<container>/<slug>/reports` directory is listed when `<container>` is
/// `root` or sits at most [`MAX_CONTAINER_DEPTH`] levels below it, and neither
/// `<container>` nor `<slug>` is a [`REPORTS_DIR`] directory.
#[must_use]
pub fn report_dirs(root: &Path) -> Vec<ReportDir> {
    let mut out = Vec::new();
    for (container_depth, container) in containers(root) {
        for slug_dir in child_dirs(&container) {
            let dir = slug_dir.join(REPORTS_DIR);
            let Some(slug) = slug_dir.file_name().and_then(OsStr::to_str) else {
                continue;
            };
            if dir.is_dir() {
                out.push(ReportDir {
                    slug: SmolStr::new(slug),
                    container_depth,
                    dir,
                });
            }
        }
    }
    out.sort_by(|a, b| (a.container_depth, &a.dir).cmp(&(b.container_depth, &b.dir)));
    out
}

/// True when `slug` is exactly one plain path component — its single
/// `Component::Normal` round-trips to the slug string itself — other than
/// [`REPORTS_DIR`]. So `""`, `"."`, `".."`, `"a/b"`, absolute paths, a leading
/// `"./"` (a `CurDir` component), and the spellings `Path::components`
/// normalises to one component (`"sma/"`, `"sma//"`, `"sma/."`) can never
/// reach a directory the walk would not list under that slug.
///
/// Not folded: on a case-insensitive filesystem (APFS, NTFS by default) a case
/// variant of a slug (`"SMA"`) resolves the same directory, and on Windows so
/// does a trailing dot (`"sma."`); [`report_dirs`] lists the on-disk spelling
/// only. No reader passes one: the Lab's `strategy_slug` maps known ids to
/// lowercase constants and passes other ids through verbatim, and every id the
/// cockpit defines is lowercase with no trailing dot.
fn is_slug_component(slug: &str) -> bool {
    let mut comps = Path::new(slug).components();
    matches!(
        (comps.next(), comps.next()),
        (Some(Component::Normal(c)), None) if c == OsStr::new(slug) && c != OsStr::new(REPORTS_DIR)
    )
}

/// Targeted lookup for the Lab (it runs per `view`): the `reports/` dirs of
/// `slug` under `root`, in (container depth, path) order. Equals
/// [`report_dirs`] filtered to `slug`, order included (pinned by LY3), without
/// listing every other slug — up to the filesystem caveats in
/// [`is_slug_component`]'s doc.
#[must_use]
pub fn report_dirs_for_slug(root: &Path, slug: &str) -> Vec<PathBuf> {
    if !is_slug_component(slug) {
        return Vec::new();
    }
    let mut out: Vec<(usize, PathBuf)> = containers(root)
        .into_iter()
        .map(|(depth, c)| (depth, c.join(slug).join(REPORTS_DIR)))
        .filter(|(_, d)| d.is_dir())
        .collect();
    // Tuple order = (container depth, path): the D8 tie rule.
    out.sort();
    out.into_iter().map(|(_, d)| d).collect()
}

/// The `backtest-*.md` filename filter: names that start with `backtest-` and
/// end with the lowercase `.md` extension. Excludes the `robustness-sweep-*.md`
/// and `test-*.md` families by construction.
#[must_use]
pub fn is_backtest_report(name: &str) -> bool {
    // Writers emit a lowercase `.md` extension (the `reports` crate's contract).
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    let md = name.ends_with(".md");
    name.starts_with("backtest-") && md
}

/// The `backtest-*.md` FILES directly inside `dir` (never a subdirectory's),
/// sorted by path. An absent or unreadable `dir` yields an empty list.
#[must_use]
pub fn backtest_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        tracing::debug!(path = %dir.display(), "report layout: reports/ dir unreadable — skipped");
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(OsStr::to_str)
                    .is_some_and(is_backtest_report)
        })
        .collect();
    // The entries of one dir are distinct paths, so unstable = stable here.
    out.sort_unstable();
    out
}

/// The shared fixture tree for the layout and discovery tests (bug-log #205).
/// Each row is a path relative to the fixture root and what the contract says
/// about it; every expected value in those tests is derived from this table.
#[cfg(test)]
pub(crate) mod test_tree {
    use std::path::Path;

    /// One fixture row.
    pub(crate) struct Row {
        /// Path relative to the fixture root (`/`-separated).
        pub(crate) path: &'static str,
        /// `true` = create a DIRECTORY at `path`; `false` = a file.
        pub(crate) is_dir: bool,
        /// `Some((slug, has_companion))` when the contract lists this file as
        /// a discovered report; `None` otherwise.
        pub(crate) listed: Option<(&'static str, bool)>,
    }

    /// The fixture table.
    pub(crate) const TREE: &[Row] = &[
        // Its stem sorts AFTER every `sma` stem, so the (slug, stem, path)
        // order puts it first only if the slug key is honoured (falsifier O9).
        Row {
            path: "flat/reports/backtest-20261231-000000-a.md",
            is_dir: false,
            listed: Some(("flat", false)),
        },
        Row {
            path: "flat/reports/test-week2-smoke.md",
            is_dir: false,
            listed: None,
        },
        Row {
            path: "flat/reports/robustness-sweep-20260101.md",
            is_dir: false,
            listed: None,
        },
        Row {
            path: "flat/reports/backtest-x.txt",
            is_dir: false,
            listed: None,
        },
        Row {
            path: "v1/sma/reports/backtest-20260505-000000-late.md",
            is_dir: false,
            listed: Some(("sma", true)),
        },
        // The companion of the row above (its stem-matched artifacts dir).
        Row {
            path: "v1/sma/reports/artifacts/backtest-20260505-000000-late/equity-1.csv",
            is_dir: false,
            listed: None,
        },
        Row {
            path: "v1/sma/reports/artifacts/backtest-20260505-000000-late/backtest-decoy.md",
            is_dir: false,
            listed: None,
        },
        // Same leaf as the v1 row; its PATH sorts after v1's but its STEM
        // sorts before — the (slug, stem, path) order is observable.
        Row {
            path: "v2/sma/reports/backtest-20260101-000000-early.md",
            is_dir: false,
            listed: Some(("sma", false)),
        },
        // The same slug at container depth 0 ...
        Row {
            path: "sma/reports/backtest-20260404-000000-d0.md",
            is_dir: false,
            listed: Some(("sma", false)),
        },
        // ... and under a container whose name sorts BEFORE `flat` and `sma`,
        // so (container depth, path) order differs from full-path order.
        Row {
            path: "a0/sma/reports/backtest-20260202-000000-a0.md",
            is_dir: false,
            listed: Some(("sma", false)),
        },
        // A twin of the container-depth-0 `d0` row: same slug AND stem. The
        // Reports sort's path key puts THIS copy first, where the layout's
        // (container depth, path) order would put the container-depth-0 copy
        // first (falsifier Q6). It also gives `a0/sma/reports` two listed
        // files, so LY4 sees the order within one dir (falsifier Q3).
        Row {
            path: "a0/sma/reports/backtest-20260404-000000-d0.md",
            is_dir: false,
            listed: Some(("sma", false)),
        },
        Row {
            path: "v1/sma/presentations/backtest-deck.md",
            is_dir: false,
            listed: None,
        },
        // Container depth 2 — out of contract.
        Row {
            path: "v9/x/too-deep/reports/backtest-20260303-000000-c.md",
            is_dir: false,
            listed: None,
        },
        // `reports` is never a container.
        Row {
            path: "reports/inner/reports/backtest-20260606-000000-r.md",
            is_dir: false,
            listed: None,
        },
        Row {
            path: "reports/backtest-20260707-000000-root.md",
            is_dir: false,
            listed: None,
        },
        // `<root>/reports/reports` exists, but `reports` is never a slug
        // either: the walk never lists it (`child_dirs` drops `reports`), and
        // `report_dirs_for_slug(root, "reports")` must agree — the
        // `REPORTS_DIR` half of `is_slug_component` (falsifier Q8).
        Row {
            path: "reports/reports/backtest-20260909-000000-rr.md",
            is_dir: false,
            listed: None,
        },
        Row {
            path: "backtest-20260808-000000-loose.md",
            is_dir: false,
            listed: None,
        },
        // A DIRECTORY whose name passes the filename filter.
        Row {
            path: "flat/reports/backtest-dir.md",
            is_dir: true,
            listed: None,
        },
        // A FILE named `reports` is not a reports dir (falsifier O6).
        Row {
            path: "fslug/reports",
            is_dir: false,
            listed: None,
        },
    ];

    /// Materialise [`TREE`] under `root`.
    pub(crate) fn build(root: &Path) {
        for row in TREE {
            let p = root.join(row.path);
            if row.is_dir {
                std::fs::create_dir_all(&p)
                    .unwrap_or_else(|e| panic!("mkdir {}: {e}", p.display()));
            } else {
                let parent = p.parent().unwrap_or(root);
                std::fs::create_dir_all(parent)
                    .unwrap_or_else(|e| panic!("mkdir {}: {e}", parent.display()));
                std::fs::write(&p, "ts\n").unwrap_or_else(|e| panic!("write {}: {e}", p.display()));
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::test_tree::{TREE, build};
    use super::*;

    /// The distinct in-contract `reports/` dirs of the listed TREE rows, in
    /// (container depth, path) order — the oracle for `report_dirs`. The
    /// container depth is counted from the row's path (`<slug>/reports` = 0,
    /// `<container>/<slug>/reports` = 1), never asked of the subject.
    fn expected_dirs(root: &Path) -> Vec<ReportDir> {
        let mut out: Vec<ReportDir> = Vec::new();
        for row in TREE {
            if let Some((slug, _)) = row.listed {
                let rel_dir = Path::new(row.path).parent().unwrap();
                let container_depth = rel_dir.components().count() - 2;
                let rd = ReportDir {
                    slug: SmolStr::new(slug),
                    container_depth,
                    dir: root.join(rel_dir),
                };
                if !out.contains(&rd) {
                    out.push(rd);
                }
            }
        }
        out.sort_by(|a, b| (a.container_depth, &a.dir).cmp(&(b.container_depth, &b.dir)));
        out
    }

    /// LY1 — `report_dirs` lists exactly the in-contract dirs of the TREE, in
    /// (container depth, path) order (the D8 tie rule). The TREE makes that
    /// order differ from full-path order (`a0/sma` sorts before `flat`), so a
    /// full-path sort is red here.
    #[test]
    fn report_dirs_lists_exactly_the_in_contract_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        build(tmp.path());
        let expected = expected_dirs(tmp.path());
        assert!(
            expected.len() >= 2,
            "the TREE must declare ≥2 listed dirs, else LY1 pins nothing"
        );
        let mut by_path = expected.clone();
        by_path.sort_by(|a, b| a.dir.cmp(&b.dir));
        assert_ne!(
            by_path, expected,
            "the TREE must make (container depth, path) order differ from full-path order"
        );
        assert_eq!(
            report_dirs(tmp.path()),
            expected,
            "report_dirs must list exactly the in-contract reports/ dirs in (container depth, \
             path) order (#205)"
        );
    }

    /// LY2 — K2 two-sided: an absent root lists nothing, the fixture root
    /// lists something.
    #[test]
    fn report_dirs_absent_root_is_empty_fixture_root_is_not() {
        let tmp = tempfile::tempdir().unwrap();
        let absent = tmp.path().join("definitely-absent");
        assert!(!absent.exists());
        assert!(
            report_dirs(&absent).is_empty(),
            "an absent root must list no reports/ dir (K2 never-panic)"
        );
        build(tmp.path());
        assert!(
            !report_dirs(tmp.path()).is_empty(),
            "the fixture root must list ≥1 reports/ dir (the positive side of LY2)"
        );
    }

    /// LY3 — the Lab's targeted lookup equals the full walk filtered to the
    /// slug, ORDER included, and equals the TREE-derived dirs of that slug, for
    /// listed, out-of-contract, unknown and malformed slugs. The malformed ones
    /// include the spellings `Path::components` normalises to a listed slug
    /// (`"sma/"`, `"sma//"`, `"sma/."`), which the exact round-trip in
    /// `is_slug_component` rejects, and `"./sma"` (a leading `CurDir`). The
    /// slug `"reports"` meets a real `<root>/reports/reports` dir in the TREE.
    #[test]
    fn report_dirs_for_slug_agrees_with_report_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        build(tmp.path());
        let all = report_dirs(tmp.path());
        let tree = expected_dirs(tmp.path());
        for slug in [
            "flat",
            "sma",
            "too-deep",
            "inner",
            "fslug",
            "nope",
            "reports",
            "",
            "x/too-deep",
            "sma/",
            "sma//",
            "sma/.",
            "./sma",
            "/sma",
        ] {
            let want: Vec<PathBuf> = all
                .iter()
                .filter(|r| r.slug == slug)
                .map(|r| r.dir.clone())
                .collect();
            let from_tree: Vec<PathBuf> = tree
                .iter()
                .filter(|r| r.slug == slug)
                .map(|r| r.dir.clone())
                .collect();
            let got = report_dirs_for_slug(tmp.path(), slug);
            assert_eq!(
                got, want,
                "report_dirs_for_slug({slug:?}) must equal report_dirs filtered to it (#205)"
            );
            assert_eq!(
                got, from_tree,
                "report_dirs_for_slug({slug:?}) must equal the TREE's dirs of that slug, in \
                 (container depth, path) order (#205)"
            );
        }
        // Non-vacuity, from the TREE: `sma` sits at container depth 0 AND 1,
        // and one of its container-depth-1 dirs sorts before its
        // container-depth-0 dir by path — so the order check above can fail.
        let sma: Vec<&ReportDir> = tree.iter().filter(|r| r.slug == "sma").collect();
        assert!(sma.iter().any(|r| r.container_depth == 0));
        assert!(sma.iter().any(|r| r.container_depth == 1));
        let d0 = sma.iter().find(|r| r.container_depth == 0).unwrap();
        assert!(
            sma.iter().any(|r| r.container_depth == 1 && r.dir < d0.dir),
            "the TREE must hold a container-depth-1 `sma` dir that sorts before the \
             container-depth-0 one by path"
        );
        assert!(report_dirs_for_slug(tmp.path(), "nope").is_empty());
    }

    /// LY4 — `backtest_files` lists exactly the direct `backtest-*.md` files
    /// of every listed dir, sorted by path (the layout's "files of one dir by
    /// path"; falsifier Q3). The TREE holds a listed dir with two reports, so
    /// the order within one dir is observable.
    #[test]
    fn backtest_files_lists_only_direct_backtest_md_files() {
        let tmp = tempfile::tempdir().unwrap();
        build(tmp.path());
        let mut most = 0;
        for rd in expected_dirs(tmp.path()) {
            let mut want: Vec<PathBuf> = TREE
                .iter()
                .filter(|r| r.listed.is_some())
                .map(|r| tmp.path().join(r.path))
                .filter(|p| p.parent() == Some(rd.dir.as_path()))
                .collect();
            want.sort();
            most = most.max(want.len());
            assert_eq!(
                backtest_files(&rd.dir),
                want,
                "backtest_files({}) must list exactly the direct backtest-*.md files, by path",
                rd.dir.display()
            );
        }
        assert!(
            most >= 2,
            "the TREE must hold a listed dir with ≥2 reports, else LY4 cannot see the order"
        );
    }

    /// LY5 — symlinks are followed, and a symlink loop terminates at the depth
    /// bound (the module doc's claim). `<root>/c/s/reports` is real;
    /// `<root>/alias -> <root>/c` is a symlinked container; `<root>/link ->
    /// <root>` is a loop. Expected, derived from those three entries: the real
    /// dir and its symlinked alias, both at container depth 1, and nothing
    /// reached through `link/` (it only re-enters `<root>`, whose children hold
    /// no `reports/` at container depth 0).
    #[cfg(unix)]
    #[test]
    fn report_dirs_follows_symlinks_and_a_symlink_loop_terminates() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir_all(root.join("c/s/reports")).unwrap();
        std::fs::write(
            root.join("c/s/reports/backtest-20260101-000000-real.md"),
            "ts\n",
        )
        .unwrap();
        std::os::unix::fs::symlink(root.join("c"), root.join("alias")).unwrap();
        std::os::unix::fs::symlink(root, root.join("link")).unwrap();
        let expected = vec![
            ReportDir {
                slug: SmolStr::new("s"),
                container_depth: 1,
                dir: root.join("alias/s/reports"),
            },
            ReportDir {
                slug: SmolStr::new("s"),
                container_depth: 1,
                dir: root.join("c/s/reports"),
            },
        ];
        assert_eq!(
            report_dirs(root),
            expected,
            "a symlinked container is followed and a symlink loop stops at the depth bound"
        );
        assert_eq!(
            report_dirs_for_slug(root, "s"),
            vec![root.join("alias/s/reports"), root.join("c/s/reports")]
        );
    }
}
