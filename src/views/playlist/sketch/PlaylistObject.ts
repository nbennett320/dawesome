
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import { invoke } from '@tauri-apps/api'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import Waveform from './Waveform'
import { CanvasProps } from './index'
import { PlaylistItem } from '../../../types/playlist'

interface Props extends PlaylistComponentBaseProps {
  playlistItem: PlaylistItem
}

class PlaylistObject extends PlaylistComponentBase {
  playlistItem: PlaylistItem
  soundData: number[] = []

  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    props: Props,
  ) {
    super(p, canvas, props)
    this.playlistItem = props.playlistItem
  }
  
  render = () => {
    const fetchData = async () => {
      const soundData = await invoke<number[]>('get_node_data', {
        id: this.playlistItem.id
      })

      this.soundData = soundData

      if(this.soundData.length > 0) {
        console.log("found data")

        const waveform = new Waveform(
          this.p,
          this.canvas,
          {
            currentScale: this.currentScale,
            timelineHeight: this.timelineHeight,
            timelineWidth: this.timelineWidth,
            soundData: this.soundData,
          }
        )
        
        waveform.render()
      }
    }
    
    fetchData()

    
    this.p.noLoop()
  }
}

export default PlaylistObject
