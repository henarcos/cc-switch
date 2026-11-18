import AppKit
import UserNotifications

struct Notice: Decodable {
    let identifier: String
    let title: String
    let body: String
}
final class Delegate: NSObject, NSApplicationDelegate, UNUserNotificationCenterDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        let center = UNUserNotificationCenter.current()
        center.delegate = self
        let mode = CommandLine.arguments.dropFirst().first ?? "--authorize"
        if mode == "--authorize" {
            NSApp.activate(ignoringOtherApps: true)
            center.requestAuthorization(options: [.alert, .sound]) { granted, error in
                print(granted && error == nil ? "authorized" : "denied")
                exit(granted && error == nil ? 0 : 2)
            }
            return
        }
        center.getNotificationSettings { settings in
            let allowed = settings.authorizationStatus == .authorized || settings.authorizationStatus == .provisional
            if mode == "--status" {
                print(allowed ? "authorized" : "not-authorized:\(settings.authorizationStatus.rawValue)")
                exit(allowed ? 0 : 2)
            }
            guard allowed else { print("not-authorized"); exit(2) }
            do {
                let notice: Notice
                if mode == "--test" {
                    notice = Notice(identifier: "installation-test", title: "CC Switch · Resets extras", body: "Notificações ativadas. Os avisos identificarão a conta e o vencimento do reset extra.")
                } else {
                    notice = try JSONDecoder().decode(Notice.self, from: FileHandle.standardInput.readDataToEndOfFile())
                }
                let content = UNMutableNotificationContent()
                content.title = notice.title
                content.body = notice.body
                content.sound = .default
                center.add(UNNotificationRequest(identifier: notice.identifier, content: content, trigger: nil)) { error in
                    print(error == nil ? "accepted" : "delivery-error")
                    exit(error == nil ? 0 : 3)
                }
            } catch { print("invalid-notice"); exit(4) }
        }
    }
    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification, withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        completionHandler([.banner, .sound, .list])
    }
}
let app = NSApplication.shared
app.setActivationPolicy(.accessory)
let delegate = Delegate()
app.delegate = delegate
app.run()
