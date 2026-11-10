
11-08-2026
---
- Mixer
    - Panning bar to left right dial 
    - Lows mids highs: dial (collapsible by height)
    - Effects toggle
        - When engaged a list of enabled effects pops out to the right 
- Preview
    - Gain and Panning to Dials
- Beat counter: rotary dial - odometer style
    - https://github.hubspot.com/odometer/docs/welcome/
- Transport controls: always visible or in a tab? Maybe a tiny one at the very top or something 
    - Tab: transport
        - Cassette wheels
        - Play, pause, stop, record
        - Fast forward, rewind
        - Bounce?
        - Master pitch?
- Playlist
    - Cmd + R -> Repeat (paste duplicate node after another)
    - playlist vertical line/cursor indicating across all tracks what is currently playing
- Shortcuts 
    - Shift mode -> selection mode 
        - Ex, 2 Shift + J = Join
        - Shift + drag = secect lasso
- context menu on right click?

- patterns -> pattern bank


11-09-2026
---
- Playlist
    - Adjusting the clip start and end time by clicking and dragging on the left or right edge of the clip in the UI
        - when Shift is held -> stretch that clip according to where the user drags the cursor
        - normal behavior -> adjust the start or end of the clip
    - Cut tool - split the sample into two
        - Prerequisite (probably): trimming clips from the front and back so we know where to start/end each "sub clip" or each half of the split clip
        - if there are two or more "instances" of the same clip (even if one is trimmed or something), i want the ability to make a new "deep" copy, where there is a new file saved on to the users computer
            - perhaps in the project folder, we have a new directory called clips/ where they are stored
                - ex: my_project_file/clips/*.wav
            - the deep copy bounces the sound as it currently plays back, so any modifications to the copy are baked into the file that is written to the users computer
                - ex: i have a file for which i adjusted the start and end time, and i adjusted the pitch. when i make a deep copy of this, the file written on the computer retains the adjusted pitch, start and end time.
                - this works similar to FL studio's "make unique as sample" feature
            - i think i like the term "deep copy"
            - this should be an option in the context menu
    - Copy/Paste/Cut tool
        - Cmd + C, Cmd + V, Cmd + X respectively 
        - Cmd + R -> Repeat (paste duplicate node after another)
    - enhancing lasso select:
        - right now I cant lasso select if i start clicking while my mouse is over a clip in the time line
        - I want to be able to start lassoing when over a clip.
- Shortcuts 
    - Shift mode -> selection mode 
        - Ex, 2 things are selected with lasso already, i press Shift + J = Join , they become one. works for patterns and samples
    - Space -> start or stop timeline
    
    
- Mixer
    - Panning bar to left right dial 
    - Lows mids highs: dial (collapsible by height)
    - Effects toggle
        - When engaged a list of enabled effects pops out to the right 
- Settings
    - Topbar
        - toggle to allow expanded transport controls (like in the transport tab)
- Sidebar
    - Sample browser (already added)
    - Pattern browser
        - user can drag in Patterns from the Drum Sequencer and place them into the timeline
        - patterns can be searched by name
    - Effects browser? Can drag them onto tracks?
    - Overall toggle 
        - If the current sidebar view is enabled, and the user clicks the button that would navigate to the current view: collapse the sidebar
    - Recordings browser!!!
- infra
    - what is the difference between audio_commands and bind_functions?