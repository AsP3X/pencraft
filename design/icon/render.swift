// Draws an SVG into a square PNG with a transparent background: swift render.swift in.svg out.png size [inset shadow]
import AppKit
let a = CommandLine.arguments
guard a.count >= 4, let size = Int(a[3]), let image = NSImage(contentsOfFile: a[1]) else {
    FileHandle.standardError.write("usage: render.swift in.svg out.png size [mac]\n".data(using: .utf8)!)
    exit(2)
}
let mac = a.count > 4 && a[4] == "mac"
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: size, pixelsHigh: size, bitsPerSample: 8, samplesPerPixel: 4,
                           hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
rep.size = NSSize(width: size, height: size)
NSGraphicsContext.saveGraphicsState()
let context = NSGraphicsContext(bitmapImageRep: rep)!
NSGraphicsContext.current = context
context.imageInterpolation = .high
let s = CGFloat(size)
if mac {
    // Apple's grid for a Mac app icon: the plate is 824 of 1024 wide, with a soft shadow under it.
    let plate = NSRect(x: s * 100 / 1024, y: s * 100 / 1024, width: s * 824 / 1024, height: s * 824 / 1024)
    let shadow = NSShadow()
    shadow.shadowColor = NSColor(white: 0, alpha: 0.3)
    shadow.shadowOffset = NSSize(width: 0, height: -s * 10 / 1024)
    shadow.shadowBlurRadius = s * 21 / 1024
    shadow.set()
    image.draw(in: plate)
} else {
    image.draw(in: NSRect(x: 0, y: 0, width: s, height: s))
}
NSGraphicsContext.restoreGraphicsState()
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: a[2]))
