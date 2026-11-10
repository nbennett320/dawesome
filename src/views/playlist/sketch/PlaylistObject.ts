import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import Waveform from './Waveform'
import { ClipView, CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItem } from '../../../types/playlist'
import { P5BoundingBox, State as ComponentState } from '../../../render/P5Component'
import { theme } from './theme'
import { STEPS_PER_BEAT, litRows, patternLabel } from '../../../types/sequencer'
import { hiddenBeyond } from './edges'

interface Props extends PlaylistComponentProps {
  playlistItem: PlaylistItem
  // the waveform's levels (empty for a pattern clip)
  soundData: Float32Array
  duration: number
  trackNumber: number
  trackHeight: number
}

type State = ComponentState<Record<string, never>>

// room at the end of the label for the modified dot, in screen pixels
const MODIFIED_DOT_SPACE = 12
// room at either end of the label for the triangle that marks a trimmed
// edge, and the narrowest clip that shows them, in screen pixels
const TRIANGLE_SPACE = 8
const TRIANGLES_FROM_WIDTH = 28
// how wide the bar on an edge that's hovered or being dragged is
const EDGE_BAR_WIDTH = 2

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
    this.minHeight = this.trackHeight * this.trackNumber + (this.trackNumber * .3) + staticDefaults.timelineHeight
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
    return view ? hiddenBeyond(view.from, view.to, view.length) : { left: false, right: false }
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
    // a pattern clip's amber)
    const selected = (this.renderer as Renderer)?.selectedIds.has(this.playlistItem.id)
    const isPattern = this.playlistItem.patternId !== undefined
    let colors = { border: theme.clipBorder, card: theme.clip, label: theme.clipLabel }
    if(isPattern) colors = { border: theme.patternBorder, card: theme.patternClip, label: theme.patternLabel }
    if(selected) colors = { border: theme.selectedBorder, card: theme.selectedClip, label: theme.selectedLabel }
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
    this.p.fill(...theme.clipText)
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
      this.p.fill(...theme.modified)
      this.p.circle(
        right * this.currentScale - MODIFIED_DOT_SPACE / 2 - rightSpace,
        top + this.labelHeight / 2,
        6,
      )
    }

    // a bar on the edge the mouse can grab, or is dragging, to trim the clip
    const bar = (this.renderer as Renderer)?.edgeBarOn?.(this.playlistItem.id)
    if(bar) {
      this.p.noStroke()
      this.p.fill(...theme.accent)
      this.p.rect(
        bar === 'left' ? left * scale : right * scale - EDGE_BAR_WIDTH,
        cardTop,
        EDGE_BAR_WIDTH,
        cardHeight,
      )
    }

    if((this.renderer as Renderer)?.debugMode) {
      // text width debug
      this.p.fill(255,187,153,255*.3)
      this.p.rect(
        (left + 1) * this.currentScale,
        top + 6,
        width * this.currentScale - 1,
        this.labelHeight,
      )
    }

    this.p.pop()
  }

  // a pattern clip's body: a lane per sequencer row, with a cell for each
  // lit step, over faint beat lines
  drawPattern = () => {
    const pattern = this.#pattern()
    const { left, top, width, height } = this.waveform.boundingBox()
    const bodyTop = top + this.labelHeight + 3
    const bodyHeight = height - this.labelHeight - 7
    if(!pattern || pattern.length <= 0 || bodyHeight <= 0) return

    // the steps the clip shows: the whole pattern, or a trimmed clip's part
    // of it (which can start or end between two steps)
    const from = this.#view ? this.#view.from : 0
    const to = this.#view ? this.#view.to : pattern.length
    if(to <= from) return
    const stepWidth = width / (to - from)
    const right = left + width
    const stepX = (step: number) => left + (step - from) * stepWidth
    const last = Math.min(to, pattern.length)

    this.p.strokeWeight(1 / this.currentScale)
    this.p.stroke(...theme.patternBeat)
    const firstBeat = (Math.floor(from / STEPS_PER_BEAT) + 1) * STEPS_PER_BEAT
    for(let step = firstBeat; step < last; step += STEPS_PER_BEAT) {
      const x = stepX(step)
      this.p.line(x, bodyTop, x, bodyTop + bodyHeight)
    }

    const lanes = litRows(pattern, (this.renderer as Renderer).sequencer.rows)
    if(!lanes.length) return
    const laneHeight = bodyHeight / lanes.length
    // a little gap around each cell, unless they'd vanish
    const gapX = Math.min(stepWidth * .15, 1 / this.currentScale)
    const gapY = Math.min(laneHeight * .15, 1)

    this.p.noStroke()
    this.p.fill(...theme.patternHit)
    lanes.forEach((steps, lane) => {
      const y = bodyTop + lane * laneHeight
      // only the hits that start inside the clip, as it plays them
      steps.filter(step => step >= from && step < last).forEach(step => {
        const x = stepX(step)
        const cell = Math.min(stepWidth, right - x)
        if(cell > gapX * 2) this.p.rect(x + gapX, y + gapY, cell - gapX * 2, laneHeight - gapY * 2)
      })
    })
  }

  render = () => {
    const { waveform } = this

    // a pattern clip is as long as its pattern, which can change at any
    // time, and any clip changes while it's trimmed
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
      this.p.stroke(255,187,153)
      this.p.line(left, pixelOffset.yOffset, right, pixelOffset.yOffset)

    }
  }
}

export default PlaylistObject
