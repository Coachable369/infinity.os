import XCTest
import SwiftUI
@testable import InfinityInstallerStudio

final class PrecisionPreviewTests: XCTestCase {
    private var root: URL {
        URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
    }

    // ------------------------=
    // FUNC: atlas
    // DESC: Loads the same checked-in production atlas bytes used by the OS and Studio.
    // ------------------=
    private func atlas() throws -> PrecisionAtlas {
        try XCTUnwrap(PrecisionAtlas(data: Data(contentsOf: root.appending(path: "assets/ui-design-kit/default/eclipse-precision-v1.rgba"))))
    }

    // ------------------------=
    // FUNC: testNativeSamplerParity
    // DESC: Compares every Swift sample to executable native-kernel evidence, including transparent bounds and intermediate focus states.
    // ------------------=
    func testNativeSamplerParity() throws {
        let native = try Data(contentsOf: root.appending(path: "build/behavior-tests/studio-precision.parity"))
        XCTAssertGreaterThan(native.count, 12)
        XCTAssertEqual(Array(native.prefix(8)), [73, 80, 65, 82, 73, 84, 89, 49])
        let count = (0..<4).reduce(0) { $0 | (Int(native[8 + $1]) << ($1 * 8)) }
        XCTAssertEqual(native.count, 12 + count * 24)
        XCTAssertGreaterThan(count, 100_000)
        let atlas = try atlas()
        let surfaces: [PrecisionSurfaceKind] = [.frame, .windowFill, .panel, .primary, .secondary, .field, .current, .inactive]
        for offset in stride(from: 12, to: native.count, by: 24) {
            let values = (0..<24).map { native[offset + $0] }
            let width = Int(values[6]) | Int(values[7]) << 8
            let height = Int(values[8]) | Int(values[9]) << 8
            let x = Int(values[10]) | Int(values[11]) << 8
            let y = Int(values[12]) | Int(values[13]) << 8
            let material = PrecisionMaterial(fill: StudioColor(red: values[14], green: values[15], blue: values[16], alpha: 255),
                edge: StudioColor(red: values[17], green: values[18], blue: values[19], alpha: 255),
                opacity: values[3], edgeOpacity: values[4], focus: values[1], pressed: values[2] != 0, scale: Int(values[5]))
            let actual = atlas.pixel(surfaces[Int(values[0])], width: width, height: height, x: x, y: y, material: material)
            let expected = PrecisionPixel(red: Int(values[20]), green: Int(values[21]), blue: Int(values[22]), alpha: Int(values[23]))
            XCTAssertEqual(actual, expected, "native record \((offset - 12) / 24)")
            if actual != expected { return }
        }
    }

    // ------------------------=
    // FUNC: testNativeSurfaceStyleParity
    // DESC: Compares selectable material and corner geometry to native executable pixels across scale, state, alpha and per-family color overrides.
    // ------------------=
    func testNativeSurfaceStyleParity() throws {
        let native = try Data(contentsOf: root.appending(path: "build/behavior-tests/studio-precision.styles"))
        XCTAssertEqual(Array(native.prefix(8)), Array("ISTYLE01".utf8))
        let count = (0..<4).reduce(0) { $0 | Int(native[8 + $1]) << ($1 * 8) }
        XCTAssertEqual(native.count, 12 + count * 36)
        XCTAssertGreaterThan(count, 100_000)
        let atlas = try atlas()
        let surfaces: [PrecisionSurfaceKind] = [.frame, .windowFill, .panel, .primary, .secondary, .field, .current, .inactive]
        for offset in stride(from: 12, to: native.count, by: 36) {
            let values = Array(native[offset..<offset + 36])
            let width = Int(values[6]) | Int(values[7]) << 8, height = Int(values[8]) | Int(values[9]) << 8
            let x = Int(values[10]) | Int(values[11]) << 8, y = Int(values[12]) | Int(values[13]) << 8
            let fill = (0..<4).reduce(UInt32(0)) { $0 | UInt32(values[28 + $1]) << ($1 * 8) }
            let border = (0..<4).reduce(UInt32(0)) { $0 | UInt32(values[32 + $1]) << ($1 * 8) }
            let style = UIKitSurfaceStyle(material: try XCTUnwrap(UIKitSurfaceMaterial(rawValue: Int(values[24]))),
                corners: try XCTUnwrap(UIKitCornerStyle(rawValue: Int(values[25]))), radius: Int(values[26]),
                fill: values[27] & 1 == 0 ? nil : fill, border: values[27] & 2 == 0 ? nil : border)
            let material = PrecisionMaterial(fill: StudioColor(red: values[14], green: values[15], blue: values[16], alpha: 255),
                edge: StudioColor(red: values[17], green: values[18], blue: values[19], alpha: 255),
                opacity: values[3], edgeOpacity: values[4], focus: values[1], pressed: values[2] != 0,
                scale: Int(values[5]), style: style)
            let actual = atlas.pixel(surfaces[Int(values[0])], width: width, height: height, x: x, y: y, material: material)
            let expected = PrecisionPixel(red: Int(values[20]), green: Int(values[21]), blue: Int(values[22]), alpha: Int(values[23]))
            XCTAssertEqual(actual, expected, "native style record \((offset - 12) / 36)")
            if actual != expected { return }
        }
    }

    // ------------------------=
    // FUNC: testProgressMarkerOverridesChangePixelsWithoutReplacingAuthoredDefaults
    // DESC: Proves material and explicit geometry edits affect both marker states while default glass and matte alpha preserve original artwork coverage.
    // ------------------=
    func testProgressMarkerOverridesChangePixelsWithoutReplacingAuthoredDefaults() throws {
        let atlas = try atlas()
        for kind in [PrecisionSurfaceKind.current, .inactive] {
            let baseline = PrecisionMaterial()
            let authored = try XCTUnwrap(atlas.render(kind, width: 80, height: 36, material: baseline))
            var glass = baseline
            glass.style.material = .glass
            XCTAssertEqual(try XCTUnwrap(atlas.render(kind, width: 80, height: 36, material: glass)), authored)
            var matte = baseline
            matte.style.material = .matte
            let flat = try XCTUnwrap(atlas.render(kind, width: 80, height: 36, material: matte))
            XCTAssertNotEqual(flat, authored)
            XCTAssertEqual(stride(from: 3, to: flat.count, by: 4).map { flat[$0] },
                           stride(from: 3, to: authored.count, by: 4).map { authored[$0] })
            var square = glass
            square.style.corners = .square
            let squared = try XCTUnwrap(atlas.render(kind, width: 80, height: 36, material: square))
            var rounded = glass
            rounded.style.corners = .rounded
            rounded.style.radius = 16
            let curved = try XCTUnwrap(atlas.render(kind, width: 80, height: 36, material: rounded))
            XCTAssertNotEqual(squared, curved)
            XCTAssertNotEqual(curved, authored)
            XCTAssertGreaterThan(squared[(3 * 80 + 3) * 4 + 3], curved[(3 * 80 + 3) * 4 + 3])
        }
    }

