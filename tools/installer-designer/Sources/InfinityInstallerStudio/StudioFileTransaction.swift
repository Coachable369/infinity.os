import Darwin
import Foundation

struct StudioFileWrite {
    let url: URL
    let data: Data
}

enum StudioFileTransaction {
    private struct Prepared {
        let target: URL
        let staged: URL
        let backup: URL?
    }

    // ------------------------=
    // FUNC: commit
    // DESC: Stages and validates a complete repository-local save before publication, restoring earlier files if any publication fails.
    // ------------------=
    static func commit(_ writes: [StudioFileWrite], root: URL,
                       beforePublish: (Int, URL) throws -> Void = { _, _ in }) throws {
        let manager = FileManager.default
        let root = root.standardizedFileURL.resolvingSymlinksInPath()
        guard !writes.isEmpty else { return }
        let writes = writes.map { StudioFileWrite(url: $0.url.standardizedFileURL, data: $0.data) }
        var targets = Set<URL>()
        for write in writes {
            let target = write.url.standardizedFileURL
            guard target.path.hasPrefix(root.path + "/"),
                  try hasNoSymlinks(target, root: root),
                  targets.insert(target).inserted else {
                throw UIKitIssue.invalid("Save destinations must be distinct files inside the project, without symlinks")
            }
            if manager.fileExists(atPath: target.path) {
                let values = try target.resourceValues(forKeys: [.isRegularFileKey])
                guard values.isRegularFile == true else { throw UIKitIssue.invalid("Save destination is not a regular file: \(target.lastPathComponent)") }
            }
        }
        let scratch = root.appending(path: "build/tmp/studio-save-\(UUID().uuidString)")
        guard try hasNoSymlinks(scratch, root: root) else {
            throw UIKitIssue.invalid("Save staging must remain inside the project")
        }
        try manager.createDirectory(at: scratch, withIntermediateDirectories: true)
        var retainRecovery = false
        defer { if !retainRecovery { try? manager.removeItem(at: scratch) } }
        var prepared: [Prepared] = []
        for (index, write) in writes.enumerated() {
            guard try hasNoSymlinks(write.url, root: root) else {
                throw UIKitIssue.invalid("Save destination changed during preparation")
            }
            let staged = scratch.appending(path: "\(index).new")
            try write.data.write(to: staged, options: .atomic)
            guard try Data(contentsOf: staged) == write.data else {
                throw UIKitIssue.invalid("Staged save verification failed")
            }
            var backup: URL?
            if manager.fileExists(atPath: write.url.path) {
                let original = scratch.appending(path: "\(index).original")
                try manager.copyItem(at: write.url, to: original)
                if let permissions = try manager.attributesOfItem(atPath: write.url.path)[.posixPermissions] {
                    try manager.setAttributes([.posixPermissions: permissions], ofItemAtPath: staged.path)
                }
                backup = original
            }
            try manager.createDirectory(at: write.url.deletingLastPathComponent(), withIntermediateDirectories: true)
            prepared.append(Prepared(target: write.url, staged: staged, backup: backup))
        }
        var published = 0
        do {
            for (index, item) in prepared.enumerated() {
                try beforePublish(index, item.target)
                guard try hasNoSymlinks(item.target, root: root) else {
                    throw UIKitIssue.invalid("Save destination changed during publication")
                }
                try replace(item.staged, at: item.target)
                published += 1
            }
        } catch {
            let originalError = error
            var failedRecovery: [String] = []
            for item in prepared.prefix(published).reversed() {
                do {
                    guard try hasNoSymlinks(item.target, root: root) else {
                        throw UIKitIssue.invalid("Recovery destination is no longer inside the project")
                    }
                    if let backup = item.backup { try replace(backup, at: item.target) }
                    else { try manager.removeItem(at: item.target) }
                } catch { failedRecovery.append(item.target.path) }
            }
            if !failedRecovery.isEmpty {
                retainRecovery = true
                throw UIKitIssue.invalid("Save failed and recovery needs attention for \(failedRecovery.joined(separator: ", ")). Originals retained at \(scratch.path). Cause: \(originalError.localizedDescription)")
            }
            throw originalError
        }
    }

    // ------------------------=
    // FUNC: hasNoSymlinks
    // DESC: Checks every existing path component, including a symlink parent of a not-yet-created output file.
    // ------------------=
    private static func hasNoSymlinks(_ target: URL, root: URL) throws -> Bool {
        var component = target
        while component.path != root.path {
            guard component.path.hasPrefix(root.path + "/") else { return false }
            // attributesOfItem does not follow a final symlink, unlike whole-path URL resolution
            // for destinations whose final component does not yet exist.
            do {
                let attributes = try FileManager.default.attributesOfItem(atPath: component.path)
                if attributes[.type] as? FileAttributeType == .typeSymbolicLink { return false }
            } catch let error as NSError {
                if error.domain != NSCocoaErrorDomain || error.code != NSFileReadNoSuchFileError {
                    throw error
                }
            }
            component.deleteLastPathComponent()
        }
        return true
    }

    // ------------------------=
    // FUNC: replace
    // DESC: Publishes one staged file by same-filesystem atomic rename without deleting the existing destination first.
    // ------------------=
    private static func replace(_ source: URL, at target: URL) throws {
        guard Darwin.rename(source.path, target.path) == 0 else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno), userInfo: [NSFilePathErrorKey: target.path])
        }
    }
}
