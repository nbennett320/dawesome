import React from 'react'
import { clamp, cx } from '../../util/helpers'

// the cassette is drawn in a 320 x 200 box (a real one is 100 x 63mm)
const WIDTH = 320
const HEIGHT = 200
// with the tape path drawn below it
const HEIGHT_WITH_TAPE_PATH = 272

// width over height, for fitting the cassette into a box
export const cassetteAspect = (tapePath = false) =>
  WIDTH / (tapePath ? HEIGHT_WITH_TAPE_PATH : HEIGHT)

// reel centres, seen through the window
const REEL_Y = 92
const LEFT_X = 112
const RIGHT_X = 208

// a tape pack's radius when its reel is empty (the hub) and when it holds
// all the tape
export const HUB_RADIUS = 15
export const FULL_RADIUS = 40

// the tape packs' radii, given how far through the tape is (0: all on the
// left reel, 1: all on the right). the tape's length is fixed, so the area
// of the two packs together is too
export const packRadii = (progress: number): { left: number, right: number } => {
  const p = clamp(Number.isFinite(progress) ? progress : 0, 0, 1)
  const empty = HUB_RADIUS ** 2
  const tape = FULL_RADIUS ** 2 - empty

  return {
    left: Math.sqrt(empty + (1 - p) * tape),
    right: Math.sqrt(empty + p * tape),
  }
}

// how fast the tape runs at play speed, in drawing units a second. it turns
// an empty hub about once a second, like a real deck
export const PLAY_TAPE_SPEED = 2 * Math.PI * HUB_RADIUS * 1.2

// the fastest a reel is drawn turning, in degrees a second. the hub has six
// teeth (60 degrees apart), so at 60 frames a second anything over 30
// degrees a frame looks like it's turning backwards
export const MAX_SPIN = 1260

// how fast a reel of this radius turns, in degrees a second, for a tape
// speed given in multiples of play speed (negative winds back). turning
// clockwise is positive, and forward tape turns both reels anticlockwise
export const reelSpin = (radius: number, speed: number): number => {
  if(!radius || !speed) return 0

  const degrees = (PLAY_TAPE_SPEED * Math.abs(speed) / radius) * (180 / Math.PI)
  return -Math.sign(speed) * Math.min(degrees, MAX_SPIN)
}

// how quickly the reels get up to speed (and slow down), in seconds
const SPIN_UP = 0.18

// how fast the marks on the tape path move at play speed, in drawing units a
// second, and the fastest they're drawn moving. the marks repeat every
// TAPE_MARK_PERIOD units, so moving more than half that a frame (at 60
// frames a second) would look like going backwards
export const TAPE_MARK_SPEED = 60
export const TAPE_MARK_PERIOD = 12
export const MAX_TAPE_MARK_SPEED = 330

// how fast the tape path's marks move, in drawing units a second (positive
// is forward: from the left reel to the right)
export const tapeMarkSpeed = (speed: number): number =>
  Math.sign(speed) * Math.min(Math.abs(speed) * TAPE_MARK_SPEED, MAX_TAPE_MARK_SPEED)

// the tape pulled out of the cassette's opening: down from the pins, round a
// guide roller, across the heads and between the capstan and pinch roller,
// round the other guide roller and back up
const TAPE_Y = 238
// (past the outside of the pins, so it wraps them)
const TAPE_PATH = `M81 176 V230 A8 8 0 0 0 89 ${TAPE_Y} H231 A8 8 0 0 0 239 230 V176`
// the stretch of tape over the play head (for grabbing it, later)
const HEAD_X = 160
const TAPE_SPAN = `M${HEAD_X - 22} ${TAPE_Y} H${HEAD_X + 22}`

