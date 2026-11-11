# Themes

Each `.json` file here is a stock theme, embedded in the engine (`src/theme.rs`).
The file's stem is the theme's id. User themes will use the same format.

```json
{
  "name": "Charcoal",
  "appearance": "dark",
  "colors":    { "surface": "#18181b", ... },
  "playlist":  { "clipWave": "#a1a1aa" },
  "fourtrack": { "capTop": "#303036" }
}
```

- `name` is shown in Preferences. `appearance` is `"light"` or `"dark"`.
- Colors are written `#rgb`, `#rrggbb` or `#rrggbbaa`.
- Required keys must be given. Every other key may be left out, and is then
  derived as the tables below say.
- An unknown key or section only produces a warning.

Derivations:

- `ref(k)`: the value of `k`.
- `mix(a, b, t)`: per channel, alpha included, `a × (1 − t) + b × t`, rounded.
- `alpha(k, a)`: `k` with its alpha set to `a`.

Keys are resolved in table order, so a derivation only uses keys above it, or
earlier sections.

The resolved theme is one flat map: core keys by name (`surface`), section keys
as `section.key` (`playlist.clip`). Values are lowercase `#rrggbb`, or
`#rrggbbaa` when the alpha isn't `ff`.

## `colors` (core)

| Key | Use | Default |
|-----|-----|---------|
| `surface` | app and canvas background, lanes | required |
| `panel` | toolbars, ruler, four-track plastic | required |
| `raised` | inputs, clip bodies, key caps | required |
| `sunken` | slots, displays, scrollbar track | required |
| `line` | borders, lane and beat lines | required |
| `lineStrong` | bar lines, input borders | required |
| `inkStrong` | headings | required |
| `ink` | body text | required |
| `inkSoft` | secondary text, waveforms | required |
| `inkFaint` | hints, ruler numbers, disabled | required |
| `accent` | playhead, selection, latched keys | required |
| `accentSoft` | drop targets, quiet accents | required |
| `accentTint` | selected backgrounds | required |
| `accentInk` | text on `accentTint` | required |
| `led` | modified dot, pattern hits, amber LED | required |
| `ledTint` | pattern clips, notice boxes | required |
| `ledInk` | text in notice boxes | required |
| `danger` | errors, destructive hovers | required |
| `dangerTint` | error boxes | required |
| `surfaceAlt` | every other lane, subtle fills | `mix(surface, panel, 0.5)` |
| `inkMid` | between `ink` and `inkSoft` | `mix(ink, inkSoft, 0.5)` |
| `accentFocus` | focus rings, hovered resizers | `mix(accent, accentSoft, 0.5)` |
| `accentStrong` | strong accent strokes | `mix(accent, accentInk, 0.5)` |
| `accentTintStrong` | selected labels | `mix(accentTint, accentSoft, 0.25)` |
| `ledLine` | notice box borders | `mix(led, ledTint, 0.45)` |
| `dangerBright` | the record dot, meter peak | `ref(danger)` |
| `dangerLine` | error box borders | `mix(dangerTint, danger, 0.25)` |
| `dangerInk` | text in error boxes | `mix(danger, inkStrong, 0.6)` |
| `ok` | meters, the lime LED | `#84cc16` |
| `shadow` | shadow color, its opacity set where used | `#000000` |

## `playlist` (the playlist canvas)

