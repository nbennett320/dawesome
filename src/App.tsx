import React from 'react';
import { useInvoke } from './hooks/swr';

function App() {
  const counter_args = { delta: 1 };
  const play_args = { delta: 1 };

  const { data: counter, update: increment } = useInvoke(counter_args, 'get_counter', 'increment_counter');

  const { data: is_playing, update: toggle_play } = useInvoke(play_args, 'get_counter', 'toggle_play_sound');

  return (
    <div className="grid p-3 place-content-center">
      times played: {counter}
      <br />
      <button type="button" className="p-3 text-white bg-indigo-700 rounded-sm" onClick={increment}>
        Try counter
      </button>
      <br />
      is playing: {is_playing}
      <br />
      <button type="button" className="p-3 text-white rounded-sm bg-violet-700" onClick={toggle_play}>
        Play
      </button>
    </div>
  );
}

export default App;
