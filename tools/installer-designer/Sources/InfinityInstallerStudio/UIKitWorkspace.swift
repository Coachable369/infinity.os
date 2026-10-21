import AppKit
import SwiftUI

struct UIKitWorkspace: View {
    @ObservedObject var store: UIKitStore
    private let columns = [GridItem(.adaptive(minimum: 164, maximum: 230), spacing: 16)]

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 12) {
                VStack(alignment: .leading, spacing: 4) {
                    Text(store.selectedCategory).font(.title2.weight(.semibold))
                    Text("Shared artwork and color settings used by InfinityOS")
                        .font(.caption).foregroundStyle(.secondary)
                }
                Spacer()
                if store.isDirty {
                    Label("Unsaved", systemImage: "circle.fill")
                        .font(.caption).foregroundStyle(.orange)
                }
            }.padding(20)
            Divider()
            if store.selectedCategory == "Themes" {
                ThemeLibraryWorkspace(store: store)
            } else if store.selectedCategory == "Surface Styles" {
                ScrollView { UIKitStylesPreview(store: store).padding(24) }
            } else if store.selectedCategory == "Colors & Theme" {
                ScrollView {
                    UIKitColorPreview(store: store).padding(24)
                }
            } else {
                HStack {
                    Image(systemName: "magnifyingglass").foregroundStyle(.secondary)
                    TextField("Find an OS element, icon, wallpaper…", text: $store.searchText)
                        .textFieldStyle(.plain)
                        .accessibilityIdentifier("studio.uikit.search")
                    if !store.searchText.isEmpty {
                        Button { store.searchText = "" } label: { Image(systemName: "xmark.circle.fill") }
                            .buttonStyle(.plain).help("Clear search")
                    }
                }
                .padding(12).background(InfinityUIKit.Palette.nativePanelRaised)
                .clipShape(RoundedRectangle(cornerRadius: 8)).padding(16)
                ScrollView {
                    LazyVGrid(columns: columns, alignment: .leading, spacing: 16) {
                        ForEach(store.filteredAssets) { asset in
                            assetCard(asset)
                        }
                    }.padding(.horizontal, 20).padding(.bottom, 20)
                    if store.filteredAssets.isEmpty {
                        ContentUnavailableView.search(text: store.searchText)
                    }
                }
            }
        }.background(InfinityUIKit.Palette.nativeCanvas)
    }

    // ------------------------=
    // FUNC: assetCard
    // DESC: Displays the actual alpha-preserving source pixels and a stable selectable runtime asset identity.
    // ------------------=
    private func assetCard(_ asset: UIKitAsset) -> some View {
        let selected = store.selectedAssetID == asset.id
        return Button { store.selectAsset(asset.id) } label: {
            VStack(alignment: .leading, spacing: 10) {
                ZStack {
                    Color(red: 0.035, green: 0.055, blue: 0.11)
                    if let image = store.previewImage(for: asset, maxDimension: 192) {
                        Image(nsImage: image).resizable().interpolation(.high)
                            .scaledToFit().padding(12)
                    } else {
                        VStack(spacing: 8) {
                            Image(systemName: "exclamationmark.triangle")
                            Text("Asset unavailable").font(.caption)
                        }.foregroundStyle(.orange)
                    }
                }.frame(height: 120).clipped()
                VStack(alignment: .leading, spacing: 4) {
                    Text(asset.title).font(.callout.weight(.medium)).lineLimit(2)
                    Text(asset.category).font(.caption2).foregroundStyle(.secondary)
                }.padding(.horizontal, 12).padding(.bottom, 12)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(InfinityUIKit.Palette.nativePanel)
            .clipShape(RoundedRectangle(cornerRadius: 10))
            .overlay(RoundedRectangle(cornerRadius: 10).strokeBorder(
                selected ? InfinityUIKit.Palette.nativeAccent : InfinityUIKit.Palette.nativeBorder.opacity(0.35),
                lineWidth: selected ? 2 : 1))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("studio.uikit.asset.\(asset.id)")
        .help(asset.target)
    }
}

struct UIKitInspector: View {
    @ObservedObject var store: UIKitStore
    @State private var lightBackdrop = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                Text(store.selectedCategory == "Colors & Theme" ? "THEME COLORS" : "ELEMENT INSPECTOR")
                    .font(.caption.weight(.semibold)).foregroundStyle(.secondary)
                if store.selectedCategory == "Themes" {
                    Text("Named OS themes").font(.title3.weight(.semibold))
                    Text("A theme saves the semantic palette, surface recipes and uploaded artwork together. Apply a preset, refine it in Colors & Theme or Surface Styles, then save a new theme.")
                        .font(.callout).foregroundStyle(.secondary)
                    Label("Undoable application", systemImage: "arrow.uturn.backward")
                    Text("Apply changes the current UIKit draft. Save Section writes it to the project; Save & Update ISO publishes it to your test image.")
                        .font(.caption).foregroundStyle(.secondary)
                    Label("Portable artwork", systemImage: "shippingbox")
                    Text("Theme exports embed uploaded PNG artwork. Screen-specific colors and layouts remain in their screen documents.")
                        .font(.caption).foregroundStyle(.secondary)
                } else if store.selectedCategory == "Surface Styles" {
                    UIKitStyleInspector(store: store)
                } else if store.selectedCategory == "Colors & Theme" {
                    ForEach(UIKitColorRole.allCases) { role in
                        UIKitColorEditor(store: store, role: role)
                    }
                    Button("Reset Theme Colors", action: store.resetColors)
                        .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
                    Text("These semantic colors are compiled into the OS default skin. Existing user-selected themes and explicit screen-layer colors remain independent.")
                        .font(.caption).foregroundStyle(.secondary)
                } else if let asset = store.selectedAsset {
                    Text(asset.title).font(.title3.weight(.semibold))
                    if let image = store.previewImage(for: asset) {
                        Image(nsImage: image).resizable().interpolation(.high).scaledToFit()
                            .padding(12).frame(maxWidth: .infinity, minHeight: 160, maxHeight: 240)
                            .background(lightBackdrop ? Color(white: 0.85) : Color(white: 0.045))
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                    }
                    Toggle("Inspect alpha on light background", isOn: $lightBackdrop).font(.caption)
                    GroupBox("Runtime identity") {
                        VStack(alignment: .leading, spacing: 8) {
                            Text(asset.id).font(.caption.monospaced()).textSelection(.enabled)
                            Text(asset.target).font(.caption2.monospaced()).textSelection(.enabled)
                            Text(asset.runtimeDescription).font(.caption).foregroundStyle(.secondary)
                        }.frame(maxWidth: .infinity, alignment: .leading).padding(4)
                    }
                    Button(action: store.chooseReplacementImage) {
                        Label("Upload Replacement…", systemImage: "photo.badge.plus")
                    }.buttonStyle(InfinityStudioButtonStyle(emphasis: .primary))
                    Button("Reset This Element", action: store.resetSelectedAsset)
                        .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
                    Text("Transparent PNG is recommended for glyphs and edges. Replacements are validated against the native dimensions and preserved through rebuilds. Save Section commits this artwork to the project.")
                        .font(.caption).foregroundStyle(.secondary)
                } else {
                    ContentUnavailableView("Select an OS element", systemImage: "cursorarrow.click",
                        description: Text("Select a component, icon, wallpaper or other image to inspect and replace its artwork."))
                }
                Divider()
                Label("Live ISO + fresh installation", systemImage: "checkmark.shield")
                    .font(.caption.weight(.medium))
                Text("Use Save & Update ISO to test saved changes. Studio does not modify a running installed OS.")
                    .font(.caption).foregroundStyle(.secondary)
            }.padding(20)
        }.background(InfinityUIKit.Palette.nativePanel)
    }
}

private struct UIKitColorEditor: View {
    @ObservedObject var store: UIKitStore
    let role: UIKitColorRole
    @State private var hex = ""
    @State private var invalid = false
    @FocusState private var hexFocused: Bool
    private var value: UInt32 { store.document.colors[role.rawValue] }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            ColorPicker(role.title, selection: color, supportsOpacity: true)
                .font(.callout)
            TextField("AARRGGBB", text: $hex)
                .textFieldStyle(.roundedBorder).font(.caption.monospaced())
                .focused($hexFocused)
                .onSubmit(commitHex)
                .onChange(of: hexFocused) { _, focused in if !focused { commitHex() } }
                .onAppear { refreshHex() }
                .onChange(of: value) { _, _ in refreshHex() }
            if invalid { Text("Use #RRGGBB or #AARRGGBB").font(.caption2).foregroundStyle(.orange) }
        }
    }

    private var color: Binding<Color> {
        Binding(get: { studioColor(value).color }, set: { next in
            guard let rgba = NSColor(next).usingColorSpace(.sRGB) else { return }
            let byte: (CGFloat) -> UInt32 = { UInt32(($0 * 255).rounded().clamped(to: 0...255)) }
            store.setColor(role, argb: byte(rgba.alphaComponent) << 24 | byte(rgba.redComponent) << 16 |
                byte(rgba.greenComponent) << 8 | byte(rgba.blueComponent))
        })
    }

    // ------------------------=
    // FUNC: refreshHex
    // DESC: Displays the exact saved ARGB channels without rounding drift through a host color profile.
    // ------------------=
    private func refreshHex() { hex = String(format: "#%08X", value); invalid = false }

    // ------------------------=
    // FUNC: commitHex
    // DESC: Applies a validated RGB or ARGB color and leaves malformed input visible for correction.
    // ------------------=
    private func commitHex() {
        let trimmed = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        let digits = trimmed.hasPrefix("#") ? String(trimmed.dropFirst()) : trimmed
        guard [6, 8].contains(digits.count), let parsed = UInt32(digits, radix: 16) else {
            invalid = true; return
        }
        store.setColor(role, argb: digits.count == 6 ? parsed | 0xff000000 : parsed)
        refreshHex()
    }
}

private struct UIKitColorPreview: View {
    @ObservedObject var store: UIKitStore

    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Eclipse Precision").font(.largeTitle.weight(.semibold))
            Text("Real surface artwork · live semantic color preview")
                .font(.callout).foregroundStyle(.secondary)
            ZStack(alignment: .topLeading) {
                PrecisionSurface(kind: .windowFill, material: material(kind: .windowFill), projectRoot: store.projectRoot, atlasData: atlas)
                PrecisionSurface(kind: .frame, material: material(kind: .frame, focus: 255), projectRoot: store.projectRoot, atlasData: atlas)
                VStack(alignment: .leading, spacing: 22) {
                    Text("InfinityOS Window").font(.headline)
                    Divider()
                    Text("Shared panels, controls and focus states").font(.title3.weight(.semibold))
                    Text("Colors preserve the authored glass highlights, antialiasing and independent edge transparency.")
                        .font(.body).fixedSize(horizontal: false, vertical: true)
                        .foregroundStyle(studioColor(store.document.colors[5]).color)
                    ZStack(alignment: .leading) {
                        PrecisionSurface(kind: .field, material: material(kind: .field), projectRoot: store.projectRoot, atlasData: atlas)
                        Text("Text input").padding(.horizontal, 16).foregroundStyle(.secondary)
                    }.frame(height: 48)
                    HStack(spacing: 16) {
                        action("Continue", kind: .primary)
                        action("Cancel", kind: .secondary)
                    }
                }.padding(26)
            }.frame(height: 320)
            HStack(spacing: 16) {
                action("Normal", kind: .primary)
                action("Focused", kind: .primary, focus: 255)
                action("Pressed", kind: .primary, pressed: true)
            }
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 140))], spacing: 16) {
                ForEach(UIKitColorRole.allCases) { role in
                    VStack(alignment: .leading, spacing: 8) {
                        RoundedRectangle(cornerRadius: 8).fill(studioColor(store.document.colors[role.rawValue]).color)
                            .frame(height: 36).overlay(RoundedRectangle(cornerRadius: 8).stroke(.white.opacity(0.15)))
                        Text(role.title).font(.caption)
                        Text(String(format: "#%08X", store.document.colors[role.rawValue]))
                            .font(.caption2.monospaced()).foregroundStyle(.secondary)
                    }
                }
            }
        }.foregroundStyle(studioColor(store.document.colors[4]).color)
    }

    // ------------------------=
    // FUNC: material
    // DESC: Resolves the same ordered skin tokens used by the kernel's shared panel compositor.
    // ------------------=
    private func material(kind: PrecisionSurfaceKind, focus: UInt8 = 0, pressed: Bool = false) -> PrecisionMaterial {
        PrecisionMaterial(fill: studioColor(store.document.colors[kind == .primary ? 6 : 1]),
            edge: studioColor(store.document.colors[3]), focus: focus, pressed: pressed,
            style: UIKitSurfaceStyle.resolved(for: kind, styles: store.document.styles))
    }

    private var atlas: Data? {
        store.previewAtlas(target: "assets/ui-design-kit/default/eclipse-precision-v1.rgba")
    }

    // ------------------------=
    // FUNC: action
    // DESC: Shows exact normal/focus/pressed artwork with labels outside the underlying atlas pixels.
    // ------------------=
    private func action(_ title: String, kind: PrecisionSurfaceKind,
                        focus: UInt8 = 0, pressed: Bool = false) -> some View {
        ZStack {
            PrecisionSurface(kind: kind, material: material(kind: kind, focus: focus, pressed: pressed),
                projectRoot: store.projectRoot, atlasData: atlas)
            Text(title).font(.body.weight(.semibold))
        }.frame(height: 48)
    }
}

// ------------------------=
// FUNC: studioColor
// DESC: Converts an exact runtime ARGB color to the existing editor's straight-alpha channel representation.
// ------------------=
func studioColor(_ argb: UInt32) -> StudioColor {
    StudioColor(red: UInt8((argb >> 16) & 255), green: UInt8((argb >> 8) & 255),
        blue: UInt8(argb & 255), alpha: UInt8(argb >> 24))
}