// the deck's mechanism under the cassette: the tape path, its rollers, the
// heads, and the capstan. `marks` is the dashed tape that moves
const TapePath = ({ marks }: { marks: React.Ref<SVGPathElement> }) => (
  <g className='ft-cassette__deck'>
    <rect className='ft-cassette__plate' x={40} y={204} width={240} height={62} rx={8} />

    {/* the heads, under the tape: erase, then play/record */}
    <rect className='ft-cassette__head ft-cassette__head--erase' x={112} y={TAPE_Y} width={14} height={14} rx={2} />
    <rect className='ft-cassette__head' x={HEAD_X - 16} y={TAPE_Y} width={32} height={20} rx={4} />
    <line className='ft-cassette__head-gap' x1={HEAD_X} y1={TAPE_Y + 1} x2={HEAD_X} y2={TAPE_Y + 6} />

    <circle className='ft-cassette__roller' cx={89} cy={230} r={6.5} />
    <circle className='ft-cassette__roller' cx={231} cy={230} r={6.5} />
    <circle className='ft-cassette__pinch' cx={202} cy={TAPE_Y + 7.5} r={6.5} />

    <path className='ft-cassette__tape-path' d={TAPE_PATH} />
    <path
      ref={marks}
      className='ft-cassette__tape-marks'
      d={TAPE_PATH}
      strokeDasharray={`2 ${TAPE_MARK_PERIOD - 2}`}
    />
    <path className='ft-cassette__tape-span' d={TAPE_SPAN} />

    <circle className='ft-cassette__capstan' cx={202} cy={TAPE_Y - 3.5} r={2.5} />
  </g>
)

export interface CassetteProps {
  // how far through the tape is, 0..1
  progress: number
  // how fast the tape is running, in multiples of play speed: 0 is stopped,
  // 1 playing, more for fast forward, negative for rewind
  speed?: number
  // written on the label
  label?: string
  // the side, in the label's corner
  side?: string
  className?: string
  // draw the tape pulled out across the deck's heads, under the cassette
  tapePath?: boolean
  // lets the label be edited: double-click the cassette (or press Enter on
  // it), type, and Enter or clicking away gives the new name. Escape cancels
  onRename?: (name: string) => void
}

// for each cassette's window clip path
let nextId = 0

// six teeth on each hub, so they can be seen turning
const TEETH = [0, 60, 120, 180, 240, 300]

const Reel = ({ x, radius, hubRef }: { x: number, radius: number, hubRef: React.Ref<SVGGElement> }) => (
  <g transform={`translate(${x} ${REEL_Y})`}>
    <circle className='ft-cassette__pack' r={radius} />
    {/* the edge of the wound tape */}
    <circle className='ft-cassette__pack-edge' r={Math.max(radius - 2.5, HUB_RADIUS)} />
    <g ref={hubRef}>
      <circle className='ft-cassette__hub' r={11} />
      <circle className='ft-cassette__bore' r={6.5} />
      {TEETH.map(angle => (
        <rect
          key={angle}
          className='ft-cassette__tooth'
          x={-1.25}
          y={-7}
          width={2.5}
          height={3}
          rx={0.75}
          transform={`rotate(${angle})`}
        />
      ))}
    </g>
  </g>
)

