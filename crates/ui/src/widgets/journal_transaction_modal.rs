//! Journal-transaction audit modal — the cockpit's first true modal.
//!
//! Click-through-to-audit destination for the live tape (tape-row-audit-modal).
//! Renders the full `journal_transactions` row + its `journal_entries` rows
//! in a 4-column `Account | Debit | Credit | Currency` table on top of a
//! dimmed cockpit body. Read-only: no writes from this surface
//! ([principles "Show the why"](../../../../spec/ui-design-principles.md#show-the-why)).
//!
//! ## Iced precedent
//!
//! This widget is the **first overlay use of `iced::widget::float::Float`**
//! (Brief A R4, M4 — iced-native-widgets v0.1.0) on top of a
//! `iced::widget::Stack`. The Stack layers in z-order are: (1) cockpit
//! main column (`content` arg) — bottom; (2) translucent backdrop
//! (`MouseArea<Container<Space>>`, captures click-outside dismiss) —
//! middle; (3) centered modal card (Container with `bg_overlay` +
//! `border_strong`) — top. The whole Stack is wrapped in `Float::new(...)` —
//! the iced 0.14 positioning primitive — so future overlays inherit the
//! upstream-blessed `Float` route rather than a hand-rolled overlay column.
//!
//! `Float` is **positioning-only** in iced 0.14 (architect H-arch-A7 /
//! H-arch-A7b RESOLVED-FALSIFIED 2026-05-13): zero `on_dismiss`,
//! `on_close`, `backdrop`, or keyboard hooks. Consequences: click-outside
//! dismiss stays via the existing hand-rolled
//! `MouseArea::new(Space).on_press(close_msg)` backdrop sibling;
//! Escape-to-dismiss stays in the cockpit's keyboard subscription
//! (`bin/cockpit.rs::subscription` at line 251 — modal-open-gated
//! `iced::event::listen_with` recipe routes `Esc` →
//! `Message::TapeAuditModalClosed`); and card centering stays via the
//! inner Container's `center_x` / `center_y` chrome — `Float`'s
//! `translate` closure is reserved for future use; default scale and
//! translate keep `Float` structurally inert at runtime while the
//! migration carries the architectural commitment (R4 in
//! iced-native-widgets v0.1.0).
//!
//! ## Principles compliance
//!
//! - **Show the why** — modal exists to surface the journal transaction
//!   behind every fill row.
//! - **No blank screens** — `Loading` / `Empty` / `Error` / `Ready`
//!   covered explicitly via `PanelState<JournalTransactionView>`.
//! - **Plain language** — column labels are `Account` / `Debit` /
//!   `Credit` / `Currency`, not the underlying SQL column names.
//! - **Numbers are scannable** — debit/credit cells right-aligned with
//!   monospace digits via `widgets::num::fmt_usdt`.
//! - **Iconography: no icons until needed** — close affordance is the
//!   text label `"Close"`, not an `×` glyph.
//! - **Density** — compact (cockpit) per the principles density table:
//!   24-px row, 12-px cell pad, 24-px modal inner pad.
//!
//! ## State integration
//!
//! The widget reads `JournalModalState` and `JournalTransactionView`,
//! both of which land in `crate::state` as part of T1206 (modal-open
//! state machine + click handler + keyboard subscription). Until then,
//! local placeholder structs in this module satisfy the compile while
//! the T1206 wiring is in flight; T1206 swaps the placeholders for
//! `pub use` re-exports from `crate::state`. The placeholders match
//! the architect's design `Modal state shape` shape exactly so the
//! swap is mechanical.

use iced::widget::{
    button, container, float, Button, Column, Container, Float, MouseArea, Row, Space, Stack, Text,
};
use iced::{Element, Length};

use trading_core::JournalEntry;

use crate::state::{JournalModalState, JournalTransactionView, PanelState};
use crate::strings::{
    TAPE_AUDIT_MODAL_CLOSE_LABEL, TAPE_AUDIT_MODAL_COL_ACCOUNT, TAPE_AUDIT_MODAL_COL_CREDIT,
    TAPE_AUDIT_MODAL_COL_CURRENCY, TAPE_AUDIT_MODAL_COL_DEBIT, TAPE_AUDIT_MODAL_DESC_LABEL,
    TAPE_AUDIT_MODAL_EMPTY, TAPE_AUDIT_MODAL_ERROR_PREFIX, TAPE_AUDIT_MODAL_LOADING,
    TAPE_AUDIT_MODAL_STRATEGY_LABEL, TAPE_AUDIT_MODAL_STRATEGY_NONE, TAPE_AUDIT_MODAL_TITLE,
    TAPE_AUDIT_MODAL_TS_LABEL, TAPE_AUDIT_MODAL_TX_LABEL,
};
use crate::theme::{color, radius, space, text, ThemeMode};

use super::num::fmt_usdt;

// ── State types — re-exported from `crate::state` (T1206 wiring) ────────────
//
// `JournalModalState` and `JournalTransactionView` live in `crate::state`
// because they are part of the `Cockpit` model. T1205 originally shipped
// placeholder structs here so the widget could be sketched against the
// architect's `Modal state shape`; T1206 swapped those placeholders for
// the `use` imports above. The shapes match the architect's design
// exactly so the swap was mechanical.

// ── Public API ──────────────────────────────────────────────────────────────

/// Modal width — `~480 px` per
/// [tape-row-audit-modal R10](../../../../spec/features/tape-row-audit-modal.md#r10).
/// Pinned here as a layout constant rather than a theme token since the
/// modal width is a feature-specific contract, not a reusable design token.
const MODAL_WIDTH_PX: u32 = 480;

/// Render the modal as a `Float`-wrapped `Stack` overlay on top of `content`.
///
/// `content` is the cockpit's existing main column — passed in as the
/// bottom layer of the inner `Stack`. The middle layer is the
/// translucent backdrop (`MouseArea<Container<Space>>`, captures
/// click-outside dismiss). The top layer is the modal card on a
/// `bg_overlay` backdrop. The whole 3-layer Stack is wrapped in
/// `iced::widget::float::Float` — R4 migration to the iced 0.14
/// positioning primitive (Brief A, iced-native-widgets v0.1.0).
///
/// `close_msg` is the message emitted when the operator clicks the
/// backdrop or the explicit `Close` button; T1206 supplies the concrete
/// `Message::TapeAuditModalClosed` variant. Keeping the close message
/// generic lets this widget compile across the cockpit and cockpit_live
/// callers. The third close path — `Esc` — is **not** wired in this
/// widget: `Float` has zero keyboard participation (H-arch-A7b
/// FALSIFIED, 2026-05-13). Escape stays in `bin/cockpit.rs::subscription`
/// + `bin/cockpit_live.rs::subscription` (modal-open-gated keyboard
/// recipe routing `Esc` to `Message::TapeAuditModalClosed`).
///
/// Returns a `Float<Stack>` with the cockpit body underneath the modal —
/// when the caller's `Cockpit.tape_audit_modal == None`, callers render
/// `content` directly without invoking this function (so the cockpit's
/// rendered iced tree is byte-identical to the pre-modal world; existing
/// snapshot tests stay green by construction).
#[must_use]
pub fn view<'a, Msg>(
    state: &'a JournalModalState,
    content: Element<'a, Msg>,
    close_msg: Msg,
) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let backdrop = backdrop_layer(close_msg.clone());
    let card = modal_card(state, close_msg);

    // The 3-layer Stack composes the cockpit content, click-outside
    // dismiss backdrop, and centered modal card. The hand-rolled
    // `MouseArea` backdrop is preserved as a sibling layer
    // (H-arch-A7 RESOLVED-FALSIFIED — `Float` provides positioning
    // only; click-outside dispatch must stay in the existing
    // `MouseArea::new(Space).on_press(close_msg)` route).
    let base = Stack::new().push(content).push(backdrop).push(card);

    // Wrap in `Float` — the iced 0.14 positioning primitive. At default
    // scale (1.0) and no `translate` closure, `Float` is structurally
    // inert (`is_floating == false`), so the runtime tree is identical
    // to the pre-migration `Stack`. The migration carries the
    // architectural commitment for R4 in iced-native-widgets v0.1.0:
    // future overlays inherit the upstream-blessed `Float` route. The
    // `style` closure is the default no-shadow Style — Q3-sub PASS
    // (closure-style, no Catalog adapter needed for `Float`).
    Float::new(base)
        .style(|_theme: &iced::Theme| float::Style::default())
        .into()
}

// ── Layers ──────────────────────────────────────────────────────────────────

/// Full-bleed translucent backdrop. Captures clicks outside the modal card
/// and dispatches them as `close_msg` (R4 — three close affordances:
/// `Esc`, click-outside, explicit Close button).
fn backdrop_layer<'a, Msg>(close_msg: Msg) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let backdrop = Container::new(Space::new().width(Length::Fill).height(Length::Fill))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(color::OVERLAY.current(ThemeMode::Dark).into()),
            ..Default::default()
        });

    MouseArea::new(backdrop).on_press(close_msg).into()
}

