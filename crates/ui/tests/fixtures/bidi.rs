//! Shared bidi helpers for the ui test binaries that shape text holding a bidi
//! paragraph separator (bug-log `#231`, `#239`; ADR-0106).
//!
//! Included through `#[path = "fixtures/bidi.rs"] mod bidi;` by
//! `layout_invariants.rs` (its "Free text the fuzzers draw" section) and by
//! `bidi_paragraph_separators.rs`. It holds exactly one of each helper both
//! files need, so the two cannot drift apart: the class-B scan, the private
//! shaping thread's name, and the private shaper.
//!
//! Nothing here touches iced's process-wide font system: a shaping panic (the
//! `#239` assert, or a regression of the ADR-0106 hunk) unwinds on a private
//! thread holding a private `FontSystem`, and comes back as an `Err`.

use std::sync::OnceLock;

use iced::advanced::graphics::text::cosmic_text as ct;

/// Every private shaping runs on a thread of this name, so a panic line naming
/// it is one of these helpers' own shapings, never iced's lock (bug-log `#231`).
#[allow(dead_code)] // each including test file uses a different subset
pub const PRIVATE_SHAPING_THREAD: &str = "bidi-private-shaping";

/// Every `char` of Bidi_Class B, scanned once per binary: `c` is class B iff
/// `"\u{5d0}{c}\u{5d0}"` splits into two bidi paragraphs. The probe is non-ASCII,
/// so `BidiParagraphs` takes its `BidiInfo` path (`bidi_para.rs`): the same
/// unicode-bidi build cosmic-text shapes with, so the scan cannot skew from the
/// shaper. `#231`'s T1 pins it in `layout_invariants`; `bidi_paragraph_separators`
/// B1 cross-checks it against an independent `unicode_bidi::bidi_class` scan.
#[allow(dead_code)] // each including test file uses a different subset
pub fn class_b_chars() -> &'static [char] {
    static SCAN: OnceLock<Vec<char>> = OnceLock::new();
    SCAN.get_or_init(|| {
        let mut probe = String::with_capacity(12);
        (0..=u32::from(char::MAX))
            .filter_map(char::from_u32)
            .filter(|&c| {
                probe.clear();
                probe.push('\u{5d0}');
                probe.push(c);
                probe.push('\u{5d0}');
                ct::BidiParagraphs::new(&probe).count() == 2
            })
            .collect()
    })
}

/// The UI face's family name, derived from the theme (as `embedded_font_contract.rs` does).
fn ui_family() -> &'static str {
    match ui::theme::font::UI.family {
        iced::font::Family::Name(name) => name,
        other => panic!("the UI font must name a family, not a generic one: {other:?}"),
    }
}

/// Shape `text` the way iced does, on a PRIVATE font system holding only the
/// embedded UI face, on its own thread named [`PRIVATE_SHAPING_THREAD`], so a
/// cosmic-text panic here can never poison iced's process-wide lock.
///
/// - `rich = false`: `Buffer::set_text`, the path of iced's `Paragraph::with_text`
///   (iced_graphics-0.14.0/src/text/paragraph.rs:87-93), which cuts lines at
///   `\r`/`\n` only, so a class-B separator stays inside one line.
/// - `rich = true`: `Buffer::set_rich_text` with one span, the path of iced's
///   `with_spans`, which cuts lines at every class-B char (`BidiParagraphs`).
///
/// `Ok` holds each `LayoutRun`'s `rtl` (`buffer.rs:151`), in order; `Err` carries
/// the panic message.
#[allow(dead_code)] // each including test file uses a different subset
pub fn shape_on_a_private_font_system(text: &str, rich: bool) -> Result<Vec<bool>, String> {
    static UI_FACE: OnceLock<ct::fontdb::Database> = OnceLock::new();
    let db = UI_FACE
        .get_or_init(|| {
            let mut db = ct::fontdb::Database::new();
            db.load_font_data(ui::theme::font::UI_REGULAR.to_vec());
            db
        })
        .clone();
    let text = text.to_owned();
    std::thread::Builder::new()
        .name(PRIVATE_SHAPING_THREAD.to_owned())
        .spawn(move || {
            let mut fonts = ct::FontSystem::new_with_locale_and_db(String::from("en-US"), db);
            let attrs = ct::Attrs::new().family(ct::Family::Name(ui_family()));
            let mut buffer = ct::Buffer::new(&mut fonts, ct::Metrics::new(14.0, 20.0));
            if rich {
                buffer.set_rich_text(
                    &mut fonts,
                    [(text.as_str(), attrs.clone())],
                    &attrs,
                    ct::Shaping::Advanced,
                    None,
                );
            } else {
                buffer.set_text(&mut fonts, &text, &attrs, ct::Shaping::Advanced, None);
            }
            buffer
                .layout_runs()
                .map(|run| run.rtl)
                .collect::<Vec<bool>>()
        })
        .expect("spawn the private-shaping thread")
        .join()
        .map_err(|p| {
            p.downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| String::from("<non-string panic>"))
        })
}
