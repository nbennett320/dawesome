// the Flanger's own panel: its comb view (draw.ts), live with the sweep,
// and below it the Sweep, Comb, Stereo, Mix and Experimental controls.
// drag on the view: across for Manual (where the comb sits), up and down
// for the feedback. shift: fine
import React from 'react'
import { painter, useThemeChange } from '../../../theme/canvas'
import { ChoiceKeys, Control, ExperimentalGroup, Group, useXYDrag } from '../modulation/ModControls'
import { liveAt, paramOf } from '../modulation/motion'
import { useLiveCanvas } from '../modulation/useLiveCanvas'
import type { EffectParamsApi } from '../useEffectParams'
import { captionOf, crossingOf, settingsOf, tapsAt } from './comb'
import { Curve, curveOf, drawFlanger, TRAIL } from './draw'

const FlangerPanel = ({ api, slotId, active }: { api: EffectParamsApi, slotId: number, active: boolean }) => {
  const theme = useThemeChange()
  const { params } = api
  const settings = settingsOf(params)
  const p = (key: string) => paramOf(params, key)
  const noteLabel = p('note')?.labels[Math.round(p('note')?.value.value ?? 0)] ?? ''
  const [hot, setHot] = React.useState(false)
  const trail = React.useRef<Curve[]>([])

  const { box, canvas, area } = useLiveCanvas(slotId, active, (ctx, size, reading, now) => {
    if(!settings) return
    const live = liveAt(reading, now)
    const { center, sides } = tapsAt(settings, live)
    const left = curveOf(sides[0], settings, center, size)
    const right = settings.spread > 0 ? curveOf(sides[1], settings, center, size) : null
    drawFlanger(ctx, {
      area: size,
      settings,
      left,
      right,
      trail: trail.current,
      centerMs: center,
      crossing: crossingOf(sides[0].delayMs, center),
      caption: captionOf(settings, live, noteLabel, center),
      hot,
    })
    // (a trail only while it moves)
    trail.current = live?.running ? [...trail.current, left].slice(-TRAIL) : []
  }, [params, theme, hot])

  const drag = useXYDrag(api, p('manual'), p('feedback'), area)
  if(!settings) return null

  return (
    <div className='h-full flex flex-col min-h-0'>
      <div ref={box} className='relative flex-1 min-h-[140px] m-2 mb-0 rounded overflow-hidden border border-line'>
        <canvas
          ref={canvas}
          aria-label='Flanger comb. Drag across for where it sits (Manual), up and down for the feedback'
          className='absolute inset-0 w-full h-full outline-none'
          style={{ cursor: drag.dragging ? 'grabbing' : 'grab', backgroundColor: painter('effect').css('background') }}
          {...drag.handlers}
          onPointerEnter={() => setHot(true)}
          onPointerLeave={() => setHot(false)}
        />
      </div>

      <div className='shrink-0 flex flex-row flex-wrap gap-2 p-2'>
        <Group name='Sweep'>
          <Control param={p('sync')} api={api} />
          <Control param={settings.sync ? p('note') : p('rate')} api={api} />
          <Control param={p('depth')} api={api} />
          <ChoiceKeys param={p('shape')} api={api} />
        </Group>
        <Group name='Comb'>
          <Control param={p('manual')} api={api} />
          <Control param={p('feedback')} api={api} />
          <Control param={p('tone')} api={api} />
          <Control param={p('zero')} api={api} />
        </Group>
        <Group name='Stereo'>
          <Control param={p('spread')} api={api} />
        </Group>
        <Group name='Mix'>
          <Control param={p('mix')} api={api} />
        </Group>
        <ExperimentalGroup params={[p('barberpole'), p('tune'), p('envelope')]} api={api} />
      </div>
    </div>
  )
}

export default FlangerPanel