/// Centered modal card on top of the backdrop. Pinned width per R10.
fn modal_card<'a, Msg>(state: &'a JournalModalState, close_msg: Msg) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let header = modal_header(close_msg);
    let metadata = metadata_block(state);
    let body = body_for(&state.entries);

    let column = Column::new()
        .spacing(space::M)
        .push(header)
        .push(metadata)
        .push(Space::new().height(space::M))
        .push(body);

    // `Padding: From<u16>` in iced 0.14; our scale maxes at 32 so the
    // cast is always lossless. Pedantic clippy wants the explicit allow.
    #[allow(clippy::cast_possible_truncation)]
    let card_padding = space::XL as u16;

    let card = Container::new(column)
        .padding(card_padding)
        .width(Length::Fixed(f32::from(
            u16::try_from(MODAL_WIDTH_PX).unwrap_or(u16::MAX),
        )))
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(color::PANEL_RAISED.current(ThemeMode::Dark).into()),
            border: iced::Border {
                color: color::BORDER_STRONG.current(ThemeMode::Dark),
                width: 1.0,
                radius: radius::R4.into(),
            },
            text_color: Some(color::FG_1.current(ThemeMode::Dark)),
            ..Default::default()
        });

    Container::new(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

// ── Modal sub-blocks ────────────────────────────────────────────────────────

fn modal_header<'a, Msg>(close_msg: Msg) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let title = Text::new(TAPE_AUDIT_MODAL_TITLE)
        .size(text::H2)
        .color(color::FG_1.current(ThemeMode::Dark));

    // T1504 — modal close button gains focus ring.
    // NOTE: iced 0.14 button::Status has no Focused variant (Active /
    // Hovered / Pressed / Disabled only). focus::ring is wired on Hovered
    // as a best-effort visual indicator; true keyboard focus ring is deferred
    // until iced exposes a Focused variant.
    let close_button = Button::new(
        Text::new(TAPE_AUDIT_MODAL_CLOSE_LABEL)
            .size(text::BODY)
            .color(color::FG_1.current(ThemeMode::Dark)),
    )
    .on_press(close_msg)
    .style(|_theme: &iced::Theme, status| {
        let (bg, ring) = match status {
            button::Status::Hovered => (
                color::BORDER_STRONG.current(ThemeMode::Dark),
                crate::theme::focus::ring(ThemeMode::Dark),
            ),
            button::Status::Pressed => (
                color::BORDER_STRONG.current(ThemeMode::Dark),
                iced::Shadow::default(),
            ),
            _ => (
                color::BORDER_1.current(ThemeMode::Dark),
                iced::Shadow::default(),
            ),
        };
        button::Style {
            background: Some(bg.into()),
            text_color: color::FG_1.current(ThemeMode::Dark),
            border: iced::Border {
                color: color::BORDER_STRONG.current(ThemeMode::Dark),
                width: 1.0,
                radius: radius::R2.into(),
            },
            shadow: ring,
            ..Default::default()
        }
    });

    Row::new()
        .push(title)
        .push(Space::new().width(Length::Fill))
        .push(close_button)
        .align_y(iced::Alignment::Center)
        .into()
}

fn metadata_block<'a, Msg>(state: &'a JournalModalState) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    // Default to the modal's own `tx_id` until the loaded view replaces it.
    let (tx_id, ts_str, description, strategy) = match &state.entries {
        PanelState::Ready(view) => (
            view.tx_id.as_str(),
            format_timestamp(view.ts),
            view.description.as_str(),
            view.strategy_id
                .as_ref()
                .map_or(TAPE_AUDIT_MODAL_STRATEGY_NONE.to_string(), |id| {
                    id.0.as_str().to_string()
                }),
        ),
        _ => (
            state.tx_id.as_str(),
            String::new(),
            "",
            TAPE_AUDIT_MODAL_STRATEGY_NONE.to_string(),
        ),
    };

    let mut col = Column::new().spacing(space::XS);
    col = col.push(metadata_row(TAPE_AUDIT_MODAL_TX_LABEL, tx_id_value(tx_id)));
    if !ts_str.is_empty() {
        col = col.push(metadata_row(TAPE_AUDIT_MODAL_TS_LABEL, mono_value(ts_str)));
    }
    if !description.is_empty() {
        col = col.push(metadata_row(
            TAPE_AUDIT_MODAL_DESC_LABEL,
            body_value(description.to_string()),
        ));
    }
    col = col.push(metadata_row(
        TAPE_AUDIT_MODAL_STRATEGY_LABEL,
        body_value(strategy),
    ));
    col.into()
}

