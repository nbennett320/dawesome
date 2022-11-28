/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
} from 'react-p5-wrapper'
import { invoke } from '@tauri-apps/api'
import Timeline from './Timeline'
import Cursor from './Cursor'
import PlaylistObject from './PlaylistObject'
import { PlaylistItem, PlaylistWindow } from '../../../types/playlist'

export interface CanvasProps extends SketchProps {
  height: number
  width: number
  maxPlaylistBeats: number
  playlistObjects: PlaylistItem[]
  onItemDrop: (pw: PlaylistWindow) => void
}

export const staticDefaults = {
  zoomSensitivity: .035,
  mouseDragDetectionThreshold: 10,
  timelineHeight: 24,
}

const fetchWaveformData = async (id: number) => {
  const wf = await invoke<number[]>('get_node_data', { id })

  return wf
}

const sketch = (p: P5CanvasInstance<CanvasProps>) => {
  let canvas: p5.Renderer
  let height: number
  let width: number
  let maxPlaylistBeats: number

  let currentScale = 1
  let transformX = 0
  let transformY = 0
  
  let isMouseDragged = false
  let mousePressedX: number | null = null
  let mousePressedY: number | null = null

  let playlistObjects: Array<{
    item: PlaylistItem
    p5PlaylistObject: PlaylistObject
  }> = []

  p.setup = () => {
    canvas = p.createCanvas(width, height)
    p.noStroke()

    canvas.mousePressed(() => {
      mousePressedX = p.mouseX
      mousePressedY = p.mouseY

      isMouseDragged = true
      p.mouseReleased = () => {
        isMouseDragged = false
      }
    })

    // handle zoom event
    canvas.mouseWheel((ev: WheelEvent) => {
      let scaleFactor = null
      if(ev?.deltaY < 0) {
        scaleFactor = 1 + staticDefaults.zoomSensitivity
      } else if(ev?.deltaY > 0) {
        scaleFactor = 1 - staticDefaults.zoomSensitivity
      } else {
        scaleFactor = 1
      }

      if(currentScale * scaleFactor < 1) return

      currentScale *= scaleFactor
      transformX = p.mouseX - (p.mouseX * scaleFactor) + (transformX * scaleFactor)
      transformY = p.mouseY - (p.mouseY * scaleFactor) + (transformY * scaleFactor)
      console.log("currentScale: ", currentScale)
    })

    // handle click release
    canvas.mouseReleased(() => {
      mousePressedX = null
      mousePressedY = null
      isMouseDragged = false
    })

    // handle mouse drag
    canvas.mouseMoved((ev: MouseEvent) => {
      if(isMouseDragged) {
        const dist = p.dist(mousePressedX ?? 0, mousePressedY ?? 0, p.mouseX, p.mouseY)
        if(dist > staticDefaults.mouseDragDetectionThreshold) {
          isMouseDragged = true
          transformX -= (p.mouseX - ev.offsetX)
          transformY -= (p.mouseY - ev.offsetY)
        }
      }
    })

    canvas.drop((dropped) => {
      console.log("dropped this: ", dropped)
    })
  }

  // handle canvas recieved props
  p.updateWithProps = props => {
    height = props.height
    width = props.width
    maxPlaylistBeats = props.maxPlaylistBeats
    p.resizeCanvas(width, height)

    const newPlaylistObjects: Array<{
      item: PlaylistItem
      p5PlaylistObject: PlaylistObject
    }> = []

    props.playlistObjects.forEach(async (item) => {
      const soundData = await fetchWaveformData(item.id)
      const p5PlaylistObject = new PlaylistObject(
        p,
        canvas,
        {
          currentScale,
          soundData,
          timelineWidth: width, 
          timelineHeight: staticDefaults.timelineHeight,
          playlistItem: item,
        }
      )

      newPlaylistObjects.push({
        item,
        p5PlaylistObject,
      } as {
        item: PlaylistItem
        p5PlaylistObject: PlaylistObject
      })
    })

    playlistObjects = newPlaylistObjects

    console.log("objects to be rendered: ", playlistObjects)
  }

  // render p5 canvas
  p.draw = () => {
    const timeline = new Timeline(
      p,
      canvas,
      {
        timelineWidth: width, 
        timelineHeight: staticDefaults.timelineHeight,
        currentScale,
      }
    )

    const cursor = new Cursor(
      p,
      canvas,
      {
        timelineWidth: width, 
        timelineHeight: staticDefaults.timelineHeight,
        currentScale,
      }
    )
    
    p.background(255, 255, 255)
    p.stroke(180, 180, 180)
    p.fill(255, 255, 255)

    transformX = p.constrain(transformX, width * (1 - currentScale), 0)
    transformY = p.constrain(transformY, height * (1 - currentScale), 0)

    p.push()
    p.translate(transformX, 1)
    p.scale(currentScale, 1)
    
    timeline.render()
    cursor.render()

    playlistObjects.forEach(item => {
      item.p5PlaylistObject.render()
    })

    for(let i = 0; i < width; i += width/maxPlaylistBeats) {
      for(let j = staticDefaults.timelineHeight; j < height+staticDefaults.timelineHeight; j += height/5) {
        p.stroke(0, 0, 0)
        p.strokeWeight(.3)
        p.line(i, 0, i, height)
        p.line(0, j, width, j)
      }
    }

    p.pop()
  }
}

export default sketch