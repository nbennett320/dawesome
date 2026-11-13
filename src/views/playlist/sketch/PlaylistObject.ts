import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import Waveform from './Waveform'
import { ClipView, CanvasProps, Renderer, staticDefaults, trackTop } from './index'
import { PlaylistItem } from '../../../types/playlist'
import { P5BoundingBox, State as ComponentState } from '../../../render/P5Component'
import { paint, theme } from './theme'
import { mixRgba } from '../../../theme/color'
import type { SectionKey } from '../../../theme/keys'
import { Pass, Pattern, STEPS_PER_BEAT, clipPasses, patternLabel } from '../../../types/sequencer'
import { hiddenBeyond, patternHiddenBeyond } from './edges'
import { SampleLane, SynthLane, firstFrom, patternLanes } from './patternDrawing'
import { drawEdgeBar } from '../../../editing/look'

interface Props extends PlaylistComponentProps {
  playlistItem: PlaylistItem
  // the waveform's levels (empty for a pattern clip)
  soundData: Float32Array
  duration: number
  trackNumber: number
  trackHeight: number
}

type ClipColor = SectionKey<'playlist'>
type State = ComponentState<Record<string, never>>

// how much of its track's color a sample clip's label band takes, like a
// colored label on a cassette. a pattern clip is in its track's color all
// over (see patternColors). a selected clip is sky, so the selection shows
const TRACK_TINT = 0.35

// room at the end of the label for the modified dot, in screen pixels
const MODIFIED_DOT_SPACE = 12
// room at either end of the label for the triangle that marks a trimmed
// edge, and the narrowest clip that shows them, in screen pixels
const TRIANGLE_SPACE = 8
const TRIANGLES_FROM_WIDTH = 28
// the widest a sample row's tick is in a pattern clip, in screen pixels
const TICK_WIDTH = 2

type PatternInk = ReturnType<Renderer['patternRgba']>

// where a pattern clip's body draws: the times through its pattern that are
// on the canvas, how wide a step is, and where a step of a time through is.
// `from` is where the clip starts in its pattern, and the canvas shows from
// step `shownFrom` to `shownTo` of the clip's, all in steps of the pattern
interface PatternBody {
  from: number
  passes: Pass[]
  stepWidth: number
  stepX: (pass: Pass, step: number) => number
  shownFrom: number
  shownTo: number
}

// the last step of a time through to draw a note from: its end, or the
// canvas's
const lastShown = ({ shownTo }: PatternBody, pass: Pass) => Math.min(pass.to, shownTo - pass.at)

