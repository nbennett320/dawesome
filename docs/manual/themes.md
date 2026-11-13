---
title: Themes
---

# 13 Themes

Dawesome has a light and a dark appearance, and a choice of color themes for
each.

## 13-1 Choosing light, dark or system

1. Choose **Preferences ▸ Appearance…** (or **Dawesome ▸ Preferences…**,
   **⌘, (Ctrl+,)**). The **Preferences** tab opens.
2. Set **Appearance**:
   - **System** — follows your computer's light or dark setting, and changes
     when it does.
   - **Light** — always light.
   - **Dark** — always dark.

Changes apply as you make them. There is no **OK** button.

![The Preferences tab](./img/preferences.png)
<!-- SCREENSHOT: Preferences tab, Appearance / Light theme / Dark theme / Plugins open -->

## 13-2 Built-in themes

| Theme | Appearance | Note |
| --- | --- | --- |
| **Slate Light** | Light | Default light theme |
| **Charcoal Light** | Light | |
| **Banana Split** | Light | |
| **Southern Lapwing** | Light | |
| **Mauve** | Light | |
| **Charcoal** | Dark | Default dark theme |
| **Slate Dark** | Dark | |
| **Taiga** | Dark | |
| **Nightshade** | Dark | |
| **Coup de Grâce** | Dark | |

<!-- SCREENSHOT: gallery, the playlist in each of the ten themes -->

To choose the theme for each appearance:

1. Open the **Preferences** tab (see [13-1](#_13-1-choosing-light-dark-or-system)).
2. Pick a theme from **Light theme**. It is used whenever the app is light.
3. Pick a theme from **Dark theme**. It is used whenever the app is dark.

Each list shows only the themes made for that appearance. Both lists stay
available whatever **Appearance** is set to, so you can set either one ahead
of time.

::: tip NOTE
Your choices are saved in `preferences.json` in Dawesome's settings folder
(see [14-5](./troubleshooting.md)). If a saved theme can't be found, that
appearance falls back to its default theme.
:::

## 13-3 Writing a custom theme

<!-- PROVISIONAL -->

::: warning CAUTION
This version of Dawesome can't load theme files of your own yet. It only
offers the ten built-in themes. The format below is the one the built-in
themes use, and the one user themes are planned to use.
:::

A theme is a JSON file. The file's name, without `.json`, is the theme's id.

```json
{
  "name": "My Theme",
  "appearance": "dark",
  "colors": {
    "surface": "#18181b",
    "panel": "#27272a",
    "accent": "#38bdf8"
  },
  "playlist": { "clip": "mix(panel, raised, 0.5)" },
  "fourtrack": { "capTop": "#303036" }
}
```

(A real theme must also give every required key; see 13-3-2.)

### 13-3-1 Top-level fields

| Field | Meaning |
| --- | --- |
| `name` | The name shown in **Preferences**. Required. |
| `appearance` | `"light"` or `"dark"`. Required. Decides which list the theme appears in. |
| `colors` | The core colors. |
| `playlist` | Colors of the playlist canvas. |
| `waveform` | Colors of the waveform on the **Sample** tab. |
| `meter` | Colors of the mixer meters. |
| `fourtrack` | Colors of the four-track controls and the cassette. |

### 13-3-2 Required keys

A theme must give these 19 keys in `colors`:

`surface`, `panel`, `raised`, `sunken`, `line`, `lineStrong`, `inkStrong`,
`ink`, `inkSoft`, `inkFaint`, `accent`, `accentSoft`, `accentTint`,
`accentInk`, `led`, `ledTint`, `ledInk`, `danger`, `dangerTint`.

Every other key (119 in all) has a default worked out from these. Give a key
only when you want it to differ from its default.

| Key | Paints |
| --- | --- |
| `surface` | App and canvas background, lanes |
| `panel` | Toolbars, ruler, four-track plastic |
| `raised` | Inputs, clip bodies, key caps |
| `sunken` | Slots, displays, scrollbar track |
| `line` | Borders, lane and beat lines |
| `lineStrong` | Bar lines, input borders |
| `inkStrong` | Headings |
| `ink` | Body text |
| `inkSoft` | Secondary text, waveforms |
| `inkFaint` | Hints, ruler numbers, disabled controls |
| `accent` | Playhead, selection, latched keys |
| `accentSoft` | Drop targets, quiet accents |
| `accentTint` | Selected backgrounds |
| `accentInk` | Text on `accentTint` |
| `led` | Modified dot, pattern hits, amber LED |
| `ledTint` | Pattern clips, notice boxes |
| `ledInk` | Text in notice boxes |
| `danger` | Errors, destructive hovers |
| `dangerTint` | Error boxes |

The full list of keys, with each default, is in
[`engine/themes/base.json`](https://github.com/nbennett320/dawesome/blob/main/engine/themes/base.json),
and what each one paints is in
[`engine/themes/README.md`](https://github.com/nbennett320/dawesome/blob/main/engine/themes/README.md).
<!-- VERIFY: these files are on branch vsts at c2295cb; confirm they are on main before the links go live -->

### 13-3-3 Values

Every value is a string in one of these forms:

| Form | Meaning |
| --- | --- |
| `#rgb`, `#rrggbb`, `#rrggbbaa` | A color. |
| `surface` | Another `colors` key's color. |
| `fourtrack.ledRedOn` | A section key's color (`section.key`). |
| `mix(a, b, t)` | `a` and `b` blended: `t` = 0 is all `a`, 1 is all `b`. Alpha is blended too. `a` and `b` can be keys, colors or other expressions. |
| `alpha(k, a)` | `k` with its opacity set to `a` (0 to 1). |

Keys may refer to each other in any order, but not in a loop.

### 13-3-4 What makes a theme fail

The theme is refused if:

- it isn't valid JSON, or `name` or `appearance` is missing or wrong;
- a required key is missing;
- a value isn't a color, a known key or a valid `mix`/`alpha` (for example a
  `mix` weight outside 0 to 1);
- keys refer to each other in a loop.

The message names the key at fault. An unknown key or section only gives a
warning, so a theme written for a newer version still loads.

## 13-4 Plugin windows

<!-- PROVISIONAL -->

The **Plugins open** setting on the same tab is described in
[Instruments and effects](./plugins.md).
