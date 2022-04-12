import { configureStore } from '@reduxjs/toolkit'
import playlistReducer from './slices/playlistSlice'
import windowReducer from './slices/windowSlice'

export const store = configureStore({
  reducer: {
    playlist: playlistReducer,
    window: windowReducer,
  },
})

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>

// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch
