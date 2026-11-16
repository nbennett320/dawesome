// built-in effects' parameter changes on their way to the engine. undo
// waits for them (see history.ts): a knob turned a moment ago has to have
// reached the engine to be undone, and mustn't land after the undo
const inFlight = new Set<Promise<unknown>>()

// keep track of a send until it's answered
export const trackParamSend = <T>(send: Promise<T>): Promise<T> => {
  inFlight.add(send)
  const done = () => { inFlight.delete(send) }
  send.then(done, done)
  return send
}

// every send still going, answered (failures included)
export const flushEffectParamSends = (): Promise<unknown> =>
  Promise.all(Array.from(inFlight, send => send.catch(() => undefined)))
