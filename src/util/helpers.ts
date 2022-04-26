// todo: move this to rust
export const hexGen = (size: number) => [...Array(size)].map(() => Math.floor(Math.random() * 16).toString(16)).join('')
