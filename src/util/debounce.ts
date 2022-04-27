export const debounce = (
  f: () => void, 
  timeout: number
) => {
  let timer: NodeJS.Timeout

  return (...args: []) => {
    clearTimeout(timer)
    timer = setTimeout(() => { 
      f.apply(this, args)
    }, timeout)
  }
}
