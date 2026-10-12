import Cocoa
import CryptoKit

@main
struct UpdaterHost {
    static func main() {
        if CommandLine.arguments.count == 3 && CommandLine.arguments[1] == "--test-key" {
            let directory = URL(fileURLWithPath: CommandLine.arguments[2])
            let key = Curve25519.Signing.PrivateKey()
            try! key.rawRepresentation.base64EncodedString().write(to: directory.appendingPathComponent("private-key"), atomically: true, encoding: .utf8)
            try! FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: directory.appendingPathComponent("private-key").path)
            try! key.publicKey.rawRepresentation.base64EncodedString().write(to: directory.appendingPathComponent("public-key"), atomically: true, encoding: .utf8)
            return
        }
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        let delegate = HostDelegate()
        app.delegate = delegate
        app.run()
    }
}
final class HostDelegate: NSObject, NSApplicationDelegate {
    var updater: GlimUpdater?
    var timer: Timer?
    var checkedQuitGuard = false
    func applicationDidFinishLaunching(_ notification: Notification) {
        let bundle = Bundle.main
        let marker = bundle.object(forInfoDictionaryKey: "GlimUpdaterTestMarker") as! String
        if bundle.object(forInfoDictionaryKey: "CFBundleVersion") as? String == "101" {
            try! "installed-and-relaunched".write(toFile: marker, atomically: true, encoding: .utf8)
            UserDefaults.standard.removePersistentDomain(forName: bundle.bundleIdentifier!)
            NSApp.terminate(nil)
            return
        }
        let updater = GlimUpdater()
        self.updater = updater
        updater.start()
        let automatic = bundle.object(forInfoDictionaryKey: "GlimUpdaterTestAutomatic") as? Bool == true
        if automatic {
            updater.command("automatic")
            updater.updater?.checkForUpdatesInBackground()
        } else {
            updater.command("check")
        }
        timer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { _ in
            switch updater.phase {
            case "available": updater.command("download")
            case "ready":
                if !self.checkedQuitGuard {
                    self.checkedQuitGuard = true
                    NSApp.terminate(nil)
                    precondition(updater.needsQuitApproval, "A system quit must pass through the application guard")
                    updater.command("quit-request-received")
                }
                if automatic {
                    updater.command("quit-approved")
                    NSApp.terminate(nil)
                } else {
                    updater.command("install-approved")
                }
            case "error", "disabled":
                var codes: [Int] = []
                var error = updater.lastError
                while let current = error, codes.count < 8 {
                    codes.append(current.code)
                    error = current.userInfo[NSUnderlyingErrorKey] as? NSError
                }
                let data = try! JSONSerialization.data(withJSONObject: ["message": updater.message, "codes": codes])
                try? data.write(to: URL(fileURLWithPath: marker + ".error"))
                NSApp.terminate(nil)
            default: break
            }
        }
    }
}