    // ------------------------=
    // FUNC: testTemplateStyleInheritanceAndWindowShape
    // DESC: Verifies independent button families, explicit per-element color ownership and compositor-controlled window silhouettes.
    // ------------------=
    func testTemplateStyleInheritanceAndWindowShape() {
        var styles = UIKitSurfaceStyle.defaults
        styles[0] = UIKitSurfaceStyle(corners: .rounded, radius: 24)
        styles[2] = UIKitSurfaceStyle(material: .matte, corners: .square, fill: 0xff12ab34, border: 0xffabcd12)
        styles[3] = UIKitSurfaceStyle(material: .glass, corners: .rounded, radius: 18)
        let panel = UIKitSurfaceStyle.resolved(for: .panel, styles: styles)
            .template(material: .authored, corners: .rounded, radius: 12)
        XCTAssertEqual(panel.material, .matte)
        XCTAssertEqual(panel.corners, .rounded)
        XCTAssertEqual(panel.radius, 12)
        XCTAssertNil(panel.fill)
        XCTAssertNil(panel.border)
        XCTAssertEqual(UIKitSurfaceStyle.resolved(for: .windowFill, styles: styles).radius, 24)
        XCTAssertEqual(UIKitSurfaceStyle.resolved(for: .secondary, styles: styles), .authored)
        XCTAssertEqual(UIKitSurfaceStyle.resolved(for: .current, styles: styles), .authored)
        XCTAssertEqual(UIKitSurfaceStyle.resolved(for: .primary, styles: styles).material, .glass)
    }

    // ------------------------=
    // FUNC: testNativeFrostParity
    // DESC: Compares complete blur, navy tint, sheen, alpha endpoints and theme changes against native compositor output rather than a screenshot approximation.
    // ------------------=
    func testNativeFrostParity() throws {
        let fixture = try Data(contentsOf: root.appending(path: "build/behavior-tests/studio-precision.frost"))
        XCTAssertEqual(Array(fixture.prefix(8)), [73, 70, 82, 79, 83, 84, 48, 49])
        let count = (0..<4).reduce(0) { $0 | Int(fixture[8 + $1]) << ($1 * 8) }
        XCTAssertEqual(count, 32)
        let atlas = try atlas()
        var offset = 12
        for scene in 0..<count {
            let header = Array(fixture[offset..<offset + 14])
            let width = Int(header[10]) | Int(header[11]) << 8
            let height = Int(header[12]) | Int(header[13]) << 8
            let length = width * height * 4
            offset += 14
            var background = Data(count: length)
            for y in 0..<height { for x in 0..<width {
                let at = (y * width + x) * 4
                background[at] = UInt8((x * 31 + y * 7) % 256)
                background[at + 1] = UInt8((x * 3 + y * 29) % 256)
                background[at + 2] = UInt8((x * 19 + y * 11) % 256)
                background[at + 3] = 255
            } }
            let material = PrecisionMaterial(fill: StudioColor(red: header[0], green: header[1], blue: header[2], alpha: header[6]),
                edge: StudioColor(red: header[3], green: header[4], blue: header[5], alpha: header[7]), scale: Int(header[8]))
            let actual = try XCTUnwrap(PrecisionFrostRenderer.render(background: background, width: width, height: height,
                material: material, atlas: atlas, blur: Int(header[9])))
            let expected = fixture.subdata(in: offset..<offset + length)
            XCTAssertTrue(actual == expected, "native frost scene \(scene)")
            if actual != expected { return }
            offset += length
        }
        XCTAssertEqual(offset, fixture.count)
    }

    // ------------------------=
    // FUNC: testNativeStyledFrostParity
    // DESC: Verifies complete custom glass and matte composites against native pixels, including rounded masks, tint alpha and unchanged background outside corners.
    // ------------------=
    func testNativeStyledFrostParity() throws {
        let fixture = try Data(contentsOf: root.appending(path: "build/behavior-tests/studio-precision.stylefrost"))
        XCTAssertEqual(Array(fixture.prefix(8)), Array("ISTFRO01".utf8))
        let count = (0..<4).reduce(0) { $0 | Int(fixture[8 + $1]) << ($1 * 8) }
        XCTAssertEqual(count, 108)
        let atlas = try atlas()
        var offset = 12
        for scene in 0..<count {
            let h = Array(fixture[offset..<offset + 26])
            let width = Int(h[10]) | Int(h[11]) << 8, height = Int(h[12]) | Int(h[13]) << 8
            let length = width * height * 4
            offset += 26
            var background = Data(count: length)
            for y in 0..<height { for x in 0..<width {
                let at = (y * width + x) * 4
                background[at] = UInt8((x * 31 + y * 7) % 256)
                background[at + 1] = UInt8((x * 3 + y * 29) % 256)
                background[at + 2] = UInt8((x * 19 + y * 11) % 256)
                background[at + 3] = 255
            } }
            let fill = (0..<4).reduce(UInt32(0)) { $0 | UInt32(h[18 + $1]) << ($1 * 8) }
            let border = (0..<4).reduce(UInt32(0)) { $0 | UInt32(h[22 + $1]) << ($1 * 8) }
            let style = UIKitSurfaceStyle(material: try XCTUnwrap(UIKitSurfaceMaterial(rawValue: Int(h[14]))),
                corners: try XCTUnwrap(UIKitCornerStyle(rawValue: Int(h[15]))), radius: Int(h[16]),
                fill: h[17] & 1 == 0 ? nil : fill, border: h[17] & 2 == 0 ? nil : border)
            let material = PrecisionMaterial(fill: StudioColor(red: h[0], green: h[1], blue: h[2], alpha: h[6]),
                edge: StudioColor(red: h[3], green: h[4], blue: h[5], alpha: h[7]), scale: Int(h[8]), style: style)
            let actual = try XCTUnwrap(PrecisionFrostRenderer.render(background: background, width: width, height: height,
                material: material, atlas: atlas, blur: Int(h[9])))
            let expected = fixture.subdata(in: offset..<offset + length)
            XCTAssertTrue(actual == expected, "native styled frost scene \(scene)")
            if actual != expected { return }
            offset += length
        }
        XCTAssertEqual(offset, fixture.count)
    }

    // ------------------------=
    // FUNC: testRenderedBufferAndIndependentOpacity
    // DESC: Exercises emitted image bytes, independent border alpha, native corner extent and bounded allocation.
    // ------------------=
    func testRenderedBufferAndIndependentOpacity() throws {
        let atlas = try atlas()
        let material = PrecisionMaterial(opacity: 0, focus: 255)
        let rgba = try XCTUnwrap(atlas.render(.frame, width: 112, height: 64, material: material))
        XCTAssertEqual(rgba.count, 112 * 64 * 4)
        XCTAssertGreaterThan(rgba[3 + (1 * 112 + 1) * 4], 0)
        XCTAssertEqual(rgba[3 + (32 * 112 + 56) * 4], 0)
        XCTAssertEqual(PrecisionAtlas.rimWidth(.frame, scale: 2), 36)
        XCTAssertNil(atlas.render(.panel, width: 4097, height: 10, material: material))
        XCTAssertNil(PrecisionAtlas(data: Data(repeating: 0, count: 12)))
        let transparent = try XCTUnwrap(atlas.render(.panel, width: 100, height: 50,
            material: PrecisionMaterial(opacity: 0, edgeOpacity: 0, focus: 255)))
        XCTAssertTrue(transparent.allSatisfy { $0 == 0 })
    }

    // ------------------------=
    // FUNC: testUploadedCellAffectsOnlyItsNativeState
    // DESC: Replaces one complete authored cell in memory and checks that only its selected state receives the new pixels.
    // ------------------=
    func testUploadedCellAffectsOnlyItsNativeState() throws {
        let original = try atlas()
        var changed = original.bytes
        let start = 4 * 256 * 256 * 4
        for offset in stride(from: start, to: start + 256 * 256 * 4, by: 4) {
            changed.replaceSubrange(offset..<offset + 4, with: [37, 61, 191, 255])
        }
        let custom = try XCTUnwrap(PrecisionAtlas(data: changed))
        let material = PrecisionMaterial(fill: InfinityUIKit.EclipseAccess.primary, edgeOpacity: 0)
        XCTAssertEqual(custom.pixel(.primary, width: 120, height: 48, x: 60, y: 24, material: material),
            PrecisionPixel(red: 37, green: 61, blue: 191, alpha: 255))
        XCTAssertEqual(custom.pixel(.field, width: 120, height: 48, x: 60, y: 24, material: material),
            original.pixel(.field, width: 120, height: 48, x: 60, y: 24, material: material))
    }

