import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponent, { PlaylistComponentProps } from './PlaylistComponent'
import Waveform from './Waveform'
import { CanvasProps, Renderer, staticDefaults } from './index'
import { PlaylistItem } from '../../../types/playlist'
import { P5BoundingBox, State as ComponentState } from '../../../render/P5Component'
import { theme } from './theme'
import { STEPS_PER_BEAT, litRows, patternLabel } from '../../../types/sequencer'

interface Props extends PlaylistComponentProps {
  playlistItem: PlaylistItem
  soundData: number[]
  duration: number
  trackNumber: number
  trackHeight: number
}

type State = ComponentState<Record<string, never>>

// room at the end of the label for the modified dot, in screen pixels
const MODIFIED_DOT_SPACE = 12

class PlaylistObject extends PlaylistComponent<State> {
  playlistItem: PlaylistItem
  soundData: number[] = []
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

  boundingBox = (): P5BoundingBox => this.waveform.boundingBox()

  // the pattern a pattern clip plays (undefined for a sample clip, or a
  // pattern that's gone)
  #pattern = () => {
    const { patternId } = this.playlistItem
    return patternId === undefined ? undefined : (this.renderer as Renderer)?.patternOf(patternId)
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
    this.p.strokeWeight(selected ? 1.5 : 1)
    this.p.stroke(...border)
    this.p.fill(...colors.card)
    this.p.rect(left, cardTop, width, cardHeight, 3)
    this.p.noStroke()
    this.p.fill(...colors.label)
    this.p.rect(left + .5, cardTop + .5, width - 1, this.labelHeight - inset - .5, 3, 3, 0, 0)
    this.p.strokeWeight(1)
    this.p.stroke(...border)
    this.p.line(left, top + this.labelHeight, right, top + this.labelHeight)

    // render text
    this.p.noStroke()
    this.p.fill(...theme.clipText)
    this.p.push()
    this.p.scale(1 / this.currentScale, 1)
    this.p.textFont(theme.font)
    this.p.textSize(11)
    this.p.textAlign(this.p.LEFT, this.p.TOP)
    this.p.textWrap(this.p.CHAR)
    // leave room for the modified dot
    const dotSpace = this.modified ? MODIFIED_DOT_SPACE : 0
    this.p.text(
      // the file's name, without its folders, or the pattern's name
      this.#label(),
      left * this.currentScale + 6,
      cardTop + 3,
      width * this.currentScale - 10 - dotSpace,
      this.labelHeight - inset - 3,
    )

    // a dot at the end of the label: this clip's settings were changed
    // in the Sample tab. drawn unscaled, so it stays round when zoomed
    if(this.modified) {
      this.p.fill(...theme.modified)
      this.p.circle(
        right * this.currentScale - MODIFIED_DOT_SPACE / 2,
        top + this.labelHeight / 2,
        6,
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

    const stepWidth = width / pattern.length

    this.p.strokeWeight(1 / this.currentScale)
    this.p.stroke(...theme.patternBeat)
    for(let step = STEPS_PER_BEAT; step < pattern.length; step += STEPS_PER_BEAT) {
      const x = left + step * stepWidth
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
      steps.forEach(step => {
        this.p.rect(left + step * stepWidth + gapX, y + gapY, stepWidth - gapX * 2, laneHeight - gapY * 2)
      })
    })
  }

  render = () => {
    const { waveform } = this

    // a pattern clip is as long as its pattern, which can change at any time
    const { patternId } = this.playlistItem
    if(patternId !== undefined) this.setDuration((this.renderer as Renderer).patternSecondsOf(patternId))

    this.drawBoundingBox()

    // render waveform, or the pattern
    if(patternId === undefined) waveform.render()
    else this.drawPattern()

    if((this.renderer as Renderer)?.debugMode) {
      // mouse drop height debug
      const { pixelOffset } = this.playlistItem
      const { left, width } = waveform.boundingBox()
      this.p.strokeWeight(2)
      this.p.stroke(255,187,153)
      this.p.line(left, pixelOffset.yOffset, left + width, pixelOffset.yOffset)

    }
  }
}

export default PlaylistObject
