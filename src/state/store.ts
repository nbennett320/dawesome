import { configureStore } from '@reduxjs/toolkit'
import playlistReducer from './slices/playlistSlice'
import windowReducer from './slices/windowSlice'
import envReducer from './slices/envSlice'
import projectReducer from './slices/projectSlice'
import mixerReducer from './slices/mixerSlice'
import sequencerReducer from './slices/sequencerSlice'
import recordingReducer from './slices/recordingSlice'
import mixdownReducer from './slices/mixdownSlice'
import appearanceReducer from './slices/appearanceSlice'
import effectsReducer from './slices/effectsSlice'

export const store = configureStore({
  reducer: {
    playlist: playlistReducer,
    window: windowReducer,
    env: envReducer,
    project: projectReducer,
    mixer: mixerReducer,
    sequencer: sequencerReducer,
    recording: recordingReducer,
    mixdown: mixdownReducer,
    appearance: appearanceReducer,
    effects: effectsReducer,
  },
})

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>

// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch
