import Foundation

enum ContentAwareAtomicWriter {
    // ------------------------=
    // FUNC: write
    // DESC: Preserves identical output bytes, inode and modification time; publishes changed compiler output by atomic replacement only.
    // ------------------=
    @discardableResult static func write(_ data: Data, to destination: URL) throws -> Bool {
        if FileManager.default.fileExists(atPath: destination.path),
           try Data(contentsOf: destination) == data {
            return false
        }
        try data.write(to: destination, options: .atomic)
        return true
    }
}
