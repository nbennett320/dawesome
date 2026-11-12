// small helpers the whole app shares

// keeps a number within a range
export const clamp = (n: number, min: number, max: number) => Math.min(max, Math.max(min, n))

// like clamp, but if the range is backwards (max below min) the low end wins
export const clampLowWins = (n: number, low: number, high: number) => clamp(n, low, Math.max(low, high))

// joins the class names that are set, leaving out the ones that aren't
export const cx = (...c: Array<string | false | null | undefined>) => c.filter(Boolean).join(' ')

// the last part of a path (a trailing slash doesn't count)
export const fileName = (path: string): string => path.split(/[\\/]/).filter(Boolean).pop() ?? path

// "1 clip", "2 clips"
export const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? '' : 's'}`