class PlaylistObject extends PlaylistComponent<State> {
  playlistItem: PlaylistItem
  soundData: Float32Array
  duration: number
  trackNumber: number
  trackHeight: number = staticDefaults.trackHeight
  labelHeight: number = staticDefaults.PlaylistObject.labelHeight
  minHeight: number
  maxHeight: number
  waveform: Waveform
  // whether the clip's settings differ from the default, shown as a dot
  modified = false

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    playlist: Renderer,
    props: Props,
  ) {
    super(p, canvas, playlist, props)

    this.playlistItem = props.playlistItem
    this.soundData = props.soundData
    this.duration = props.duration
    this.trackNumber = props.trackNumber
    this.trackHeight = props?.trackHeight ?? staticDefaults.trackHeight
    this.minHeight = trackTop(this.trackNumber, this.trackHeight)
    this.maxHeight = this.minHeight + this.trackHeight
    this.waveform = new Waveform(
      this.p,
      this.canvas,
      playlist,
      {
        duration: this.duration,
        currentScale: this.currentScale,
        timelineHeight: this.timelineHeight,
        timelineWidth: this.timelineWidth,
        soundData: this.soundData,
        startBeat: this.playlistItem.startBeat,
        trackNumber: this.playlistItem.trackNumber,
        pixelOffset: this.playlistItem.pixelOffset,
        minHeight: this.minHeight,
        maxHeight: this.maxHeight,
        trackHeight: this.trackHeight,
      }
    )
  }

  // where the clip is and what of its source it shows, as last worked out
  #view: ClipView | null = null

  // follow where the renderer has the clip now: its start and length, and
  // the part of its sample or pattern it shows. these change when the clip
  // is trimmed, while an edge is dragged, and with the tempo
  syncView = () => {
    const renderer = this.renderer as Renderer
    const view = renderer?.viewOf?.(this) ?? null
    this.#view = view
    this.waveform.startBeat = view ? view.startBeat : this.playlistItem.startBeat

    const { patternId } = this.playlistItem
    if(view) {
      this.setDuration(view.seconds)
      this.waveform.range = view.length > 0 ? [view.from / view.length, view.to / view.length] : [0, 1]
    } else if(patternId !== undefined && renderer?.patternSecondsOf) {
      // (a pattern clip whose pattern is gone)
      this.setDuration(renderer.patternSecondsOf(patternId))
    }
  }

  boundingBox = (): P5BoundingBox => {
    this.syncView()
    return this.waveform.boundingBox()
  }

  // which edges hide some of the sample or pattern beyond them
  #hidden = (): { left: boolean, right: boolean } => {
    const view = this.#view
    if(!view) return { left: false, right: false }
    const hidden = this.playlistItem.patternId === undefined ? hiddenBeyond : patternHiddenBeyond
    return hidden(view.from, view.to, view.length)
  }

  // the pattern a pattern clip plays (undefined for a sample clip, or a
  // pattern that's gone)
  #pattern = () => {
    const { patternId } = this.playlistItem
    return patternId === undefined ? undefined : (this.renderer as Renderer)?.patternOf(patternId)
  }

  // the label as last fitted to the clip (see #fittedLabel)
  #fitted: { label: string, width: number, text: string } | null = null

  // as much of the label as fits on one line `width` pixels wide (the font
  // set up already). measured only when the label or the width changes,
  // rather than wrapped a character at a time every frame
  #fittedLabel = (width: number): string => {
    const label = this.#label()
    const fitted = this.#fitted
    if(fitted && fitted.label === label && fitted.width === width) return fitted.text

    let text = label
    if(width <= 0) text = ''
    else if(this.p.textWidth(label) > width) {
      // the longest start of it that fits
      let low = 0
      let high = label.length
      while(low < high) {
        const mid = Math.ceil((low + high) / 2)
        if(this.p.textWidth(label.slice(0, mid)) <= width) low = mid
        else high = mid - 1
      }
      text = label.slice(0, low)
    }

    this.#fitted = { label, width, text }
    return text
  }

  // what the label says: the sample's file name, or the pattern's name
  #label = (): string => {
    const { patternId, path } = this.playlistItem
    if(patternId !== undefined) return patternLabel(this.#pattern() ?? { id: patternId, name: null })
    return path.split(/[\\/]/).pop() ?? path
  }

  // how long the clip plays for, in seconds. changes with its pitch
  setDuration = (seconds: number) => {
    this.duration = seconds
    this.waveform.duration = seconds
  }

  // call function on right click
  // eslint-disable-next-line class-methods-use-this
  onDrop = (
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _ev: MouseEvent,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _data: {
      playlistObject: PlaylistObject
      mouseX: number
      mouseY: number
    }
  ) => {}

  drawBoundingBox = () => {
    const {
      top,
      bottom,
      left,
      right,
      height,
      width,
    } = this.waveform.boundingBox()
    // a card, inset a little from the track lines, with a label band
    const inset = 2
    const cardTop = top + inset
    const cardHeight = height - inset * 2
    // selected (shift+click, or a shift+drag box): sky instead of slate (or
    // a pattern clip's track color)
    const renderer = this.renderer as Renderer | undefined
    const selected = renderer?.selectedIds.has(this.playlistItem.id)
    const isPattern = this.playlistItem.patternId !== undefined
    const key = (k: ClipColor) => paint.rgba(k)
    let colors = { border: key('clipBorder'), card: key('clip'), label: key('clipLabel') }
    if(renderer) {
      const track = this.playlistItem.trackNumber
      if(isPattern) colors = renderer.patternRgba(track)
      else colors.label = mixRgba(renderer.trackRgba(track), colors.label, TRACK_TINT)
    }
    if(selected) colors = { border: key('selectedBorder'), card: key('selectedClip'), label: key('selectedLabel') }
    const { border } = colors
    // drawn in screen pixels across (like the label's text), so zoom doesn't
    // stretch the card's border or its corners
    const scale = this.currentScale
    this.p.push()
    this.p.scale(1 / scale, 1)
    this.p.strokeWeight(selected ? 1.5 : 1)
    this.p.stroke(...border)
    this.p.fill(...colors.card)
    this.p.rect(left * scale, cardTop, width * scale, cardHeight, 3)
    this.p.noStroke()
    this.p.fill(...colors.label)
    this.p.rect(left * scale + .5, cardTop + .5, width * scale - 1, this.labelHeight - inset - .5, 3, 3, 0, 0)
    this.p.strokeWeight(1)
    this.p.stroke(...border)
    this.p.line(left * scale, top + this.labelHeight, right * scale, top + this.labelHeight)

    // a small triangle in the label, pointing into the clip, at each edge
    // that hides some of the sample or pattern beyond it (a trimmed edge)
    const hidden = width * scale >= TRIANGLES_FROM_WIDTH ? this.#hidden() : { left: false, right: false }
    const labelMiddle = cardTop + (this.labelHeight - inset) / 2
    this.p.noStroke()
    this.p.fill(...border)
    if(hidden.left) {
      const x = left * scale + 3
      this.p.triangle(x, labelMiddle - 4, x, labelMiddle + 4, x + 5, labelMiddle)
    }
    if(hidden.right) {
      const x = right * scale - 3
      this.p.triangle(x, labelMiddle - 4, x, labelMiddle + 4, x - 5, labelMiddle)
    }
    const leftSpace = hidden.left ? TRIANGLE_SPACE : 0
    const rightSpace = hidden.right ? TRIANGLE_SPACE : 0

    // render text
    this.p.noStroke()
    paint.fill(this.p, 'clipText')
    this.p.textFont(theme.font)
    this.p.textSize(11)
    this.p.textAlign(this.p.LEFT, this.p.TOP)
    // leave room for the modified dot
    const dotSpace = this.modified ? MODIFIED_DOT_SPACE : 0
    this.p.text(
      // the file's name, without its folders, or the pattern's name: as much
      // as fits on the label
      this.#fittedLabel(Math.round(width * this.currentScale - 10 - dotSpace - leftSpace - rightSpace)),
      left * this.currentScale + 6 + leftSpace,
      cardTop + 3,
    )

    // a dot at the end of the label: this clip's settings were changed
    // in the Sample tab. drawn unscaled, so it stays round when zoomed
    if(this.modified) {
      paint.fill(this.p, 'modified')
      this.p.circle(
        right * this.currentScale - MODIFIED_DOT_SPACE / 2 - rightSpace,
        top + this.labelHeight / 2,
        6,
      )
    }

    // a bar on the edge the mouse can grab, or is dragging, to trim the clip
    const bar = (this.renderer as Renderer)?.edgeBarOn?.(this.playlistItem.id)
    if(bar) {
      drawEdgeBar(this.p.drawingContext as CanvasRenderingContext2D, bar, left * scale, right * scale, cardTop, cardHeight)
    }

    if((this.renderer as Renderer)?.debugMode) {
      // text width debug
      paint.fill(this.p, 'debug', 0.3)
      this.p.rect(
        (left + 1) * this.currentScale,
        top + 6,
        width * this.currentScale - 1,
        this.labelHeight,
      )
    }

    this.p.pop()
  }

  // a pattern clip's body: its pattern, played end to end from where the
  // clip starts in it, each time through after the first marked with a
  // faint repeat line (see clipPasses), over fainter beat lines. a lane for
  // each row that plays notes in it, in the rows' order: a sample row's
  // notes are short ticks where they start, and a synth row's small bars on
  // rows of their pitches, each cut at the end of its time through. only
  // the times through that are on the canvas are drawn, and of those, only
  // the notes that start on it
  drawPattern = () => {
    const pattern = this.#pattern()
    const view = this.#view
    const { left, top, width, height } = this.waveform.boundingBox()
    const bodyTop = top + this.labelHeight + 3
    const bodyHeight = height - this.labelHeight - 7
    if(!pattern || !view || pattern.length <= 0 || bodyHeight <= 0 || view.to <= view.from) return

    const body = this.#bodyOf(pattern, view, left, width)
    // in the track's color (see patternColors)
    const renderer = this.renderer as Renderer
    const ink = renderer.patternRgba(this.playlistItem.trackNumber)
    this.#drawLines(body, ink, bodyTop, bodyHeight)

    const lanes = patternLanes(pattern, renderer.sequencer.rows)
    if(!lanes.length) return
    const laneHeight = bodyHeight / lanes.length
    this.p.noStroke()
    this.p.fill(...ink.hit)
    lanes.forEach((lane, n) => {
      const laneTop = bodyTop + n * laneHeight
      if(lane.kind === 'sample') this.#drawTicks(body, lane, laneTop, laneHeight)
      else this.#drawBars(body, lane, laneTop, laneHeight)
    })
  }

  // where a pattern clip, `left` to `left + width`, draws the part of its
  // pattern it plays (`view.from` to `view.to`, in steps): every time through
  // the pattern that's on the canvas, and the steps on it
  #bodyOf = (pattern: Pattern, { from, to }: ClipView, left: number, width: number): PatternBody => {
    const stepWidth = width / (to - from)
    const ctx = this.p.drawingContext as CanvasRenderingContext2D
    const { a: scaleX, e: offsetX } = ctx.getTransform()
    const stepAt = (x: number) => from + (x - left) / stepWidth
    const shownFrom = stepAt(-offsetX / scaleX)
    const shownTo = stepAt((ctx.canvas.width - offsetX) / scaleX)
    return {
      from,
      passes: clipPasses(from, to - from, pattern.length, shownFrom, shownTo),
      stepWidth,
      stepX: (pass, step) => left + (pass.at + step - from) * stepWidth,
      shownFrom,
      shownTo,
    }
  }

  // a repeat line where each time through after the first starts, and a
  // beat line on each beat inside the times through
  #drawLines = ({ from, passes, stepX }: PatternBody, ink: PatternInk, top: number, height: number) => {
    this.p.strokeWeight(1 / this.currentScale)
    const line = (x: number) => this.p.line(x, top, x, top + height)
    passes.forEach(pass => {
      if(pass.at > from) {
        this.p.stroke(...ink.repeat)
        line(stepX(pass, 0))
      }
      this.p.stroke(...ink.beat)
      for(let step = (Math.floor(pass.from / STEPS_PER_BEAT) + 1) * STEPS_PER_BEAT; step < pass.to; step += STEPS_PER_BEAT) {
        line(stepX(pass, step))
      }
    })
  }

  // a sample row's notes: a short tick where each starts, at most half a
  // step wide, but never under a pixel
  #drawTicks = (body: PatternBody, { ticks }: SampleLane, top: number, height: number) => {
    const { passes, stepX, stepWidth, shownFrom } = body
    const scale = this.currentScale
    const tick = Math.max(Math.min(stepWidth / 2, TICK_WIDTH / scale), 1 / scale)
    // a little gap between lanes, unless they'd vanish
    const gap = Math.min(height * .15, 1)
    passes.forEach(pass => {
      // (a tick starting a step before the canvas can still show)
      const first = Math.max(pass.from, shownFrom - pass.at - 1)
      const last = lastShown(body, pass)
      const end = stepX(pass, pass.to)
      for(let i = firstFrom(ticks, step => step, first); i < ticks.length && ticks[i] < last; i += 1) {
        const x = stepX(pass, ticks[i])
        const width = Math.min(tick, end - x)
        if(width > 0) this.p.rect(x, top + gap, width, height - gap * 2)
      }
    })
  }

  // a synth row's notes: small bars on rows of their pitches
  #drawBars = (body: PatternBody, { bars, pitches }: SynthLane, top: number, height: number) => {
    const { passes, stepX } = body
    const rowHeight = height / pitches
    passes.forEach(pass => {
      const last = lastShown(body, pass)
      for(let i = firstFrom(bars, bar => bar.start, pass.from); i < bars.length && bars[i].start < last; i += 1) {
        const { start, end, row } = bars[i]
        const x = stepX(pass, start)
        const width = stepX(pass, Math.min(end, pass.to)) - x
        if(width > 0) this.p.rect(x, top + row * rowHeight, width, Math.max(1, rowHeight * 0.8))
      }
    })
  }

  render = () => {
    const { waveform } = this

    // a pattern clip's pattern can change at any time, and any clip
    // changes while it's trimmed
    const { patternId } = this.playlistItem
    this.syncView()

    // off the canvas (scrolled or zoomed away), there's nothing to draw:
    // with hundreds of clips most are. worked out from where this draws on
    // the canvas, so a clip being dragged is found where it's dragged to
    const ctx = this.p.drawingContext as CanvasRenderingContext2D
    const { a: scaleX, e: offsetX, d: scaleY, f: offsetY } = ctx.getTransform()
    const { left, right, top, bottom } = waveform.boundingBox()
    if(
      right * scaleX + offsetX < 0 || left * scaleX + offsetX > ctx.canvas.width
      || bottom * scaleY + offsetY < 0 || top * scaleY + offsetY > ctx.canvas.height
    ) return

    this.drawBoundingBox()

    // render waveform, or the pattern
    if(patternId === undefined) waveform.render()
    else this.drawPattern()

    if((this.renderer as Renderer)?.debugMode) {
      // mouse drop height debug
      const { pixelOffset } = this.playlistItem
      this.p.strokeWeight(2)
      paint.stroke(this.p, 'debug')
      this.p.line(left, pixelOffset.yOffset, right, pixelOffset.yOffset)

    }
  }
}

export default PlaylistObject
