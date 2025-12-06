// swift-tools-version: 6.2
import PackageDescription

let package = Package(
    name: "aisuggest",
    platforms: [
        .macOS(.v26)
    ],
    products: [
        .executable(
            name: "aisuggest",
            targets: ["aisuggest"]
        )
    ],
    targets: [
        .executableTarget(
            name: "aisuggest",
            path: "Sources/aisuggest"
        )
    ]
)
