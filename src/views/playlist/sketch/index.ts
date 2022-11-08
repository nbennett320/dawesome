/* eslint-disable @typescript-eslint/no-unused-vars */
import p5 from 'p5'
import React from 'react'
import {
  P5CanvasInstance,
  SketchProps,
} from 'react-p5-wrapper'
import Timeline from './Timeline'
import Cursor from './Cursor'

export interface CanvasProps extends SketchProps {
  height: number
  width: number
  maxPlaylistBeats: number
}

export const staticDefaults = {
  zoomSensitivity: 0.025,
  mouseDragDetectionThreshold: 10,
  timelineHeight: 24,
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

  const renderedObjects = []

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
    })


    canvas.mouseReleased(() => {
      mousePressedX = null
      mousePressedY = null
      isMouseDragged = false
    })

    canvas.mouseReleased(() => {
      isMouseDragged = false
    })
    
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
  }

  p.updateWithProps = props => {
    height = props.height
    width = props.width
    maxPlaylistBeats = props.maxPlaylistBeats
    p.resizeCanvas(width, height)
  }

  p.draw = () => {
    const timeline = new Timeline(
      p,
      canvas,
      width,
      staticDefaults.timelineHeight,
    )

    const cursor = new Cursor(
      p,
      canvas,
      width,
      staticDefaults.timelineHeight,
    )
    
    p.background(255, 255, 255)
    p.stroke(180, 180, 180)
    p.fill(255, 255, 255)

    transformX = p.constrain(transformX, -1 * width, 0)
    transformY = p.constrain(transformY, -1 * height, 0)

    p.push()
    p.translate(transformX, transformY)
    p.scale(currentScale)
    
    cursor.render()
    timeline.render()

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
