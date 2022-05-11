import React from 'react'
import styles from './styles.module.scss'

interface Props {
  path: string
}

const PlaylistItemWaveform = (props: Props) => (
  <div className={styles.PlaylistItemWaveform}>
    <svg 
      viewBox="0 -28068 7490 56136" 
      style={{
        height: 'max-content',
      }}
      xmlns="http://www.w3.org/2000/svg"
    >
      <path 
        d={props.path} 
        fill="black" 
        stroke="black" 
        strokeWidth="0.05%"
      />
    </svg>
  </div>
)

export default PlaylistItemWaveform
