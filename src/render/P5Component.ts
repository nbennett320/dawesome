/* eslint-disable no-underscore-dangle */
import p5 from 'p5'
import {
  P5CanvasInstance,
  SketchProps,
} from 'react-p5-wrapper'
import P5ComponentBase from './P5ComponentBase'
import RendererBase from './RendererBase'

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type State<S> = S | Record<string, any>

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

abstract class P5Component<T extends SketchProps, S> extends P5ComponentBase<T> {
  private _state: State<S> = {}

  constructor(
    p: P5CanvasInstance<T>,
    canvas: p5.Renderer,
    renderer: RendererBase
  ) {
    super(p, canvas, renderer)
  }

  private componentDidUpdate(prevState: State<S>): void {

  }

  // eslint-disable-next-line @typescript-eslint/no-dupe-class-members, class-methods-use-this
  set state(newState: State<S>) {
    this._state = newState
  }

  get state(): State<S> {
    return this._state
  }

  setState(newState: State<S>) {
    const prevState = this.state

    this.state = newState
    this.componentDidUpdate(prevState)
  }

  abstract boundingBox(): P5BoundingBox

  mouseOver = (): boolean => {
    const { mouseX, mouseY } = this.p
    const { top, left, bottom, right } = this.boundingBox()

    return (
      left <= mouseX && mouseX <= right &&
      top <= mouseY && mouseY <= bottom
    )
  }

  // call a function when the mouse is over this component
  onMouseOver = (fn: () => void) => {
    if(this.mouseOver()) {
      fn()
    }
  }

  // call a function when clicking on this component
  onClick = (
    fn: (
      ev: MouseEvent,
      data: {
        target: P5Component<T, S> 
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
      target: P5Component<T, S>
      mouseX: number
      mouseY: number
    }
  ) => {}

  // call a function when double clicking on this component
  onDoubleClick = (
    fn: (
      ev: MouseEvent, 
      data: {
        target: P5Component<T, S> 
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

  // call a function when releasing on this component
  onMouseReleased = (
    fn: (
      ev: MouseEvent, 
      data: {
        target: P5Component<T, S> 
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
