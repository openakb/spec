# OpenAKB brand assets

Logo lockups for OpenAKB, in **SVG** (the vector source of truth) and **PNG**
(raster, for Markdown/READMEs and other renderers that don't handle SVG reliably).

## Variants

| File stem | Lockup | Use for |
| --- | --- | --- |
| `openakb-horizontal-{light,dark}` | Icon + wordmark, side by side | Wide headers, README banners |
| `openakb-stacked-{light,dark}` | Icon above the wordmark | Square-ish spaces, cards |
| `openakb-icon-{light,dark}` | Icon only | Favicons, avatars, social |

- **`-light`** — for light backgrounds: dark navy wordmark on a transparent field.
- **`-dark`** — for dark or busy backgrounds: light wordmark on a navy rounded field.

PNG dimensions are ~4× the vector size for crisp high-DPI display (horizontal
880×240, stacked 600×432, icon 512×512); scale down as needed.

## Typography

The wordmark is set in **Acme** ([SIL Open Font License 1.1][ofl]) and converted to
vector outlines, so every SVG is self-contained and renders identically without the
font installed. Source: <https://fonts.google.com/specimen/Acme>.

[ofl]: https://openfontlicense.org/

## Using the logo in Markdown

Swap light/dark automatically by the reader's color theme:

```html
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/logo/openakb-horizontal-dark.png">
  <img alt="OpenAKB" src="assets/logo/openakb-horizontal-light.png" width="280">
</picture>
```
