import Cocoa
import FinderSync

@objc(FinderSyncExtension)
final class FinderSyncExtension: FIFinderSync {
    override init() {
        super.init()

        // Finder Sync requires at least one observed directory before Finder
        // asks the extension for contextual menus. Watching "/" makes the
        // menu available throughout Finder without adding badges.
        FIFinderSyncController.default().directoryURLs = [URL(fileURLWithPath: "/")]
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
        let configuration = NSWorkspace.OpenConfiguration()
        NSWorkspace.shared.open(
            urls,
            withApplicationAt: containingAppURL(),
            configuration: configuration,
            completionHandler: nil
        )
    }

    private func containingAppURL() -> URL {
        // .../Unterm.app/Contents/PlugIns/UntermFinderSync.appex
        return Bundle.main.bundleURL
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
    }
}
