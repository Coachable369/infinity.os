import CoreGraphics
import Foundation
import ImageIO

enum NativeBitmapEncoder {
    // ------------------------=
    // FUNC: image
    // DESC: Decodes native BMP pixels directly when ImageIO cannot read the OS's top-down V4 alpha layout.
    // ------------------=
    static func image(_ bytes: Data) -> CGImage? {
        guard supportedBitmap(bytes), let space = CGColorSpace(name: CGColorSpace.sRGB) else { return nil }
        let width = Int(value(bytes, 18)), signedHeight = Int(Int32(bitPattern: value(bytes, 22)))
        let height = abs(signedHeight), offset = Int(value(bytes, 10))
        let step = Int(bytes[28]) / 8, row = ((width * step) + 3) & ~3
        let usesAlpha = value(bytes, 30) == 3
        var pixels = Data(count: width * height * 4)
        for y in 0..<height {
            let sourceY = signedHeight < 0 ? y : height - y - 1
            for x in 0..<width {
                let source = offset + sourceY * row + x * step, target = (y * width + x) * 4
                pixels[target] = bytes[source + 2]; pixels[target + 1] = bytes[source + 1]
                pixels[target + 2] = bytes[source]; pixels[target + 3] = usesAlpha ? bytes[source + 3] : 255
            }
        }
        guard let provider = CGDataProvider(data: pixels as CFData) else { return nil }
        return CGImage(width: width, height: height, bitsPerComponent: 8, bitsPerPixel: 32,
            bytesPerRow: width * 4, space: space, bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.last.rawValue),
            provider: provider, decode: nil, shouldInterpolate: false, intent: .defaultIntent)
    }

    // ------------------------=
    // FUNC: dimensions
    // DESC: Returns bounded native BMP dimensions without rasterizing or changing its stored pixels.
    // ------------------=
    static func dimensions(_ bytes: Data) -> (Int, Int)? {
        guard supportedBitmap(bytes) else { return nil }
        return (Int(value(bytes, 18)), abs(Int(Int32(bitPattern: value(bytes, 22)))))
    }

    // ------------------------=
    // FUNC: read
    // DESC: Uses the same alpha-preserving bitmap bytes for embedded runtime artwork and unsaved production previews.
    // ------------------=
    static func read(_ url: URL) throws -> Data {
        let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard size > 0, size <= 128 * 1024 * 1024 else { throw NativePreviewIssue.invalid("Image exceeds native byte limits") }
        let bytes = try Data(contentsOf: url)
        if supportedBitmap(bytes) { return bytes }
        guard let source = CGImageSourceCreateWithData(bytes as CFData, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
            throw NativePreviewIssue.invalid("Image cannot be decoded for the native renderer")
        }
        return try encode(image)
    }

    // ------------------------=
    // FUNC: supportedBitmap
    // DESC: Mirrors native BMP layout bounds so existing supported artwork bypasses host color conversion entirely.
    // ------------------=
    static func supportedBitmap(_ bytes: Data) -> Bool {
        guard bytes.count >= 54, bytes[0] == 66, bytes[1] == 77 else { return false }
        let dib = Int(value(bytes, 14)), offset = Int(value(bytes, 10))
        let width = Int(Int32(bitPattern: value(bytes, 18))), height = abs(Int(Int32(bitPattern: value(bytes, 22))))
        let planes = Int(bytes[26]) | Int(bytes[27]) << 8, bits = Int(bytes[28]) | Int(bytes[29]) << 8
        let compression = value(bytes, 30)
        guard dib >= 40, width > 0, height > 0, width <= 8192, height <= 8192,
              width * height <= 16_777_216, planes == 1 else { return false }
        if compression == 3 {
            guard bits == 32, dib >= 56, bytes.count >= 70,
                  value(bytes, 54) == 0x00ff0000, value(bytes, 58) == 0x0000ff00,
                  value(bytes, 62) == 0x000000ff, value(bytes, 66) == 0xff000000 else { return false }
        } else if compression != 0 || (bits != 24 && bits != 32) { return false }
        let row = ((width * (bits / 8)) + 3) & ~3
        return offset >= 14 + dib && offset <= bytes.count && row * height <= bytes.count - offset
    }

    // ------------------------=
    // FUNC: encode
    // DESC: Serializes straight-alpha sRGB pixels to the production-supported top-down V4 BGRA format.
    // ------------------=
    static func encode(_ source: CGImage) throws -> Data {
        guard source.width > 0, source.height > 0, source.width <= 8192, source.height <= 8192,
              source.width * source.height <= 16_777_216,
              let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else {
            throw NativePreviewIssue.invalid("Image exceeds native image limits")
        }
        let width = source.width, height = source.height
        var pixels = Data(count: width * height * 4)
        let byteOrder = source.bitmapInfo.intersection(.byteOrderMask)
        if source.bitsPerComponent == 8, source.bitsPerPixel == 32, source.alphaInfo == .last,
           byteOrder.isEmpty || byteOrder == .byteOrder32Big,
           let decoded = source.dataProvider?.data,
           CFDataGetLength(decoded) >= source.bytesPerRow * height,
           let pointer = CFDataGetBytePtr(decoded) {
            // ImageIO PNG bytes already contain straight alpha. Drawing through a premultiplied
            // CGContext would irreversibly quantize colors at low alpha and diverge from packaging.
            for y in 0..<height {
                pixels.replaceSubrange((y * width * 4)..<((y + 1) * width * 4),
                    with: UnsafeBufferPointer(start: pointer + y * source.bytesPerRow, count: width * 4))
            }
        } else {
            let painted = pixels.withUnsafeMutableBytes { (bytes: UnsafeMutableRawBufferPointer) -> Bool in
                guard let context = CGContext(data: bytes.baseAddress, width: width, height: height, bitsPerComponent: 8,
                    bytesPerRow: width * 4, space: colorSpace,
                    bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue) else { return false }
                context.draw(source, in: CGRect(x: 0, y: 0, width: width, height: height))
                return true
            }
            guard painted else { throw NativePreviewIssue.invalid("Image cannot be rasterized") }
            for offset in stride(from: 0, to: pixels.count, by: 4) {
                let alpha = Int(pixels[offset + 3])
                for c in 0..<3 {
                    pixels[offset + c] = alpha == 0 ? 0 : UInt8(min(255, (Int(pixels[offset + c]) * 255 + alpha / 2) / alpha))
                }
            }
        }
        var result = Data(repeating: 0, count: 122 + pixels.count)
        result[0] = 66; result[1] = 77
        put(UInt32(122 + pixels.count), at: 2, into: &result)
        put(122, at: 10, into: &result); put(108, at: 14, into: &result)
        put(UInt32(width), at: 18, into: &result)
        put(UInt32(bitPattern: -Int32(height)), at: 22, into: &result)
        result[26] = 1; result[28] = 32
        put(3, at: 30, into: &result); put(UInt32(pixels.count), at: 34, into: &result)
        for (offset, mask) in [(54, UInt32(0x00ff0000)), (58, 0x0000ff00), (62, 0x000000ff), (66, 0xff000000)] {
            put(mask, at: offset, into: &result)
        }
        put(0x73524742, at: 70, into: &result)
        pixels.withUnsafeBytes { (source: UnsafeRawBufferPointer) in
            result.withUnsafeMutableBytes { (target: UnsafeMutableRawBufferPointer) in
                for offset in stride(from: 0, to: source.count, by: 4) {
                    target[122 + offset] = source[offset + 2]
                    target[123 + offset] = source[offset + 1]
                    target[124 + offset] = source[offset]
                    target[125 + offset] = source[offset + 3]
                }
            }
        }
        return result
    }

    // ------------------------=
    // FUNC: value
    // DESC: Reads one validated little-endian bitmap field independently of host byte order.
    // ------------------=
    private static func value(_ bytes: Data, _ offset: Int) -> UInt32 {
        (0..<4).reduce(0) { $0 | UInt32(bytes[offset + $1]) << ($1 * 8) }
    }

    // ------------------------=
    // FUNC: put
    // DESC: Writes one little-endian V4 header field.
    // ------------------=
    private static func put(_ value: UInt32, at offset: Int, into data: inout Data) {
        for byte in 0..<4 { data[offset + byte] = UInt8(truncatingIfNeeded: value >> (byte * 8)) }
    }
}
