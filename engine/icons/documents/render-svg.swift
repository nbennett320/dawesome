// render an SVG to a square PNG, keeping its transparency, with macOS's
// own SVG support (build-icons.sh uses it)
//   swiftc render-svg.swift -o render-svg
//   ./render-svg in.svg out.png 512
import AppKit

let args = CommandLine.arguments
guard args.count == 4, let size = Int(args[3]) else {
  print("usage: render-svg in.svg out.png size")
  exit(2)
}
guard let image = NSImage(contentsOfFile: args[1]) else {
  print("could not read \(args[1])")
  exit(1)
}
let rep = NSBitmapImageRep(
  bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8, samplesPerPixel: 4,
  hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
image.draw(in: NSRect(x: 0, y: 0, width: size, height: size))
NSGraphicsContext.restoreGraphicsState()
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: args[2]))
