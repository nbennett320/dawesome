import React from 'react'
import styles from './styles.module.scss'

interface Props {
  path: string,
  viewBox: string,
}

const PlaylistItemWaveform = (props: Props) => (
  <div className={styles.PlaylistItemWaveform}>
    <svg 
      viewBox={props.viewBox} 
      style={{
        height: '80px',
        maxHeight: '100px',
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
