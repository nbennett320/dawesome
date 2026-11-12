import { defineConfig } from 'vitepress'

const repo = 'https://github.com/nbennett320/dawesome'

export default defineConfig({
  title: 'Dawesome',
  description: 'A libre, cross-platform DAW',
  base: '/dawesome/',
  cleanUrls: true,
  lastUpdated: true,

  themeConfig: {
    nav: [
      { text: 'Manual', link: '/manual/' },
      { text: 'Shortcuts', link: '/shortcuts' },
    ],

    sidebar: {
      '/manual/': [
        {
          text: 'Manual',
          items: [
            { text: '1 Before you begin', link: '/manual/' },
            { text: '2 Installing and first launch', link: '/manual/install' },
            { text: '3 Names and functions of parts', link: '/manual/parts' },
            { text: '4 Projects', link: '/manual/projects' },
            { text: '5 Basic recording', link: '/manual/recording' },
            { text: '6 Playlist', link: '/manual/playlist' },
            { text: '7 Patterns and sequencer', link: '/manual/patterns' },
            { text: '8 Piano roll', link: '/manual/piano-roll' },
            { text: '9 Instruments and effects', link: '/manual/plugins' },
            { text: '10 Mixer', link: '/manual/mixer' },
            { text: '11 Samples', link: '/manual/samples' },
            { text: '12 Mixdown', link: '/manual/mixdown' },
            { text: '13 Themes', link: '/manual/themes' },
            { text: '14 Troubleshooting', link: '/manual/troubleshooting' },
            { text: '15 Specifications', link: '/manual/specs' },
            { text: '16 Glossary', link: '/manual/glossary' },
          ],
        },
        {
          text: 'Reference',
          items: [{ text: 'Keyboard shortcuts', link: '/shortcuts' }],
        },
      ],
    },

    search: { provider: 'local' },
    outline: 'deep',
    socialLinks: [{ icon: 'github', link: repo }],
    editLink: {
      pattern: `${repo}/edit/main/docs/:path`,
      text: 'Edit this page on GitHub',
    },
  },
})
