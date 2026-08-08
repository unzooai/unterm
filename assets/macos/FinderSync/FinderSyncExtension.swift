import Cocoa
import FinderSync

@objc(FinderSyncExtension)
final class FinderSyncExtension: FIFinderSync {
    override init() {
        super.init()

        // Finder Sync requires at least one observed directory before Finder
        // asks the extension for contextual menus. We used to watch "/" as
        // a catch-all but macOS 26 (Tahoe) silently stopped honouring it —
        // the extension loaded, the process spawned, but `menu(for:)` was
        // never called, so the right-click menu never appeared anywhere.
        // Listing common roots explicitly works on Tahoe; subfolders are
        // inherited automatically (right-click in ~/Desktop/foo/bar still
        // fires because ~/Desktop is under NSHomeDirectory()).
        let roots: Set<URL> = [
            URL(fileURLWithPath: NSHomeDirectory()),
            URL(fileURLWithPath: "/Volumes"),
            URL(fileURLWithPath: "/Users"),
            URL(fileURLWithPath: "/Applications"),
            URL(fileURLWithPath: "/private/tmp"),
            URL(fileURLWithPath: "/tmp"),
        ]
        FIFinderSyncController.default().directoryURLs = roots
    }

    override var toolbarItemName: String {
        return "Unterm"
    }

    override var toolbarItemToolTip: String {
        return "Open the selected folder in Unterm"
    }

    override var toolbarItemImage: NSImage {
        return NSImage(named: "NSComputer") ?? NSImage()
    }

    override func menu(for menuKind: FIMenuKind) -> NSMenu? {
        switch menuKind {
        case .contextualMenuForItems, .contextualMenuForContainer, .toolbarItemMenu:
            let menu = NSMenu(title: "Unterm")
            let item = NSMenuItem(
                title: "Open in Unterm",
                action: #selector(openInUnterm(_:)),
                keyEquivalent: ""
            )
            item.target = self
            menu.addItem(item)
            return menu
        default:
            return nil
        }
    }

    @objc private func openInUnterm(_ sender: Any?) {
        let controller = FIFinderSyncController.default()
        let selected = controller.selectedItemURLs() ?? []
        let url = selected.first ?? controller.targetedURL()
        NSLog("UntermFinderSync: menu clicked, selected=%d, target=%@",
              selected.count, url?.path ?? "nil")

        guard let targetURL = url else {
            openUntermWithoutDocument()
            return
        }

        openUnterm(with: [targetURL])
    }

    private func openUntermWithoutDocument() {
        let configuration = NSWorkspace.OpenConfiguration()
        NSWorkspace.shared.openApplication(
            at: containingAppURL(),
            configuration: configuration,
            completionHandler: nil
        )
    }

    private func openUnterm(with urls: [URL]) {
        let appURL = containingAppURL()
        NSLog("UntermFinderSync: opening %@ with %@", urls.first?.path ?? "?", appURL.path)
        let configuration = NSWorkspace.OpenConfiguration()
        NSWorkspace.shared.open(
            urls,
            withApplicationAt: appURL,
            configuration: configuration
        ) { app, error in
            // The silent path is how "clicked, nothing happened" stays
            // undiagnosable; the error, if any, is the whole story.
            if let error = error {
                NSLog("UntermFinderSync: open FAILED: %@", error.localizedDescription)
            } else {
                NSLog("UntermFinderSync: open ok, app pid=%d", app?.processIdentifier ?? -1)
            }
        }
    }

    private func containingAppURL() -> URL {
        // .../Unterm.app/Contents/PlugIns/UntermFinderSync.appex
        return Bundle.main.bundleURL
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
    }
}
