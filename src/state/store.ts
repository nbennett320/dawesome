import { configureStore } from '@reduxjs/toolkit'
import playPausePlaylistReducer from './slices/playPausePlaylistSlice'
import playlistTempoReducer from './slices/playlistTempoSlice'
import playlistMetronomeEnabledReducer from './slices/playlistMetronomeEnabledSlice'
import playlistRuntimeReducer from './slices/playlistRuntimeSlice'

export const store = configureStore({
  reducer: {
    playlistPlaying: playPausePlaylistReducer,
    playlistTempo: playlistTempoReducer,
    playlistMetronomeEnabled: playlistMetronomeEnabledReducer,
    playlistRuntime: playlistRuntimeReducer
  },
})

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>

// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch
