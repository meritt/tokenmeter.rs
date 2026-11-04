// swift-tools-version: 6.2
import Foundation
import PackageDescription

let release: (url: String, checksum: String)? = (url: "https://github.com/meritt/tokenmeter.rs/releases/download/v0.1.0/tokenmeter-v0.1.0-xcframework.zip", checksum: "e0ba7b419970d5cb9f804f01a7b0909117f137221f53987b67e12dfb9c9b7e78")

let local = "packages/swift/TokenMeterFFI.xcframework"
let ffi: Target
if FileManager.default.fileExists(atPath: Context.packageDirectory + "/" + local) {
    ffi = .binaryTarget(name: "TokenMeterFFI", path: local)
} else if let release {
    ffi = .binaryTarget(name: "TokenMeterFFI", url: release.url, checksum: release.checksum)
} else {
    fatalError("no released TokenMeterFFI yet: build \(local) with `cargo xtask xcframework`")
}

let package = Package(
    name: "TokenMeter",
    platforms: [.macOS(.v26)],
    products: [
        .library(name: "TokenMeter", targets: ["TokenMeter"]),
    ],
    targets: [
        ffi,
        .target(name: "TokenMeter", dependencies: ["TokenMeterFFI"], path: "packages/swift/Sources/TokenMeter"),
        .testTarget(name: "TokenMeterTests", dependencies: ["TokenMeter"], path: "packages/swift/Tests/TokenMeterTests"),
    ]
)
