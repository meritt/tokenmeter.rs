import TokenMeterFFI

public enum TokenMeter {
    public static let version = String(cString: tokenmeter_version())

    public static let encoding = String(cString: tokenmeter_encoding())

    public static func prepare() {
        tokenmeter_prepare()
    }

    public static func count(_ text: String) -> Int {
        let input = Array(text.utf8)
        let tokens = input.withUnsafeBufferPointer { input in
            tokenmeter_count(input.baseAddress, input.count)
        }
        return Int(tokens)
    }

    public static func split(_ text: String, limit: Int = 200) throws(TokenMeterError) -> Split {
        guard limit >= 0 else {
            throw .invalidArgument
        }
        let input = Array(text.utf8)
        var out = tokenmeter_groups()
        input.withUnsafeBufferPointer { input in
            tokenmeter_split(input.baseAddress, input.count, limit, &out)
        }
        defer { tokenmeter_split_free(&out) }
        switch out.status {
        case UInt32(TOKENMETER_OK):
            break
        case UInt32(TOKENMETER_INVALID_ARGUMENT):
            throw .invalidArgument
        default:
            throw .internal
        }

        let pairs = UnsafeBufferPointer(start: out.groups, count: out.groups_len * 2)
        var groups: [Group] = []
        var start = 0
        for at in stride(from: 0, to: pairs.count, by: 2) {
            let end = Int(pairs[at])
            groups.append(Group(text: String(decoding: input[start..<end], as: UTF8.self), tokens: Int(pairs[at + 1])))
            start = end
        }
        return Split(tokens: Int(out.tokens), groups: groups, truncated: out.truncated != 0)
    }
}

public struct Split: Equatable, Sendable {
    public let tokens: Int
    public let groups: [Group]
    public let truncated: Bool
}

public struct Group: Equatable, Sendable {
    public let text: String
    public let tokens: Int
}

public enum TokenMeterError: Error, Equatable, Sendable {
    case invalidArgument
    case `internal`
}
