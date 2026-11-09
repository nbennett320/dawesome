import React from 'react'
import TransportDeck from './TransportDeck'

// the Transport tab: the tape deck, centred in whatever room the pane has
const Transport = () => (
  <div className='ft-panel h-full w-full overflow-auto flex flex-col items-center justify-center p-6'>
    <TransportDeck />
  </div>
)

export default Transport
