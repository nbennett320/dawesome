const base = require('./engine/themes/base.json')

const kebab = s => s.replace(/[A-Z]/g, c => `-${c.toLowerCase()}`)

// Tailwind 3.0 has no <alpha-value>: a function color receives the opacity
const token =
  name =>
  ({ opacityValue }) =>
    opacityValue === undefined ? `rgb(var(--c-${name}))` : `rgb(var(--c-${name}) / ${opacityValue})`

// every core theme color, as a utility color: bg-surface, text-ink-soft, ...
const colors = Object.fromEntries(Object.keys(base.colors).map(k => [kebab(k), token(kebab(k))]))

module.exports = {
  content: ['./index.html', './src/**/!(*.stories|*.spec|*.test).{js,ts,jsx,tsx}'],
  theme: {
    extend: { colors },
  },
  plugins: [],
}
