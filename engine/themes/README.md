# Themes

Each `.json` file here is a stock theme, embedded in the engine
(`src/theme.rs`). The file's stem is the theme's id. User themes will use the
same format.

**`base.json` lists every key a theme has, and what each one is when a theme
leaves it out.** A theme gives the `required` keys, then whatever else it
wants to differ from the base.

```json
{
  "name": "Charcoal",
  "appearance": "dark",
  "colors":    { "surface": "#18181b", ... },
  "playlist":  { "clip": "mix(panel, raised, 0.5)" },
  "fourtrack": { "capTop": "#303036" }
}
```

- `name` is shown in Preferences. `appearance` is `"light"` or `"dark"`.
- An unknown key or section only produces a warning, so an older app can read
  a newer theme.

## Values

A value, in a theme or in `base.json`, is one of:

| Form | Means |
|------|-------|
| `#rgb`, `#rrggbb`, `#rrggbbaa` | a color |
| `surface`, `fourtrack.ledRedOn` | another key's color. A bare name is a `colors` key; `section.key` is a key of a section |
| `mix(a, b, t)` | per channel, alpha included: `a × (1 − t) + b × t`, rounded. `a` and `b` are keys, colors or expressions |
| `alpha(k, a)` | `k` with its alpha set to `a` (0 to 1) |
| `required` | (`base.json` only) the theme must give this key |

Keys may refer to each other in any order; a loop is an error.

The engine resolves a theme into one flat map. Core keys are listed by name
(`surface`) and section keys as `section.key` (`playlist.clip`). Values are
lowercase `#rrggbb`, or `#rrggbbaa` when the alpha isn't `ff`. The frontend
only ever sees that map.

`test/slate-light.resolved.json` is Slate Light fully resolved. A test checks
it, so a change to `base.json` can't quietly change the default light theme.
If a change to Slate Light is intended, update the snapshot in the same
commit.

## The core keys (`colors`)

| Key | Use |
|-----|-----|
| `surface` | app and canvas background, lanes |
| `panel` | toolbars, ruler, four-track plastic |
| `raised` | inputs, clip bodies, key caps |
| `sunken` | slots, displays, scrollbar track |
| `line` | borders, lane and beat lines |
| `lineStrong` | bar lines, input borders |
| `inkStrong` | headings |
| `ink` | body text |
| `inkSoft` | secondary text, waveforms |
| `inkFaint` | hints, ruler numbers, disabled |
| `accent` | playhead, selection, latched keys |
| `accentSoft` | drop targets, quiet accents |
| `accentTint` | selected backgrounds |
| `accentInk` | text on `accentTint` |
| `led` | modified dot, pattern hits, amber LED |
| `ledTint` | pattern clips, notice boxes |
| `ledInk` | text in notice boxes |
| `danger` | errors, destructive hovers |
| `dangerTint` | error boxes |
| `surfaceAlt` | every other lane, subtle fills |
| `inkMid` | between `ink` and `inkSoft` |
| `accentFocus` | focus rings, hovered resizers |
| `accentStrong` | strong accent strokes |
| `accentTintStrong` | selected labels |
| `ledLine` | notice box borders |
| `dangerBright` | the record dot, meter peak |
| `dangerLine` | error box borders |
| `dangerInk` | text in error boxes |
| `ok` | meters, the lime LED |
| `shadow` | shadow color, its opacity set where used |

## Sections

- **`playlist`**: the playlist canvas.
- **`waveform`**: the clip page's waveform.
- **`meter`**: the mixer meters.
- **`fourtrack`**: the four-track controls and cassette. `fourtrack.someKey`
  becomes the CSS variable `--ft-some-key`.

The key names follow what they paint. For example, `capTop`/`capBottom` are a
key cap's gradient, `tray` is the recess keys sit in, `face` covers strips,
focused fields and wheels, and `hub` covers the cassette's reel hubs, teeth,
pins and rollers.
