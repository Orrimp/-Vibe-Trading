//! The embedded-font contract (ADR-0093): every glyph the cockpit draws comes from a
//! face this repository ships, on every platform.
//!
//! ## Why this file exists
//!
//! Until 2026-09-16 the cockpit named no font at all. `Font::DEFAULT` is the generic
//! `SansSerif`, which cosmic-text maps to "Open Sans" — a face neither macOS nor the
//! CI runners ship — so every glyph fell through to whatever the OS font database
//! offered. The 56 byte-exact visual baselines were therefore a property of the
//! machine that captured them: they went red across an OS update on the canonical box
//! (`docs/dev-notes/visual-baseline-drift-2026-07-27.md`) and could never be shared
//! across OSes at all.
//!
//! Naming the face fixes that only if three things hold, and each is a test here:
//!
//! 1. the face is really loaded and really named ([`the_embedded_face_is_loaded_and_named`]);
//! 2. every application actually selects it, including the ones the pixel gates render
//!    ([`every_application_selects_the_embedded_font`]) — `iced_test`'s `Emulator`
//!    takes the renderer's default font from `Program::settings()`, so a program built
//!    without `.default_font(..)` silently renders through the OS again;
//! 3. nothing draws a glyph the face lacks ([`every_glyph_the_ui_draws_is_in_the_embedded_face`]),
//!    because a missing glyph falls back to an OS font per-glyph and re-arms exactly
//!    the same drift for that character.
//!
//! Canvas text gets its own guard ([`every_canvas_text_names_the_embedded_font`]): a
//! `canvas::Text` carries a concrete `font` field and does NOT inherit the renderer
//! default, so it is the one place where "the application set it" is not enough.
//!
//! **Scope.** These tests read string literals under `crates/ui/src`. Text the cockpit
//! LOADS at runtime is outside them — report bodies from `evidence/`, rendered by the
//! Reports screen and the `viewer` binary, do contain glyphs the face lacks (block-element
//! sparkline rows above all). That gap is measured in bug-log #100; do not read these tests
//! as proving more than they scan.
//!
//! These run on every platform on purpose — the point of the contract is that the
//! answer does not depend on which one.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::advanced::graphics::text::font_system;

/// The family name the cockpit's font constant asks for.
fn ui_family() -> &'static str {
    match ui::theme::font::UI.family {
        iced::font::Family::Name(name) => name,
        other => panic!("the UI font must name a family, not a generic one: {other:?}"),
    }
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `.rs` file under `root`, sorted, so failures are reported in a stable order.
fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// A tiny Rust scanner: enough to tell code from comments, strings and char literals.
///
/// Deliberately not a regex — a doc comment mentioning `"✓"` is prose, not a glyph the
/// UI draws, and `'"'` must not open a string. Decodes `\u{..}` escapes, because most
/// of the symbols in `strings.rs` are written that way.
fn string_literals(src: &str) -> Vec<(usize, String)> {
    let b: Vec<char> = src.chars().collect();
    let mut out: Vec<(usize, String)> = Vec::new();
    let (mut i, mut line) = (0usize, 1usize);
    while i < b.len() {
        match b[i] {
            '\n' => {
                line += 1;
                i += 1;
            }
            '/' if b.get(i + 1) == Some(&'/') => {
                while i < b.len() && b[i] != '\n' {
                    i += 1;
                }
            }
            '/' if b.get(i + 1) == Some(&'*') => {
                let mut depth = 1;
                i += 2;
                while i < b.len() && depth > 0 {
                    match (b[i], b.get(i + 1)) {
                        ('/', Some('*')) => {
                            depth += 1;
                            i += 2;
                        }
                        ('*', Some('/')) => {
                            depth -= 1;
                            i += 2;
                        }
                        ('\n', _) => {
                            line += 1;
                            i += 1;
                        }
                        _ => i += 1,
                    }
                }
            }
            '\'' => {
                // A char literal ('x', '\n', '\u{2713}') or a lifetime ('a).
                let mut j = i + 1;
                if b.get(j) == Some(&'\\') {
                    j += 1;
                    if b.get(j) == Some(&'u') {
                        while j < b.len() && b[j] != '}' {
                            j += 1;
                        }
                    }
                    j += 1;
                } else if j < b.len() {
                    j += 1;
                }
                i = if b.get(j) == Some(&'\'') {
                    j + 1
                } else {
                    i + 1
                };
            }
            'r' if matches!(b.get(i + 1), Some('"') | Some('#')) => {
                let mut j = i + 1;
                let mut hashes = 0;
                while b.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                if b.get(j) != Some(&'"') {
                    i += 1;
                    continue;
                }
                let start = line;
                let mut s = String::new();
                j += 1;
                while j < b.len() {
                    if b[j] == '"' && (1..=hashes).all(|k| b.get(j + k) == Some(&'#')) {
                        j += hashes + 1;
                        break;
                    }
                    if b[j] == '\n' {
                        line += 1;
                    }
                    s.push(b[j]);
                    j += 1;
                }
                out.push((start, s));
                i = j;
            }
            '"' => {
                let start = line;
                let mut s = String::new();
                let mut j = i + 1;
                while j < b.len() {
                    match b[j] {
                        '"' => {
                            j += 1;
                            break;
                        }
                        '\\' if b.get(j + 1) == Some(&'u') && b.get(j + 2) == Some(&'{') => {
                            let mut k = j + 3;
                            let mut hex = String::new();
                            while let Some(&c) = b.get(k) {
                                if c == '}' {
                                    break;
                                }
                                hex.push(c);
                                k += 1;
                            }
                            if let Some(c) =
                                u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
                            {
                                s.push(c);
                            }
                            j = k + 1;
                        }
                        '\\' => {
                            if b.get(j + 1) == Some(&'\n') {
                                line += 1;
                            }
                            j += 2;
                        }
                        '\n' => {
                            line += 1;
                            s.push('\n');
                            j += 1;
                        }
                        c => {
                            s.push(c);
                            j += 1;
                        }
                    }
                }
                out.push((start, s));
                i = j;
            }
            _ => i += 1,
        }
    }
    out
}

