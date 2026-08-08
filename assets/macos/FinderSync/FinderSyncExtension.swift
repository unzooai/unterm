import Cocoa
import FinderSync

/// The unified log swallows this appex's NSLog output whole — the process
/// runs, the instance lives, and `log show` returns nothing. A file in the
/// sandbox container is the one channel nothing can redact:
/// ~/Library/Containers/ai.unzoo.unterm.finder-sync/Data/fs-trace.log
private func trace(_ message: String) {
    NSLog("UntermFinderSync: %@", message)
    let stamp = ISO8601DateFormatter().string(from: Date())
    let line = "\(stamp) \(message)\n"
    let path = NSHomeDirectory() + "/fs-trace.log"
    if let handle = FileHandle(forWritingAtPath: path) {
        handle.seekToEndOfFile()
        if let data = line.data(using: .utf8) {
            handle.write(data)
        }
        handle.closeFile()
    } else {
        try? line.write(toFile: path, atomically: true, encoding: .utf8)
    }
}

@objc(FinderSyncExtension)
final class FinderSyncExtension: FIFinderSync {
    override init() {
        super.init()

        // Finder Sync requires at least one observed directory before Finder
        // asks the extension for contextual menus. We used to watch "/" as
        // a catch-all but macOS 26 (Tahoe) silently stopped honouring it —
        // the extension loaded, the process spawned, but `menu(for:)` was
        // never called, so the right-click menu never appeared anywhere.
        // Watching "/Volumes" turned out to be the same trap one level down:
        // observation never crosses into a mounted volume, so a window on a
        // second disk got no menu while the home folder did. Every mounted
        // volume gets watched by its own root, refreshed as disks come and go.
        refreshRoots(reason: "init")
        let center = NSWorkspace.shared.notificationCenter
        for name in [NSWorkspace.didMountNotification, NSWorkspace.didUnmountNotification] {
            center.addObserver(
                forName: name, object: nil, queue: nil
            ) { [weak self] _ in
                self?.refreshRoots(reason: "volumes changed")
            }
        }
    }

    private func refreshRoots(reason: String) {
        var roots: Set<URL> = [
            URL(fileURLWithPath: "/Users"),
            URL(fileURLWithPath: "/Applications"),
            URL(fileURLWithPath: "/private/tmp"),
            URL(fileURLWithPath: "/tmp"),
        ]
        let mounted = FileManager.default.mountedVolumeURLs(
            includingResourceValuesForKeys: nil,
            options: [.skipHiddenVolumes]
        ) ?? []
        // "/" stays out: Tahoe ignores it, and listing it would only hide
        // the volumes that actually need their own entry.
        roots.formUnion(mounted.filter { $0.path != "/" })
        FIFinderSyncController.default().directoryURLs = roots
        trace("watching \(roots.count) roots (\(reason)): \(roots.map(\.path).sorted().joined(separator: ", "))")
    }

    // Whether these ever fire tells us if Finder considers the browsed
    // folder to be inside our watched roots — the question the missing
    // context-menu item hinges on.
    override func beginObservingDirectory(at url: URL) {
        trace("begin observing \(url.path)")
    }

    override func endObservingDirectory(at url: URL) {
        trace("end observing \(url.path)")
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
        trace("menu requested, kind=\(menuKind.rawValue)")
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
        trace("menu clicked, selected=\(selected.count), target=\(url?.path ?? "nil")")

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
        trace("opening \(urls.first?.path ?? "?") with \(appURL.path)")
        let configuration = NSWorkspace.OpenConfiguration()
        NSWorkspace.shared.open(
            urls,
            withApplicationAt: appURL,
            configuration: configuration
        ) { app, error in
            // The silent path is how "clicked, nothing happened" stays
            // undiagnosable; the error, if any, is the whole story.
            if let error = error {
                trace("open FAILED: \(error.localizedDescription)")
                self.openUntermFallback(with: urls)
            } else {
                trace("open ok, app pid=\(app?.processIdentifier ?? -1)")
            }
        }
    }

    /// On macOS 26 the appex sandbox refuses open-with-application for
    /// document URLs ("杂项错误"). Two more roads, each attempted only when
    /// the previous one failed and each leaving its verdict in the trace:
    /// the older single-file LaunchServices API, then a unterm:// deep link
    /// the app resolves itself (v0.63.3+ registers the scheme).
    private func openUntermFallback(with urls: [URL]) {
        guard let target = urls.first else { return }
        if NSWorkspace.shared.openFile(target.path, withApplication: "Unterm") {
            trace("openFile fallback ok for \(target.path)")
            return
        }
        trace("openFile fallback refused for \(target.path)")
        var components = URLComponents()
        components.scheme = "unterm"
        components.host = "open"
        components.queryItems = [URLQueryItem(name: "path", value: target.path)]
        guard let link = components.url else {
            trace("deep link build failed for \(target.path)")
            return
        }
        NSWorkspace.shared.open(link, configuration: NSWorkspace.OpenConfiguration()) { app, error in
            if let error = error {
                trace("deep link FAILED: \(error.localizedDescription)")
            } else {
                trace("deep link ok, app pid=\(app?.processIdentifier ?? -1)")
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
