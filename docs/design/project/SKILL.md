---
name: lumen-design
description: Use this skill to generate well-branded interfaces and assets for Lumen, the AI-driven trading platform, either for production or throwaway prototypes/mocks/etc. Contains essential design guidelines, colors, type, fonts, assets, and UI kit components for prototyping.
user-invocable: true
---

Read the README.md file within this skill, and explore the other available files.

If creating visual artifacts (slides, mocks, throwaway prototypes, etc), copy assets out and create static HTML files for the user to view. If working on production code, you can copy assets and read the rules here to become an expert in designing with this brand.

If the user invokes this skill without any other guidance, ask them what they want to build or design, ask some questions, and act as an expert designer who outputs HTML artifacts _or_ production code, depending on the need.

## Quick orientation
- `README.md` — full brand book (voice, visual foundations, iconography).
- `colors_and_type.css` — every design token, both light and dark.
- `assets/brand/` — logo, monogram, AI lens variant, wordmark (SVG).
- `assets/icons/` — icon notes (Lucide is the icon system; load from CDN).
- `preview/` — design-system cards demonstrating each piece in isolation.
- `ui_kits/desktop/` — full desktop trading app recreation (React + JSX).

## Lumen rules of thumb
- Calm > clever. No exclamation marks, no emoji, no hype.
- One accent color (muted teal). Up = sage, down = clay. Never neon.
- Numerics are JetBrains Mono with tabular figures, always.
- Whisper shadows + 1 px hairlines. No heavy elevation.
- Minus sign is `−` (U+2212), not a hyphen.
- Light and dark are first-class — design and verify both.