/// The text of each builder chain starting at `needle`, with its 1-based line.
///
/// Walks from the call, tracking bracket depth and skipping strings and comments, and
/// stops where the statement does: a `;` or a block `{` at depth 0, or the `}` that
/// closes the enclosing block. That keeps multi-line chains — including
/// `cockpit_live`'s 140-line boot closure — in one piece.
fn builder_chains(src: &str, needle: &str) -> Vec<(usize, String)> {
    let b: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    // Match only real code: a doc comment that CITES `iced::application(..)` (there are
    // two, one of them in `theme::font::embedded`'s own docs) is prose, not a program.
    let masked = mask_comments(src);
    for (byte_at, _) in masked.match_indices(needle) {
        let start = src[..byte_at].chars().count();
        let line = src[..byte_at].matches('\n').count() + 1;
        let mut depth: i32 = 0;
        let mut i = start;
        while i < b.len() {
            match b[i] {
                '/' if b.get(i + 1) == Some(&'/') => {
                    while i < b.len() && b[i] != '\n' {
                        i += 1;
                    }
                    continue;
                }
                '"' => {
                    i += 1;
                    while i < b.len() && b[i] != '"' {
                        i += if b[i] == '\\' { 2 } else { 1 };
                    }
                }
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                '{' if depth == 0 => break,
                '{' => depth += 1,
                '}' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                ';' if depth == 0 => break,
                _ => {}
            }
            if depth < 0 {
                break;
            }
            i += 1;
        }
        out.push((line, b[start..i.min(b.len())].iter().collect()));
    }
    out
}

