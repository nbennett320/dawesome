import React from 'react'
import styles from './styles.module.scss'

interface Props {
  path: string,
  viewBox: string,
  setItemWidth: React.Dispatch<React.SetStateAction<number>>,
}

const PlaylistItemWaveform = (props: Props) => {
  const ref = React.useRef<SVGSVGElement>(null)

  React.useEffect(() => {
    if(ref?.current) {
      const { width } = ref.current.getBoundingClientRect()
      props.setItemWidth(width)
    }
  }, [props.path])

  return (
    <div className={styles.PlaylistItemWaveform}>
      <svg 
        ref={ref}
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
}

export default PlaylistItemWaveform
