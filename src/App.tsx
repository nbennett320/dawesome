/* eslint-disable jsx-a11y/label-has-associated-control */
import React from 'react';
import { invoke } from '@tauri-apps/api';
import { useInvoke } from './hooks/swr';

function App() {
  // const [tempo, setTempo] = React.useState(() => invoke('get_playlist_tempo').then(x => x))
  const [tempo, setTempo] = React.useState(120);
  const [runtime, setRuntime] = React.useState<number>();
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const timer = React.useRef<any>(null);
  const { data: playing, update: togglePlay } = useInvoke({}, 'get_playlist_playing', 'toggle_playlist');

  // const { data: tempo, update: setTempo } = useInvoke(
  //   { val: 120 },
  //   'get_playlist_tempo',
  //   'set_playlist_tempo'
  // )

  React.useEffect(() => {
    if (playing) {
      timer.current = setInterval(() => {
        invoke('get_playlist_start_time').then((data) => {
          const res = Date.now().valueOf() - (data as number) * 1000;
          setRuntime(res);
        });
      }, 1000);
    } else {
      clearInterval(timer.current);
    }
  }, [playing]);

  React.useEffect(() => {
    if (playing) togglePlay({});
    invoke('set_playlist_tempo', { val: tempo as number });
  }, [tempo]);

  return (
    <div className="min-h-screen text-neutral-200">
      <nav className="w-full">
        <ul className="flex flex-row space-x-3 place-items-center">
          <li aria-label="transport control">
            <button
              type="button"
              aria-label="Play/Pause"
              onClick={togglePlay}
              className="p-1.5 bg-neutral-800 border border-neutral-700/50 rounded-md hover:bg-neutral-700"
            >
              {playing ? <>Pause</> : <>Play</>}
            </button>
          </li>
          <li>
            <div className="flex flex-row space-x-1.5 place-items-center">
              <label htmlFor="tempo-control" className="text-sm text-neutral-400">
                Tempo:
              </label>
              <input
                id="tempo-control"
                name="tempo"
                type="number"
                value={tempo as number}
                onInput={(e) => {
                  setTempo(parseFloat(e.currentTarget.value) as number);
                }}
                className="border border-neutral-700/50 bg-neutral-800 px-1 py-0.5 rounded-md w-14 focus:bg-neutral-700"
              />
            </div>
          </li>
        </ul>
      </nav>

      {playing && (
        <div
          style={{ display: 'flex', flexDirection: 'column', width: '200px', marginLeft: 'auto', marginRight: 'auto' }}
        >
          <span>Playlist runtime:</span>{' '}
          <span style={{ display: 'flex', flexDirection: 'row' }}>
            <span style={{ width: '100px', left: '0' }}>{runtime}</span> <span style={{ left: '100%' }}>ms</span>
          </span>
        </div>
      )}
    </div>
  );
}

export default App;