/// Blank out `//` and `/* */` comments, preserving every byte offset and newline, so a
/// pattern found in the result is code and its line number still matches the file.
fn mask_comments(src: &str) -> String {
    /// Replace one char with as many spaces as it occupies in UTF-8, so every byte
    /// offset in the masked copy still names the same byte of the original.
    fn blank(out: &mut Vec<char>, c: char) {
        for _ in 0..c.len_utf8() {
            out.push(' ');
        }
    }

    let b: Vec<char> = src.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(b.len());
    let mut i = 0usize;
    while i < b.len() {
        match (b[i], b.get(i + 1)) {
            ('/', Some('/')) => {
                while i < b.len() && b[i] != '\n' {
                    blank(&mut out, b[i]);
                    i += 1;
                }
            }
            ('/', Some('*')) => {
                let mut depth = 1;
                out.push(' ');
                out.push(' ');
                i += 2;
                while i < b.len() && depth > 0 {
                    match (b[i], b.get(i + 1)) {
                        ('/', Some('*')) => {
                            depth += 1;
                            out.extend([' ', ' ']);
                            i += 2;
                        }
                        ('*', Some('/')) => {
                            depth -= 1;
                            out.extend([' ', ' ']);
                            i += 2;
                        }
                        (c, _) => {
                            if c == '\n' {
                                out.push('\n');
                            } else {
                                blank(&mut out, c);
                            }
                            i += 1;
                        }
                    }
                }
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    out.into_iter().collect()
}

/// Every `{ .. }` literal opened by `needle`, with its 1-based line.
fn brace_literals(src: &str, needle: &str) -> Vec<(usize, String)> {
    let b: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let masked = mask_comments(src);
    for (byte_at, _) in masked.match_indices(needle) {
        let start = src[..byte_at].chars().count();
        let line = src[..byte_at].matches('\n').count() + 1;
        let mut i = start + needle.chars().count();
        let mut depth = 1;
        while i < b.len() && depth > 0 {
            match b[i] {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        out.push((line, b[start..i.min(b.len())].iter().collect()));
    }
    out
}

/// The face is in the font database, under the name the cockpit asks for.
///
/// Fails on a platform where the embedded bytes do not parse or the family name in the
/// file ever changes — which would otherwise show up as silent OS-font rendering.
#[test]
fn the_embedded_face_is_loaded_and_named() {
    let font = ui::theme::font::embedded();
    assert_eq!(font, ui::theme::font::UI);

    let family = ui_family();
    let system = font_system();
    let mut guard = system.write().expect("font system lock");
    let names: Vec<String> = guard
        .raw()
        .db()
        .faces()
        .filter(|face| face.families.iter().any(|(name, _)| name == family))
        .map(|face| face.post_script_name.clone())
        .collect();
    assert!(
        !names.is_empty(),
        "`{family}` is not in the font database after `theme::font::embedded()`: the \
         cockpit would render through the OS font database instead"
    );
}

/// The renderer default really reaches the widgets: text drawn by a program whose
/// `default_font` is the embedded face is pixel-identical to the same text that names
/// the face itself.
///
/// The second half is the control: the comparison can tell frames apart at all.
#[test]
fn the_default_font_is_the_embedded_face_at_the_pixels() {
    const SAMPLE: &str = "Honest Advisor 0123456789 ✓ ✗ ⚠ ★ ●";

    let inherited = render_sample(SAMPLE, false, Some(ui::theme::font::embedded()));
    let named = render_sample(SAMPLE, true, None);
    assert!(
        inherited == named,
        "text that INHERITS the program's default font must rasterise exactly like text \
         that names the embedded face; it does not, so `.default_font(..)` is not \
         reaching the widgets"
    );

    let different = render_sample(
        "Honest Advisor 0123456788 ✓ ✗ ⚠ ★ ●",
        false,
        Some(ui::theme::font::embedded()),
    );
    assert!(
        inherited != different,
        "the comparison above is vacuous: two different strings rasterise identically"
    );
}

/// Nothing the cockpit can draw needs a glyph the embedded face lacks.
///
/// cosmic-text falls back PER GLYPH: one missing character is enough to put an OS font
/// back into an otherwise repo-deterministic frame.
#[test]
fn every_glyph_the_ui_draws_is_in_the_embedded_face() {
    let family = ui_family();
    // Read the character map out of the embedded bytes. cosmic-text's
    // `Font::unicode_codepoints()` is a lazily-filled cache of what has been shaped so
    // far, not the font's cmap — asked cold it called `—` and `→` absent from a face
    // that carries both.
    let face = ttf_parser::Face::parse(ui::theme::font::UI_REGULAR, 0)
        .expect("the embedded face parses as a TTF");
    let mut covered: HashSet<u32> = HashSet::new();
    for subtable in face
        .tables()
        .cmap
        .expect("the embedded face has a cmap")
        .subtables
    {
        if subtable.is_unicode() {
            subtable.codepoints(|cp| {
                if subtable.glyph_index(cp).is_some() {
                    covered.insert(cp);
                }
            });
        }
    }
    assert!(
        covered.len() > 1000,
        "only {} codepoints read out of `{family}` — the cmap parse is wrong, not the UI",
        covered.len()
    );

    let src = crate_dir().join("src");
    let mut offenders = Vec::new();
    for file in rust_sources(&src) {
        let text = std::fs::read_to_string(&file).expect("read source");
        for (line, literal) in string_literals(&text) {
            for ch in literal.chars().filter(|c| !c.is_ascii()) {
                if !covered.contains(&(ch as u32)) {
                    offenders.push(format!(
                        "{}:{line}  U+{:04X} {ch}",
                        file.strip_prefix(crate_dir()).unwrap_or(&file).display(),
                        ch as u32
                    ));
                }
            }
        }
    }
    offenders.dedup();
    assert!(
        offenders.is_empty(),
        "these glyphs are not in `{family}`, so they would be drawn by whatever OS font \
         has them — pick a glyph the embedded face carries, or embed a face that has \
         this one:\n  {}",
        offenders.join("\n  ")
    );
}

/// Every `iced::application(..)` loads AND names the embedded face.
///
/// `iced_test`'s `Emulator` builds its renderer from `Program::settings().default_font`
/// and never loads `settings.fonts`, so a program that forgets either half renders
/// through the OS font database — and a baseline captured from it drifts with the OS.
#[test]
fn every_application_selects_the_embedded_font() {
    let mut offenders = Vec::new();
    for dir in ["src", "tests"] {
        for file in rust_sources(&crate_dir().join(dir)) {
            // This file renders a control frame WITHOUT the default font on purpose.
            if file
                .file_name()
                .is_some_and(|n| n == "embedded_font_contract.rs")
            {
                continue;
            }
            let text = std::fs::read_to_string(&file).expect("read source");
            for (line, chain) in builder_chains(&text, "iced::application(") {
                if !chain.contains("font::embedded()") {
                    offenders.push(format!(
                        "{}:{line}",
                        file.strip_prefix(crate_dir()).unwrap_or(&file).display()
                    ));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these applications do not `.default_font(theme::font::embedded())`, so their \
         text comes from the OS font database:\n  {}",
        offenders.join("\n  ")
    );
}

/// Every canvas `Text` names the embedded face.
///
/// A `canvas::Text` carries a concrete `font` field defaulting to `Font::DEFAULT`; it
/// does not inherit the renderer default, so chart axis labels, legends and tooltips
/// would keep rendering through the OS font database even after every application sets
/// its default.
#[test]
fn every_canvas_text_names_the_embedded_font() {
    let mut offenders = Vec::new();
    for file in rust_sources(&crate_dir().join("src")) {
        let text = std::fs::read_to_string(&file).expect("read source");
        for (line, literal) in brace_literals(&text, "CanvasText {") {
            if !literal.contains("font:") {
                offenders.push(format!(
                    "{}:{line}",
                    file.strip_prefix(crate_dir()).unwrap_or(&file).display()
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "these canvas texts do not set `font: theme::font::UI`, so they draw through the \
         OS font database:\n  {}",
        offenders.join("\n  ")
    );
}

// ── the sample program the pixel guard renders ────────────────────────────────

#[derive(Debug, Clone)]
enum Message {}

#[derive(Default)]
struct Sample {
    text: String,
    names_the_font: bool,
}

impl Sample {
    fn update(&mut self, _message: Message) -> iced::Task<Message> {
        iced::Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let label = iced::widget::text(self.text.clone()).size(18);
        let label = if self.names_the_font {
            label.font(ui::theme::font::UI)
        } else {
            label
        };
        iced::widget::container(label).padding(12).into()
    }
}

/// Render `text` at a fixed size; optionally naming the font on the widget, and
/// optionally setting the program's default font.
fn render_sample(text: &str, names_the_font: bool, default_font: Option<iced::Font>) -> Vec<u8> {
    let owned = text.to_owned();
    let boot = move || {
        (
            Sample {
                text: owned.clone(),
                names_the_font,
            },
            iced::Task::none(),
        )
    };
    let program = iced::application(boot, Sample::update, Sample::view);
    let program = match default_font {
        Some(font) => program.default_font(font),
        None => program,
    };
    iced_test::screenshot(&program, &iced::Theme::Dark, (640, 80), 1.0, Duration::ZERO)
        .rgba
        .to_vec()
}
