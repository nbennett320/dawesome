import { configureStore } from '@reduxjs/toolkit'
import playlistReducer from './slices/playlistSlice'
import windowReducer from './slices/windowSlice'
import envReducer from './slices/envSlice'
import projectReducer from './slices/projectSlice'
import mixerReducer from './slices/mixerSlice'
import sequencerReducer from './slices/sequencerSlice'

export const store = configureStore({
  reducer: {
    playlist: playlistReducer,
    window: windowReducer,
    env: envReducer,
    project: projectReducer,
    mixer: mixerReducer,
    sequencer: sequencerReducer,
  },
})

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>

// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch
