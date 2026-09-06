// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "InfinityInstallerStudio",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "InfinityInstallerStudio", targets: ["InfinityInstallerStudio"]),
    ],
    targets: [
        .executableTarget(
            name: "InfinityInstallerStudio",
            resources: [.process("Resources")]
        ),
        .testTarget(
            name: "InfinityInstallerStudioTests",
            dependencies: ["InfinityInstallerStudio"]
        ),
    ]
)
