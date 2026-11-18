// the Phaser's own panel: its swirl view (draw.ts), live with the sweep,
// and below it the Sweep, Notches, Stereo, Mix and Experimental controls.
// drag on the view: across for the centre, up and down for the feedback.
// shift: fine
import React from 'react'
import { painter, useThemeChange } from '../../../theme/canvas'
import { ChoiceKeys, Control, ExperimentalGroup, Group, useXYDrag } from '../modulation/ModControls'
import { lamp, liveAt, paramOf } from '../modulation/motion'
import { useLiveCanvas } from '../modulation/useLiveCanvas'
import type { EffectParamsApi } from '../useEffectParams'
import { drawPhaser, Ring, ringOf, TRAIL } from './draw'
import { captionOf, settingsOf, sidesAt, SLOT } from './notches'

const PhaserPanel = ({ api, slotId, active }: { api: EffectParamsApi, slotId: number, active: boolean }) => {
  const theme = useThemeChange()
  const { params } = api
  const settings = settingsOf(params)
  const p = (key: string) => paramOf(params, key)
  const noteLabel = p('note')?.labels[Math.round(p('note')?.value.value ?? 0)] ?? ''
  const [hot, setHot] = React.useState(false)
  const trail = React.useRef<Ring[]>([])

  const { box, canvas, area } = useLiveCanvas(slotId, active, (ctx, size, reading, now) => {
    if(!settings) return
    const live = liveAt(reading, now)
    const rate = reading?.readout.rate || 48000
    const [l, r] = sidesAt(settings, live, rate)
    const left = ringOf(l, settings, rate, size)
    const vibe = live ? live.slots[SLOT.VIBE] ?? 0 : settings.vibe ? 1 : 0
    drawPhaser(ctx, {
      area: size,
      settings,
      left,
      right: settings.spread > 0 ? ringOf(r, settings, rate, size) : null,
      trail: trail.current,
      phase: live?.phase ?? 0,
      glow: vibe * (0.5 + 0.5 * lamp(live?.phase ?? 0)),
      caption: captionOf(settings, live, noteLabel),
      hot,
    })
    trail.current = live?.running ? [...trail.current, left].slice(-TRAIL) : []
  }, [params, theme, hot])

  const drag = useXYDrag(api, p('center'), p('feedback'), area)
  if(!settings) return null

  return (
    <div className='h-full flex flex-col min-h-0'>
      <div ref={box} className='relative flex-1 min-h-[160px] m-2 mb-0 rounded overflow-hidden border border-line'>
        <canvas
          ref={canvas}
          aria-label='Phaser notches, round a circle. Drag across for the centre, up and down for the feedback'
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
          <ChoiceKeys param={p('shape')} api={api} dim={settings.vibe} />
        </Group>
        <Group name='Notches'>
          <Control param={p('center')} api={api} />
          <ChoiceKeys param={p('stages')} api={api} dim={settings.vibe} />
          <Control param={p('feedback')} api={api} />
          <ChoiceKeys param={p('mode')} api={api} />
        </Group>
        <Group name='Stereo'>
          <Control param={p('spread')} api={api} />
        </Group>
        <Group name='Mix'>
          <Control param={p('mix')} api={api} />
        </Group>
        <ExperimentalGroup params={[p('melt'), p('barberpole'), p('envelope')]} api={api} />
      </div>
    </div>
  )
}

export default PhaserPanel