| Key | Default |
|-----|---------|
| `background` | `ref(surface)` |
| `laneAlt` | `ref(surfaceAlt)` |
| `laneLine` | `ref(line)` |
| `beatLine` | `ref(line)` |
| `barLine` | `ref(lineStrong)` |
| `ruler` | `ref(panel)` |
| `rulerLine` | `ref(lineStrong)` |
| `rulerText` | `ref(inkFaint)` |
| `rulerBarText` | `ref(inkMid)` |
| `playhead` | `ref(accent)` |
| `playheadStopped` | `ref(inkFaint)` |
| `loopEnd` | `ref(inkMid)` |
| `clip` | `ref(raised)` |
| `clipBorder` | `ref(lineStrong)` |
| `clipLabel` | `ref(panel)` |
| `clipText` | `ref(ink)` |
| `clipWave` | `ref(inkSoft)` |
| `modified` | `ref(led)` |
| `patternClip` | `ref(ledTint)` |
| `patternLabel` | `mix(ledTint, led, 0.15)` |
| `patternBorder` | `mix(led, ledLine, 0.5)` |
| `patternHit` | `mix(led, ledInk, 0.25)` |
| `patternBeat` | `mix(ledLine, ledTint, 0.4)` |
| `selectedClip` | `ref(accentTint)` |
| `selectedLabel` | `ref(accentTintStrong)` |
| `selectedBorder` | `ref(accent)` |
| `lassoFill` | `ref(accent)` (drawn faint) |
| `accent` | `ref(accent)` |
| `accentSoft` | `ref(accentSoft)` |
| `idle` | `ref(inkFaint)` |
| `scrollbar` | `ref(inkSoft)` |
| `debug` | `#ffbb99` (the debug overlay's panels and guides, drawn faint) |
| `debugText` | `ref(inkStrong)` |

## `waveform` (the clip page's waveform)

| Key | Default |
|-----|---------|
| `background` | `ref(surface)` |
| `ruler` | `ref(panel)` |
| `rulerLine` | `ref(lineStrong)` |
| `grid` | `ref(line)` |
| `beat` | `ref(inkFaint)` |
| `centre` | `ref(lineStrong)` |
| `wave` | `ref(accentStrong)` |
| `dimmed` | `ref(lineStrong)` |
| `wash` | `alpha(panel, 0.6)` |
| `fill` | `alpha(accent, 0.12)` |
| `envelope` | `ref(led)` |
| `fade` | `alpha(led, 0.07)` |

## `meter` (mixer meters)

| Key | Default |
|-----|---------|
| `low` | `ref(ok)` |
| `mid` | `ref(led)` |
| `high` | `ref(dangerBright)` |
| `hold` | `ref(inkMid)` |

## `fourtrack` (the four-track controls and cassette)

`fourtrack.someKey` becomes the CSS variable `--ft-some-key`.

| Key | Use | Default |
|-----|-----|---------|
| `ink` | | `ref(ink)` |
| `inkSoft` | | `ref(inkSoft)` |
| `inkFaint` | | `ref(inkFaint)` |
| `accent` | knob pointers, latched keys | `ref(accent)` |
| `focus` | | `ref(accentFocus)` |
| `panel` | plastic | `ref(panel)` |
| `line` | | `ref(line)` |
| `lineStrong` | | `ref(lineStrong)` |
| `capTop` | key cap gradient | `ref(raised)` |
| `capBottom` | | `mix(raised, panel, 0.8)` |
| `capDown` | a pressed key | `mix(panel, sunken, 0.2)` |
| `display` | readouts, the counter | `mix(panel, sunken, 0.6)` |
| `displayText` | | `ref(inkMid)` |
| `slot` | the slot a fader or meter sits in | `mix(sunken, lineStrong, 0.6)` |
| `tray` | the recess keys sit in | `mix(panel, sunken, 0.65)` |
| `face` | strips, focused fields, wheels | `ref(raised)` |
| `knobTop` | knob and fader cap gradient | `ref(raised)` |
| `knobBottom` | | `ref(panel)` |
| `dialA` | a knob's ridged ring | `ref(line)` |
| `dialB` | | `ref(panel)` |
| `meterGap` | between LED meter segments | `alpha(surface, 0.7)` |
| `danger` | a destructive key's hover | `ref(danger)` |
| `record` | the record key's dot | `ref(dangerBright)` |
| `ledRedOn` | | `ref(dangerBright)` |
| `ledRedGlow` | | `alpha(ledRedOn, 0.45)` |
| `ledRedOff` | | `mix(ledRedOn, sunken, 0.85)` |
| `ledAmberOn` | | `ref(led)` |
| `ledAmberGlow` | | `alpha(ledAmberOn, 0.45)` |
| `ledAmberOff` | | `mix(ledAmberOn, sunken, 0.85)` |
| `ledLimeOn` | | `ref(ok)` |
| `ledLimeGlow` | | `alpha(ledLimeOn, 0.45)` |
| `ledLimeOff` | | `mix(ledLimeOn, sunken, 0.85)` |
| `ledBlueOn` | | `ref(accent)` |
| `ledBlueGlow` | | `alpha(ledBlueOn, 0.45)` |
| `ledBlueOff` | | `mix(ledBlueOn, sunken, 0.85)` |
| `tape` | cassette tape | `ref(inkFaint)` |
| `label` | cassette label | `ref(surfaceAlt)` |
| `stripe` | the label's second stripe | `ref(accentSoft)` |
| `hub` | reel hubs, teeth, pins, rollers | `ref(raised)` |
| `tapeMarks` | marks on the tape pack | `alpha(surface, 0.55)` |
