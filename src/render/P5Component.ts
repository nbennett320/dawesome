import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
} from 'react-p5-wrapper'
import P5ComponentBase from './P5ComponentBase'
import RendererBase from './RendererBase'

export type P5BoundingBox = {
  minHeight: number
  maxHeight: number
  bottom: number
  top: number
  left: number
  right: number
  height: number
  width: number 
}

abstract class P5Component<T extends SketchProps> extends P5ComponentBase<T> {
  minHeight: number
  maxHeight: number
  bottom: number
  top: number
  left: number
  right: number
  height: number
  width: number 

  constructor(
    p: P5CanvasInstance<T>,
    canvas: p5.Renderer,
    renderer: RendererBase
  ) {
    super(p, canvas, renderer)
    
    const {
      minHeight,
      maxHeight,
      bottom,
      top,
      left,
      right,
      height,
      width,
    } = this.boundingBox()

    this.minHeight = minHeight
    this.maxHeight = maxHeight
    this.bottom = bottom
    this.top = top
    this.left = left
    this.right = right
    this.height = height
    this.width = width
  }

  abstract boundingBox(): P5BoundingBox

  #mouseOver = (): boolean => {
    const { mouseX, mouseY } = this.p
    const { top, left, bottom, right } = this

    return (
      left <= mouseX && mouseX <= right &&
      top <= mouseY && mouseY <= bottom
    )
  }

  // call a function when the mouse is over this component
  onMouseOver = (fn: () => void) => {
    if(this.#mouseOver()) {
      fn()
    }
  }

  // call a function when clicking on this component
  onClick = (
    fn: (
      ev: MouseEvent,
      data: {
        target: P5Component<T> 
        mouseX: number
        mouseY: number
      }
    ) => void
  ) => {
    this.onMouseOver(() => {
      this.canvas.mouseClicked((ev: MouseEvent) => {
        const { mouseX, mouseY } = this.p

        fn(ev, { 
          mouseX, 
          mouseY,
          target: this,
        })
      })
    })
  }

  // call function on right click
  // eslint-disable-next-line class-methods-use-this
  onRightClick = (
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _ev: MouseEvent,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _data: {
      target: P5Component<T>
      mouseX: number
      mouseY: number
    }
  ) => {}

  // call a function when double clicking on this component
  onDoubleClick = (
    fn: (
      ev: MouseEvent, 
      data: {
        target: P5Component<T> 
        mouseX: number
        mouseY: number
      }
    ) => void
  ) => {
    this.onMouseOver(() => {
      this.canvas.doubleClicked((ev) => {
        const { mouseX, mouseY } = this.p

        fn(ev, { 
          mouseX, 
          mouseY,
          target: this,
        })
      })
    })
  }
}

export default P5Component
