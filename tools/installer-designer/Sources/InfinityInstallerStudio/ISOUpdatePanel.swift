import SwiftUI
import AppKit

struct ISOUpdatePanel: View {
    @ObservedObject var store: TemplateStore
    @ObservedObject var updater: ISOUpdateService
    @State private var showDetails = false

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 12) {
                Image(systemName: "opticaldisc").font(.title2)
                    .foregroundStyle(InfinityUIKit.Palette.nativeAccent)
                VStack(alignment: .leading, spacing: 3) {
                    Text("Update test ISO").font(.headline)
                    Text("Save all sections, rebuild UI-linked kernels, and update this ISO.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                Spacer()
                Picker("Architecture", selection: $updater.target) {
                    Text("ARM64").tag(ISOArchitecture.aarch64)
                    Text("x86_64").tag(ISOArchitecture.x86_64)
                }.frame(width: 180).disabled(updater.isRunning)
                Button("Choose ISO…", action: updater.chooseISO).disabled(updater.isRunning)
                Button {
                    Task { await store.saveAndUpdateISO() }
                } label: {
                    Label("Save & Update ISO", systemImage: "arrow.up.doc")
                }
                .buttonStyle(InfinityStudioButtonStyle(emphasis: .primary))
                .disabled(updater.isRunning || updater.selectedISO == nil)
                .accessibilityIdentifier("studio.iso.update")
                if updater.isRunning {
                    Button(updater.isCancelling ? "Cancelling…" : "Cancel update", action: updater.cancel)
                        .disabled(updater.isCancelling)
                        .accessibilityIdentifier("studio.iso.cancel")
                }
                Button { store.showISOUpdate = false } label: { Image(systemName: "xmark") }
                    .help("Hide ISO update panel; running work continues")
            }
            if let iso = updater.selectedISO {
                Text(iso.path).font(.caption.monospaced()).foregroundStyle(.secondary)
                    .textSelection(.enabled).lineLimit(2)
            } else {
                Text("Choose an existing project ISO. Models, browser and toolchain payloads are reused.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            HStack(spacing: 10) {
                if updater.isRunning {
                    ProgressView().controlSize(.small)
                }
                Text(updater.status).font(.callout).textSelection(.enabled)
                Spacer()
                Button(showDetails ? "Hide details" : "Show details") { showDetails.toggle() }
                    .buttonStyle(.plain).font(.caption).foregroundStyle(.secondary)
            }
            if updater.isRunning {
                ProgressView(value: updater.progress, total: 1)
                    .tint(InfinityUIKit.Palette.nativeAccent)
                    .accessibilityLabel("ISO update progress")
            }
            if let error = updater.errorMessage {
                Text(error).font(.callout).foregroundStyle(.red).textSelection(.enabled)
            }
            if let result = updater.lastResult {
                HStack {
                    Label("Verified SHA-256: \(result.sha256)", systemImage: "checkmark.shield")
                        .font(.caption.monospaced()).textSelection(.enabled).lineLimit(2)
                    Spacer()
                    Button("Reveal ISO") {
                        NSWorkspace.shared.activateFileViewerSelecting([result.isoURL])
                    }
                }
            }
            if showDetails {
                ScrollView {
                    Text(updater.output.isEmpty ? "No update has been run." : updater.output)
                        .font(.caption.monospaced()).textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading).padding(10)
                }
                .frame(height: 110)
                .background(InfinityUIKit.Palette.nativeCanvas)
            }
            Text("The existing ISO is replaced only after verification. Restart your test VM from the updated ISO; installed systems are not changed.")
                .font(.caption).foregroundStyle(.secondary)
        }
        .buttonStyle(InfinityStudioButtonStyle(emphasis: .secondary))
        .padding(16)
        .background(InfinityUIKit.Palette.nativePanel)
    }
}
