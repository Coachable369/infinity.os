import AppKit
import ImageIO
import UniformTypeIdentifiers

let root = URL(fileURLWithPath: CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : FileManager.default.currentDirectoryPath)
let names = ["classic-white", "classic-black", "outline", "crystal", "silver", "comet", "rocket", "leaf", "wand", "pixel"]
let size = 128
var sheet = [UInt8](repeating: 0, count: 1000 * 440 * 4)
let space = CGColorSpaceCreateDeviceRGB()
let info = CGImageAlphaInfo.premultipliedLast.rawValue
let sheetContext = CGContext(data: &sheet, width: 1000, height: 440, bitsPerComponent: 8, bytesPerRow: 4000, space: space, bitmapInfo: info)!
sheetContext.setFillColor(CGColor(red: 0.035, green: 0.075, blue: 0.12, alpha: 1))
sheetContext.fill(CGRect(x: 0, y: 0, width: 1000, height: 440))
var metadata = "// Generated from RGBA sprites by tools/build-cursor-assets.swift.\n"
metadata += "pub const HOTSPOTS: [(usize, usize); 10] = [\n"
for (index, name) in names.enumerated() {
    let source = CGImageSourceCreateWithURL(root.appendingPathComponent("assets/cursors/generated/\(name).png") as CFURL, nil)!
    let image = CGImageSourceCreateImageAtIndex(source, 0, nil)!
    var original = [UInt8](repeating: 0, count: image.width * image.height * 4)
    let context = CGContext(data: &original, width: image.width, height: image.height, bitsPerComponent: 8, bytesPerRow: image.width * 4, space: space, bitmapInfo: info)!
    context.draw(image, in: CGRect(x: 0, y: 0, width: image.width, height: image.height))
    var minX = image.width, minY = image.height, maxX = 0, maxY = 0
    for y in 0..<image.height { for x in 0..<image.width {
        if original[(y * image.width + x) * 4 + 3] > 8 {
            minX = min(minX, x); minY = min(minY, y); maxX = max(maxX, x); maxY = max(maxY, y)
        }
    }}
    let cropped = context.makeImage()!.cropping(to: CGRect(x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1))!
    var pixels = [UInt8](repeating: 0, count: size * size * 4)
    let target = CGContext(data: &pixels, width: size, height: size, bitsPerComponent: 8, bytesPerRow: size * 4, space: space, bitmapInfo: info)!
    target.interpolationQuality = name == "pixel" ? .none : .high
    let factor = 120.0 / Double(max(cropped.width, cropped.height))
    let w = Double(cropped.width) * factor, h = Double(cropped.height) * factor
    target.draw(cropped, in: CGRect(x: 4, y: Double(size) - 4 - h, width: w, height: h))
    let preview = target.makeImage()!
    let col = index % 5, row = index / 5
    sheetContext.draw(preview, in: CGRect(x: col * 200 + 36, y: 440 - row * 220 - 160, width: 128, height: 128))
    sheetContext.draw(preview, in: CGRect(x: col * 200 + 85, y: 440 - row * 220 - 205, width: 32, height: 32))
    var hotspot = (x: 0, y: 0), score = Int.max
    for y in 0..<size { for x in 0..<size {
        let p = (y * size + x) * 4, alpha = Int(pixels[p + 3])
        if alpha > 200 && x + y < score { hotspot = (x, y); score = x + y }
        if alpha > 0 { for c in 0..<3 { pixels[p + c] = UInt8(min(255, Int(pixels[p + c]) * 255 / alpha)) } }
    }}
    try Data(pixels).write(to: root.appendingPathComponent("assets/cursors/\(name).rgba"))
    metadata += "    (\(hotspot.x), \(hotspot.y)), // \(name)\n"
}
metadata += "];\n"
try metadata.write(to: root.appendingPathComponent("assets/cursors/hotspots.rs"), atomically: true, encoding: .utf8)
let destination = CGImageDestinationCreateWithURL(root.appendingPathComponent("docs/design/spatial-pointer/cursor-contact-sheet.png") as CFURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(destination, sheetContext.makeImage()!, nil)
precondition(CGImageDestinationFinalize(destination))
