import Foundation
import Testing

@testable import TokenMeter

private let corpusURL: URL = {
    if let path = ProcessInfo.processInfo.environment["TOKENMETER_CORPUS"] {
        return URL(fileURLWithPath: path)
    }
    return URL(fileURLWithPath: #filePath)
        .deletingLastPathComponent()
        .appendingPathComponent("../../../../testdata/o200k.tsv")
        .standardized
}()

private func corpus() throws -> [(text: String, tokens: Int)] {
    try String(contentsOf: corpusURL, encoding: .utf8)
        .split(separator: "\n")
        .map { line in
            let fields = line.split(separator: "\t", omittingEmptySubsequences: false)
            try #require(fields.count == 2)
            let hex = Array(fields[1].utf8)
            let bytes = try stride(from: 0, to: hex.count, by: 2).map { at in
                try #require(UInt8(String(decoding: hex[at..<at + 2], as: UTF8.self), radix: 16))
            }
            let text = try #require(String(validating: bytes, as: UTF8.self))
            return (text, fields[0].isEmpty ? 0 : fields[0].split(separator: ",").count)
        }
}

@Test func countsTheCorpusAsTiktoken() throws {
    let cases = try corpus()

    #expect(!cases.isEmpty)
    for (text, tokens) in cases {
        #expect(TokenMeter.count(text) == tokens, "\(text.debugDescription)")
    }
}

@Test func countsAPhraseFromTheSite() {
    #expect(TokenMeter.encoding == "o200k_base")
    #expect(TokenMeter.count("Пользователи — это наше всё.") == 9)
    #expect(TokenMeter.count("") == 0)
}

@Test func splitsIntoGroupsThatJoinIntoTheText() throws {
    let text = "👩🏽‍💻 é 🇷🇺 ไทย\r\nfn main() {}"
    let split = try TokenMeter.split(text)

    #expect(split.groups.map(\.text).joined() == text)
    #expect(split.tokens == TokenMeter.count(text))
    #expect(!split.truncated)
    #expect(split.groups.contains { $0.text.contains("👩🏽‍💻") })
}

@Test func stopsAtTheLimit() throws {
    let split = try TokenMeter.split(String(repeating: "слово ", count: 1000), limit: 200)

    #expect(split.groups.count == 200)
    #expect(split.truncated)
}

@Test func refusesANegativeLimit() {
    #expect(throws: TokenMeterError.invalidArgument) { try TokenMeter.split("a", limit: -1) }
}
