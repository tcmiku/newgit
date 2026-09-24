import CoreGraphics
import Foundation
import ImageIO

guard CommandLine.arguments.count == 2 else {
    fputs("Usage: swift scripts/generate-tray-icon.swift OUTPUT.png\n", stderr)
    exit(1)
}

let size = 44
guard let context = CGContext(data: nil, width: size, height: size,
                              bitsPerComponent: 8, bytesPerRow: 0,
                              space: CGColorSpaceCreateDeviceRGB(),
                              bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else {
    fatalError("Could not create tray icon canvas")
}
context.setStrokeColor(CGColor(red: 0, green: 0, blue: 0, alpha: 1))
context.setLineWidth(3.6)
context.setLineCap(.round)
context.move(to: CGPoint(x: 13, y: 17))
context.addLine(to: CGPoint(x: 13, y: 27))
context.move(to: CGPoint(x: 30, y: 17))
context.addLine(to: CGPoint(x: 30, y: 21))
context.addCurve(to: CGPoint(x: 13, y: 27),
                 control1: CGPoint(x: 30, y: 27),
                 control2: CGPoint(x: 13, y: 21))
context.strokePath()
for point in [CGPoint(x: 13, y: 12), CGPoint(x: 30, y: 12), CGPoint(x: 13, y: 32)] {
    context.strokeEllipse(in: CGRect(x: point.x - 4.1, y: point.y - 4.1,
                                     width: 8.2, height: 8.2))
}

guard let image = context.makeImage(),
      let destination = CGImageDestinationCreateWithURL(
        URL(fileURLWithPath: CommandLine.arguments[1]) as CFURL,
        "public.png" as CFString, 1, nil) else {
    fatalError("Could not write tray icon")
}
CGImageDestinationAddImage(destination, image, nil)
guard CGImageDestinationFinalize(destination) else { fatalError("Could not finish tray icon") }
