
import p5 from 'p5'
import { P5CanvasInstance } from 'react-p5-wrapper'
import PlaylistComponentBase, { PlaylistComponentBaseProps } from './PlaylistComponentBase'
import { CanvasProps } from './index'

interface Props extends PlaylistComponentBaseProps {}

class PlaylistItem extends PlaylistComponentBase {
  constructor(
    p: P5CanvasInstance<CanvasProps>,
    canvas: p5.Renderer,
    props: Props,
  ) {
    super(p, canvas, props)
  }
  
  render = () => {

  }
}

export default PlaylistItem
