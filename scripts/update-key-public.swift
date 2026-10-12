import Foundation
import CryptoKit
// Never print the private seed or place it in an argument/environment value.
let data = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
guard let text = String(data: data, encoding: .utf8),
      let seed = Data(base64Encoded: text.trimmingCharacters(in: .whitespacesAndNewlines)), seed.count == 32 else {
    fatalError("Use a Sparkle private key exported in the 32-byte seed format")
}
let key = try Curve25519.Signing.PrivateKey(rawRepresentation: seed)
print(key.publicKey.rawRepresentation.base64EncodedString())