    // ------------------------=
    // FUNC: testCaptureProductionCanvasScenes
    // DESC: Exports the real canvas views for human review of both wizard collections without mutating project templates or using a mockup renderer.
    // ------------------=
    @MainActor func testCaptureProductionCanvasScenes() throws {
        guard let directory = ProcessInfo.processInfo.environment["INFINITY_STUDIO_PRECISION_PROOFS"] else { return }
        let destination = URL(fileURLWithPath: directory).standardizedFileURL.resolvingSymlinksInPath()
        let allowed = root.appending(path: "builds", directoryHint: .isDirectory).standardizedFileURL.resolvingSymlinksInPath()
        guard destination.path.hasPrefix(allowed.path + "/") else { throw CocoaError(.fileWriteInvalidFileName) }
        try FileManager.default.createDirectory(at: destination, withIntermediateDirectories: true)
        let store = TemplateStore()
        for (collection, screen, name) in [(ScreenCollection.installation, 6, "installer-review"),
            (.configuration, 1, "configuration-welcome"), (.configuration, 5, "configuration-security"),
            (.configuration, 7, "configuration-network")] {
            store.selectScreenCollection(collection, screen: screen)
            store.selectElement(nil)
            let layers = try XCTUnwrap(store.selectedScreen?.elements)
                .filter { !$0.hidden }
                .sorted { $0.zIndex == $1.zIndex ? $0.id.uuidString < $1.id.uuidString : $0.zIndex < $1.zIndex }
            let view = ZStack(alignment: .topLeading) {
                CanvasRuntimeBackdrop(store: store)
                ForEach(layers) { element in
                    CanvasElementView(element: element, store: store, canvasScale: CGSize(width: 1.6, height: 1), renderingBackdrop: true)
                }
                if collection == .configuration, screen == 7,
                   let content = layers.first(where: { $0.role == .content }) {
                    ConfigurationNetworkPreview(canvasScale: CGSize(width: 1.6, height: 1), content: content.frame,
                        projectRoot: store.projectRoot,
                        atlasData: store.uiKit.previewAtlas(target: "assets/ui-design-kit/default/eclipse-precision-v1.rgba"),
                        glyphData: store.uiKit.previewAtlas(target: "assets/ui-design-kit/default/eclipse-access-glyphs-v1.rgba"),
                        styles: store.uiKit.document.styles)
                }
            }.frame(width: 1600, height: 1000)
            let image = try XCTUnwrap(ImageRenderer(content: view).cgImage)
            let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
            try png.write(to: destination.appending(path: "\(name).png"))
            XCTAssertEqual(image.width, 1600)
            XCTAssertEqual(image.height, 1000)
        }
        let originalStyles = store.uiKit.document.styles
        for (name, material, corners) in [("surface-styles-glass", UIKitSurfaceMaterial.glass, UIKitCornerStyle.rounded),
            ("surface-styles-matte", .matte, .square)] {
            for index in 2..<6 {
                var style = originalStyles[index]
                style.material = material; style.corners = corners; style.radius = 18
                store.uiKit.setSurfaceStyle(index, style: style)
            }
            let view = UIKitStylesPreview(store: store.uiKit).padding(24).frame(width: 940)
                .background(InfinityUIKit.Palette.nativeCanvas)
            let image = try XCTUnwrap(ImageRenderer(content: view).cgImage)
            let png = try XCTUnwrap(NSBitmapImageRep(cgImage: image).representation(using: .png, properties: [:]))
            try png.write(to: destination.appending(path: "\(name).png"))
            XCTAssertEqual(image.width, 940)
            XCTAssertGreaterThan(image.height, 600)
        }
    }
}
