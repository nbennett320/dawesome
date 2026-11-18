// the Chorus's own panel: its "ghost strings" view (draw.ts), live with the
// LFO, and below it the Motion, Voices, Tone, Mix and Experimental
// controls. drag on the view: across for the rate (the note value when
// synced), up and down for the depth. shift: fine
import React from 'react'
import { painter, useThemeChange } from '../../../theme/canvas'
import { ChoiceKeys, Control, ExperimentalGroup, Group, useXYDrag } from '../modulation/ModControls'
import { liveAt, paramOf } from '../modulation/motion'
import { useLiveCanvas } from '../modulation/useLiveCanvas'
import type { EffectParamsApi } from '../useEffectParams'
import { drawChorus } from './draw'
import { captionOf, settingsOf, SLOT, voicesAt } from './voices'

const ChorusPanel = ({ api, slotId, active }: { api: EffectParamsApi, slotId: number, active: boolean }) => {
  const theme = useThemeChange()
  const { params } = api
  const settings = settingsOf(params)
  const p = (key: string) => paramOf(params, key)
  const noteLabel = p('note')?.labels[Math.round(p('note')?.value.value ?? 0)] ?? ''
  const [hot, setHot] = React.useState(false)

  const { box, canvas, area } = useLiveCanvas(slotId, active, (ctx, size, reading, now) => {
    if(!settings) return
    const live = liveAt(reading, now)
    drawChorus(ctx, {
      area: size,
      settings,
      voices: voicesAt(settings, live),
      phase: live?.phase ?? 0,
      open: settings.bloom > 0 ? live?.slots[SLOT.OPEN] ?? 1 : 1,
      caption: captionOf(settings, live, noteLabel),
      hot,
    })
  }, [params, theme, hot])

  const drag = useXYDrag(api, settings?.sync ? p('note') : p('rate'), p('depth'), area)
  if(!settings) return null

  return (
    <div className='h-full flex flex-col min-h-0'>
      <div ref={box} className='relative flex-1 min-h-[140px] m-2 mb-0 rounded overflow-hidden border border-line'>
        <canvas
          ref={canvas}
          aria-label='Chorus voices. Drag across for the rate, up and down for the depth'
          className='absolute inset-0 w-full h-full outline-none'
          style={{ cursor: drag.dragging ? 'grabbing' : 'grab', backgroundColor: painter('effect').css('background') }}
          {...drag.handlers}
          onPointerEnter={() => setHot(true)}
          onPointerLeave={() => setHot(false)}
        />
      </div>

      <div className='shrink-0 flex flex-row flex-wrap gap-2 p-2'>
        <Group name='Motion'>
          <Control param={p('sync')} api={api} />
          <Control param={settings.sync ? p('note') : p('rate')} api={api} />
          <Control param={p('depth')} api={api} />
          <ChoiceKeys param={p('shape')} api={api} />
        </Group>
        <Group name='Voices'>
          <ChoiceKeys param={p('voices')} api={api} />
          <Control param={p('delay')} api={api} />
          <Control param={p('spread')} api={api} />
        </Group>
        <Group name='Tone'>
          <Control param={p('lowcut')} api={api} />
          <Control param={p('tone')} api={api} />
        </Group>
        <Group name='Mix'>
          <Control param={p('mix')} api={api} />
        </Group>
        <ExperimentalGroup params={[p('bloom'), p('drift'), p('prism')]} api={api} />
      </div>
    </div>
  )
}

export default ChorusPanel