fn metadata_row<'a, Msg>(label: &'a str, value: Element<'a, Msg>) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Row::new()
        .spacing(space::M)
        .push(
            Text::new(label)
                .size(text::MICRO)
                .color(color::FG_3.current(ThemeMode::Dark))
                .width(Length::Fixed(f32::from(112u16))),
        )
        .push(value)
        .align_y(iced::Alignment::Center)
        .into()
}

/// Transaction-id value rendering — `info` accent (informational, not
/// interactive) per
/// [tape-row-audit-modal R6](../../../../spec/features/tape-row-audit-modal.md#r6).
fn tx_id_value<'a, Msg>(value: &'a str) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(value)
        .size(text::BODY)
        .color(color::INFO_500.current(ThemeMode::Dark))
        .into()
}

fn mono_value<'a, Msg>(value: String) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(value)
        .size(text::BODY)
        .color(color::FG_1.current(ThemeMode::Dark))
        .into()
}

fn body_value<'a, Msg>(value: String) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(value)
        .size(text::BODY)
        .color(color::FG_1.current(ThemeMode::Dark))
        .into()
}

fn body_for<'a, Msg>(entries: &'a PanelState<JournalTransactionView>) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    match entries {
        PanelState::Loading => centered_message(
            TAPE_AUDIT_MODAL_LOADING,
            color::FG_3.current(ThemeMode::Dark),
        ),
        PanelState::Empty => {
            centered_message(TAPE_AUDIT_MODAL_EMPTY, color::FG_3.current(ThemeMode::Dark))
        }
        PanelState::Error(msg) => {
            // `<prefix><detail>` — matches the existing `*_ERROR_PREFIX`
            // pattern for tape / positions / P&L panels (principles
            // "Voice and copy" — "what's broken: what to check").
            let line = format!("{TAPE_AUDIT_MODAL_ERROR_PREFIX}{msg}");
            centered_message_owned(line, color::DOWN_500.current(ThemeMode::Dark))
        }
        PanelState::Ready(view) => entries_table(&view.entries),
    }
}

fn centered_message<'a, Msg>(text_value: &'a str, c: iced::Color) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let label = Text::new(text_value).size(text::BODY).color(c);
    Container::new(label)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

fn centered_message_owned<'a, Msg>(text_value: String, c: iced::Color) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let label = Text::new(text_value).size(text::BODY).color(c);
    Container::new(label)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

fn entries_table<'a, Msg>(entries: &'a [JournalEntry]) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    let header = Row::new()
        .spacing(space::M)
        .push(col_label_left(TAPE_AUDIT_MODAL_COL_ACCOUNT))
        .push(col_label_right(TAPE_AUDIT_MODAL_COL_DEBIT))
        .push(col_label_right(TAPE_AUDIT_MODAL_COL_CREDIT))
        .push(col_label_center(TAPE_AUDIT_MODAL_COL_CURRENCY));

    let mut rows = Column::new().spacing(space::XS);
    for entry in entries {
        rows = rows.push(entry_row(entry));
    }

    Column::new()
        .spacing(space::S)
        .push(header)
        .push(rows)
        .into()
}

fn entry_row<'a, Msg>(entry: &'a JournalEntry) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Row::new()
        .spacing(space::M)
        .push(account_cell(entry.account.0.as_str()))
        .push(amount_cell(fmt_usdt(entry.debit.amount())))
        .push(amount_cell(fmt_usdt(entry.credit.amount())))
        .push(currency_cell(entry.currency.as_str()))
        .into()
}

fn col_label_left<'a, Msg>(label: &'a str) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(label)
        .size(text::MICRO)
        .color(color::FG_3.current(ThemeMode::Dark))
        .width(Length::FillPortion(3))
        .into()
}

fn col_label_right<'a, Msg>(label: &'a str) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(label)
        .size(text::MICRO)
        .color(color::FG_3.current(ThemeMode::Dark))
        .width(Length::FillPortion(3))
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

fn col_label_center<'a, Msg>(label: &'a str) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(label)
        .size(text::MICRO)
        .color(color::FG_3.current(ThemeMode::Dark))
        .width(Length::FillPortion(2))
        .align_x(iced::alignment::Horizontal::Center)
        .into()
}

