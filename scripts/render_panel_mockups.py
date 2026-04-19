#!/usr/bin/env python3
"""Render cockpit panel mockup PNGs from logical-state .txt artifacts.

Uses ui::theme tokens for colors so the mockups stay faithful to the cockpit
design system. These are honest layout/typography mockups, not pixel-perfect
iced renders — iced needs a wgpu surface which the headless sandbox lacks.
For actual pixel screenshots, the operator runs `cargo run --bin cockpit
--features fixtures` and uses the OS screenshot tool (see smoke checklist).
"""

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path("/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading")
DIR = ROOT / "spec/reports/screenshots/v0-paper-sma"

# crates/ui/src/theme.rs color tokens (as RGB tuples)
BG       = (0x11, 0x14, 0x1A)
BG_ELEV  = (0x1A, 0x1F, 0x29)
FG       = (0xE8, 0xEC, 0xF2)
FG_MUTED = (0x8B, 0x93, 0xA3)
ACCENT   = (0x5E, 0xA3, 0xFF)
POS      = (0x3E, 0xCF, 0x8E)
NEG      = (0xFF, 0x6B, 0x6B)
WARN     = (0xFF, 0xC4, 0x5A)
BORDER   = (0x2A, 0x31, 0x3F)

MENLO = "/System/Library/Fonts/Menlo.ttc"


def font(size, bold=False):
    idx = 2 if bold else 0
    try:
        return ImageFont.truetype(MENLO, size, index=idx)
    except OSError:
        return ImageFont.load_default()


W, H = 720, 460


def state_chip_color(state):
    return {
        "loading": FG_MUTED,
        "empty": FG_MUTED,
        "error": NEG,
        "ready": POS,
    }.get(state, ACCENT)


def state_label(state):
    return {
        "loading": "LOADING",
        "empty": "EMPTY",
        "error": "ERROR",
        "ready": "READY",
    }.get(state, state.upper())


def key_value_color(state, key, value):
    v = value.strip()
    if state == "error" and key in {"banner", "reason"}:
        return NEG
    if state == "error" and key in {"hint", "runbook_link_label"}:
        return WARN if key == "hint" else ACCENT
    if state == "ready":
        if key in {"matched", "confirm_enabled"}:
            return POS if "true" in v.lower() else NEG
        if key == "typed":
            return FG
        if key == "dialog_title":
            return FG
    if key in {"delta", "pnl_pct", "return_pct"}:
        try:
            n = float(v.replace("%", "").replace(",", "").strip())
            return POS if n > 0 else NEG if n < 0 else FG_MUTED
        except Exception:
            return FG
    return FG


def render(txt_path):
    name = txt_path.stem  # e.g. "tape-ready"
    panel, _, state = name.partition("-")
    content = txt_path.read_text()

    img = Image.new("RGB", (W, H), BG)
    draw = ImageDraw.Draw(img)

    # Panel card
    pad = 16
    draw.rounded_rectangle(
        (pad, pad, W - pad, H - pad),
        radius=4,
        fill=BG_ELEV,
        outline=BORDER,
        width=1,
    )

    # Header row
    title = f"cockpit  ·  {panel}"
    draw.text((pad + 16, pad + 12), title, fill=FG, font=font(16, bold=True))

    chip_text = state_label(state)
    chip_color = state_chip_color(state)
    chip_font = font(11, bold=True)
    chip_tw = draw.textlength(chip_text, font=chip_font)
    chip_pad_x = 8
    chip_h = 18
    chip_x2 = W - pad - 16
    chip_x1 = chip_x2 - chip_tw - chip_pad_x * 2
    chip_y1 = pad + 14
    draw.rounded_rectangle(
        (chip_x1, chip_y1, chip_x2, chip_y1 + chip_h),
        radius=2,
        fill=chip_color,
    )
    draw.text(
        (chip_x1 + chip_pad_x, chip_y1 + 2),
        chip_text,
        fill=BG,
        font=chip_font,
    )

    # Separator
    sep_y = pad + 48
    draw.line((pad + 16, sep_y, W - pad - 16, sep_y), fill=BORDER, width=1)

    # Body — render .txt content with basic key:value styling
    y = sep_y + 14
    body = font(12)
    cap = font(11)
    title_f = font(18, bold=True)

    for raw in content.splitlines():
        if y > H - pad - 28:
            draw.text((pad + 16, y), "…", fill=FG_MUTED, font=cap)
            break
        line = raw.rstrip()
        if not line:
            y += 6
            continue
        # nested note bullet
        if line.lstrip().startswith("- "):
            draw.text((pad + 28, y), line.strip(), fill=FG_MUTED, font=cap)
            y += 14
            continue
        # quoted continuation / plain indented text
        if line.startswith("  "):
            draw.text((pad + 28, y), line.strip(), fill=FG_MUTED, font=cap)
            y += 14
            continue
        # key: value
        if ":" in line and not line.startswith("#"):
            key, _, val = line.partition(":")
            key = key.strip()
            val = val.strip()
            # banner line gets elevated display
            if key == "banner" and state == "error":
                draw.text((pad + 16, y), val, fill=NEG, font=title_f)
                y += 26
                continue
            # headline values render larger
            if key in {"total_equity", "cumulative_return", "headline"}:
                draw.text((pad + 16, y), f"{key}:", fill=FG_MUTED, font=body)
                kw = draw.textlength(f"{key}:", font=body)
                draw.text(
                    (pad + 16 + kw + 10, y - 2),
                    val,
                    fill=key_value_color(state, key, val),
                    font=font(16, bold=True),
                )
                y += 22
                continue
            draw.text((pad + 16, y), f"{key}:", fill=FG_MUTED, font=body)
            kw = draw.textlength(f"{key}:", font=body)
            draw.text(
                (pad + 16 + kw + 10, y),
                val,
                fill=key_value_color(state, key, val),
                font=body,
            )
            y += 16
            continue
        # plain line
        draw.text((pad + 16, y), line, fill=FG_MUTED, font=cap)
        y += 14

    # Footer tag
    footer = f"v0 cockpit mockup · ui::theme · panel={panel} · state={state}"
    draw.text(
        (pad + 16, H - pad - 18),
        footer,
        fill=FG_MUTED,
        font=font(10),
    )

    png_path = txt_path.with_suffix(".png")
    img.save(png_path, optimize=True)
    return png_path


written = []
for txt in sorted(DIR.glob("*.txt")):
    out = render(txt)
    written.append(out.name)

print("\n".join(written))
print(f"\ntotal: {len(written)} PNGs written to {DIR}")