// a cassette seen from the front: the tape moves from the left reel to the
// right as `progress` goes from 0 to 1, and the reels turn with `speed`
export const Cassette = ({
  progress,
  speed = 0,
  label = 'dawesome',
  side = 'A',
  className,
  tapePath = false,
  onRename,
}: CassetteProps) => {
  const radii = packRadii(progress)
  const leftHub = React.useRef<SVGGElement>(null)
  const rightHub = React.useRef<SVGGElement>(null)

  // read by the animation frame, which outlives any one render
  const marks = React.useRef<SVGPathElement>(null)
  const live = React.useRef({ radii, target: speed, speed: 0, left: 0, right: 0, tape: 0 })
  live.current.radii = radii
  live.current.target = speed

  // the running animation frame, while the reels are turning
  const anim = React.useRef<number | null>(null)

  React.useEffect(() => {
    // turning reels are decoration, so they stay still for anyone who's
    // asked for less motion (the packs still show where the tape is)
    const still = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
    const reels = live.current
    if(still || anim.current !== null || (!speed && !reels.speed)) return

    let last: number | null = null

    const step = (now: number) => {
      const dt = last === null ? 0 : Math.min((now - last) / 1000, 0.1)
      last = now

      // ease towards the speed asked for, like a motor spinning up
      reels.speed += (reels.target - reels.speed) * (1 - Math.exp(-dt / SPIN_UP))
      if(!reels.target && Math.abs(reels.speed) < 0.01) reels.speed = 0

      reels.left = (reels.left + reelSpin(reels.radii.left, reels.speed) * dt) % 360
      reels.right = (reels.right + reelSpin(reels.radii.right, reels.speed) * dt) % 360
      leftHub.current?.setAttribute('transform', `rotate(${reels.left})`)
      rightHub.current?.setAttribute('transform', `rotate(${reels.right})`)

      // the marks move along the path the other way to the dash offset
      reels.tape = (reels.tape - tapeMarkSpeed(reels.speed) * dt) % TAPE_MARK_PERIOD
      marks.current?.setAttribute('stroke-dashoffset', String(reels.tape))

      // carries on until the reels have come to rest
      anim.current = reels.speed || reels.target ? requestAnimationFrame(step) : null
    }

    anim.current = requestAnimationFrame(step)
  }, [speed])

  React.useEffect(() => () => {
    if(anim.current !== null) cancelAnimationFrame(anim.current)
  }, [])

  // more than one cassette can be on screen, each needs its own window
  const clipId = React.useRef(`ft-cassette-window-${nextId++}`).current

  const percent = Math.round(clamp(progress, 0, 1) * 100)

  // the label being typed, while editing it
  const [draft, setDraft] = React.useState<string | null>(null)
  const startEditing = () => { if(onRename && draft === null) setDraft(label) }
  const finishEditing = () => {
    if(draft === null) return
    setDraft(null)
    if(draft.trim() && draft.trim() !== label) onRename?.(draft.trim())
  }

  const description = `Tape, ${percent}% through`

  return (
    <svg
      viewBox={`0 0 ${WIDTH} ${tapePath ? HEIGHT_WITH_TAPE_PATH : HEIGHT}`}
      className={cx('ft-cassette', onRename && 'ft-cassette--renamable', className)}
      {...(onRename
        ? {
          role: 'button',
          tabIndex: 0,
          'aria-label': `${description}. Press Enter to name the project`,
          onDoubleClick: startEditing,
          onKeyDown: (e: React.KeyboardEvent) => {
            if(e.key === 'Enter' && e.target === e.currentTarget) startEditing()
          },
        }
        : { role: 'img', 'aria-label': description })}
    >
      {onRename && <title>Double-click to name the project</title>}
      <defs>
        <clipPath id={clipId}>
          <rect x={56} y={50} width={208} height={84} rx={12} />
        </clipPath>
      </defs>

      <rect className='ft-cassette__shell' x={4} y={4} width={312} height={192} rx={10} />
      {[[16, 16], [304, 16], [16, 184], [304, 184]].map(([x, y]) => (
        <circle key={`${x},${y}`} className='ft-cassette__screw' cx={x} cy={y} r={3.5} />
      ))}

      {/* the label, with the window cut out of it */}
      <rect className='ft-cassette__label' x={24} y={16} width={272} height={136} rx={6} />
      {draft === null
        ? <text className='ft-cassette__title' x={38} y={36}>{label}</text>
        : (
          <foreignObject x={34} y={22} width={224} height={19}>
            <input
              className='ft-cassette__title-input'
              aria-label='Project name'
              value={draft}
              // eslint-disable-next-line jsx-a11y/no-autofocus
              autoFocus
              onFocus={e => e.currentTarget.select()}
              onChange={e => setDraft(e.currentTarget.value)}
              onBlur={finishEditing}
              onKeyDown={e => {
                e.stopPropagation()
                if(e.key === 'Enter') e.currentTarget.blur()
                if(e.key === 'Escape') setDraft(null)
              }}
              onDoubleClick={e => e.stopPropagation()}
            />
          </foreignObject>
        )}
      <line className='ft-cassette__rule' x1={38} y1={42} x2={258} y2={42} />
      <rect className='ft-cassette__side' x={266} y={25} width={18} height={18} rx={3} />
      <text className='ft-cassette__side-text' x={275} y={38} textAnchor='middle'>{side}</text>
      <rect className='ft-cassette__stripe ft-cassette__stripe--1' x={24} y={140} width={272} height={3} />
      <rect className='ft-cassette__stripe ft-cassette__stripe--2' x={24} y={144} width={272} height={2} />

      <rect className='ft-cassette__window' x={56} y={50} width={208} height={84} rx={12} />
      <g clipPath={`url(#${clipId})`}>
        <Reel x={LEFT_X} radius={radii.left} hubRef={leftHub} />
        <Reel x={RIGHT_X} radius={radii.right} hubRef={rightHub} />
      </g>

      {/* the opening at the bottom, where the tape passes the heads */}
      <path className='ft-cassette__opening' d='M64 196 L80 164 H240 L256 196' />
      {/* the tape across the opening, unless it's pulled out over the heads */}
      {!tapePath && <line className='ft-cassette__tape' x1={78} y1={176} x2={242} y2={176} />}
      <circle className='ft-cassette__pin' cx={84} cy={176} r={3} />
      <circle className='ft-cassette__pin' cx={236} cy={176} r={3} />

      {tapePath && <TapePath marks={marks} />}
    </svg>
  )
}