fn account_cell<'a, Msg>(value: &'a str) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(value)
        .size(text::BODY)
        .color(color::FG_1.current(ThemeMode::Dark))
        .width(Length::FillPortion(3))
        .into()
}

fn amount_cell<'a, Msg>(value: String) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(value)
        .size(text::BODY)
        .color(color::FG_1.current(ThemeMode::Dark))
        .width(Length::FillPortion(3))
        .align_x(iced::alignment::Horizontal::Right)
        .into()
}

fn currency_cell<'a, Msg>(value: &'a str) -> Element<'a, Msg>
where
    Msg: Clone + 'a,
{
    Text::new(value)
        .size(text::BODY)
        .color(color::FG_3.current(ThemeMode::Dark))
        .width(Length::FillPortion(2))
        .align_x(iced::alignment::Horizontal::Center)
        .into()
}

/// Render a `Timestamp` as RFC 3339, monospaced. The cockpit's `widgets::num`
/// helpers don't carry a timestamp formatter — the `Timestamp` upstream
/// already exposes a `Display` impl that renders RFC 3339, which is what
/// the modal needs.
fn format_timestamp(ts: trading_core::Timestamp) -> String {
    // `Timestamp::Display` already produces RFC 3339 with microsecond
    // precision; the modal renders it verbatim.
    ts.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use trading_core::{AccountId, Money, Timestamp};

    fn fixture_entry(account: &str, debit: i64, credit: i64, currency: &str) -> JournalEntry {
        JournalEntry {
            account: AccountId::new(account),
            debit: Money::from_decimal(rust_decimal::Decimal::from(debit)),
            credit: Money::from_decimal(rust_decimal::Decimal::from(credit)),
            currency: SmolStr::new(currency),
            ts: Timestamp::now(),
            memo: SmolStr::new(""),
        }
    }

    fn fixture_view() -> JournalTransactionView {
        JournalTransactionView {
            tx_id: SmolStr::new("4f9a2c1e-aaaa-bbbb-cccc-000000000001"),
            ts: Timestamp::now(),
            description: SmolStr::new("Buy 0.4 BTCUSDT @ 52,341.20"),
            strategy_id: Some(trading_core::StrategyId::new("sma-cross-btc-1m")),
            entries: vec![
                fixture_entry("assets:cash:USDT", 0, 52341, "USDT"),
                fixture_entry("assets:position:BTCUSDT", 1, 0, "BTC"),
            ],
        }
    }

    /// Loading state renders the modal without panicking. This is a smoke
    /// test — exhaustive UI rendering is covered by `panel_snapshots.rs`
    /// in T1208.
    #[test]
    fn loading_renders_without_panic() {
        let state = JournalModalState {
            tx_id: SmolStr::new("tx-loading"),
            entries: PanelState::Loading,
        };
        // Dummy `()` message type — the widget is generic and the test
        // only exercises the rendering path.
        let _: Element<()> = view(&state, Container::new(Text::new("cockpit")).into(), ());
    }

    /// Empty state renders without panic.
    #[test]
    fn empty_renders_without_panic() {
        let state = JournalModalState {
            tx_id: SmolStr::new("tx-empty"),
            entries: PanelState::Empty,
        };
        let _: Element<()> = view(&state, Container::new(Text::new("cockpit")).into(), ());
    }

    /// Error state renders without panic.
    #[test]
    fn error_renders_without_panic() {
        let state = JournalModalState {
            tx_id: SmolStr::new("tx-error"),
            entries: PanelState::Error(SmolStr::new("ledger unreachable")),
        };
        let _: Element<()> = view(&state, Container::new(Text::new("cockpit")).into(), ());
    }

    /// Ready state with a 2-entry transaction renders without panic.
    #[test]
    fn ready_renders_without_panic() {
        let state = JournalModalState {
            tx_id: SmolStr::new("4f9a2c1e-aaaa-bbbb-cccc-000000000001"),
            entries: PanelState::Ready(fixture_view()),
        };
        let _: Element<()> = view(&state, Container::new(Text::new("cockpit")).into(), ());
    }

    /// `fmt_usdt` is the single number-formatting source for debit/credit
    /// cells — guards against a future "just one inline format!" drift.
    #[test]
    fn debit_credit_formatting_matches_num_helper() {
        let entry = fixture_entry("assets:cash:USDT", 52341, 0, "USDT");
        assert_eq!(fmt_usdt(entry.debit.amount()), "52,341.00 USDT");
        assert_eq!(fmt_usdt(dec!(0)), "0.00 USDT");
    }
}
