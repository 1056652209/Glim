import Cocoa
import ImageIO
import Quartz

// ImageIO retains a file-backed source; only the requested page is decoded.
final class ImagePreview: NSObject {
    let view = NSView()
    private let scroll = NSScrollView()
    private let imageView = NSImageView()
    private let status = NSTextField(labelWithString: "")
    private let previous = NSButton(title: L("Previous page"), target: nil, action: nil)
    private let next = NSButton(title: L("Next page"), target: nil, action: nil)
    private let resolution = NSButton(title: L("Original resolution"), target: nil, action: nil)
    private let url: URL
    private var page = 0
    private var count = 0
    private var original = false
    private var generation = 0
    private var task: DispatchWorkItem?
    private var closed = false
    private var quickLook: QLPreviewView?
    init(url: URL) {
        self.url = url
        super.init()
        let stack = NSStackView(); stack.orientation = .vertical; stack.spacing = 6
        stack.translatesAutoresizingMaskIntoConstraints = false; view.addSubview(stack)
        NSLayoutConstraint.activate([stack.leadingAnchor.constraint(equalTo: view.leadingAnchor), stack.trailingAnchor.constraint(equalTo: view.trailingAnchor), stack.topAnchor.constraint(equalTo: view.topAnchor), stack.bottomAnchor.constraint(equalTo: view.bottomAnchor)])
        let toolbar = NSStackView()
        previous.target = self; previous.action = #selector(previousPage)
        next.target = self; next.action = #selector(nextPage)
        resolution.target = self; resolution.action = #selector(toggleResolution)
        [previous, status, next, resolution].forEach { toolbar.addArrangedSubview($0) }
        stack.addArrangedSubview(toolbar)
        imageView.imageScaling = .scaleAxesIndependently
        scroll.hasVerticalScroller = true; scroll.hasHorizontalScroller = true
        scroll.allowsMagnification = true; scroll.minMagnification = 0.00001; scroll.maxMagnification = 8
        scroll.documentView = imageView
        stack.addArrangedSubview(scroll)
        scroll.widthAnchor.constraint(equalTo: stack.widthAnchor).isActive = true
        scroll.heightAnchor.constraint(greaterThanOrEqualToConstant: 100).isActive = true
        load()
    }
    @objc private func previousPage() { if page > 0 { page -= 1; load() } }
    @objc private func nextPage() { if page + 1 < count { page += 1; load() } }
    @objc private func toggleResolution() { original.toggle(); load() }
    private func controls(_ busy: Bool) {
        previous.isEnabled = !busy && page > 0; next.isEnabled = !busy && page + 1 < count
        resolution.isEnabled = !busy && quickLook == nil
        resolution.title = original ? L("Fit image") : L("Original resolution")
    }
    private func load() {
        task?.cancel(); generation += 1
        let generation = generation, page = page, original = original, url = url
        status.stringValue = L("Loading preview…"); controls(true)
        let work = DispatchWorkItem { [weak self] in
            let source = CGImageSourceCreateWithURL(url as CFURL, [kCGImageSourceShouldCache: false] as CFDictionary)
            let count = source.map(CGImageSourceGetCount) ?? 0
            let props = source.flatMap { CGImageSourceCopyPropertiesAtIndex($0, page, nil) as? [CFString: Any] }
            let width = (props?[kCGImagePropertyPixelWidth] as? NSNumber)?.doubleValue ?? 1
            let height = (props?[kCGImagePropertyPixelHeight] as? NSNumber)?.doubleValue ?? 1
            let edge = original ? max(width, height) : Double(ReaderLimits.IMAGE_PREVIEW_EDGE)
            let image = source.flatMap { CGImageSourceCreateThumbnailAtIndex($0, page, [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceCreateThumbnailWithTransform: true, kCGImageSourceThumbnailMaxPixelSize: edge, kCGImageSourceShouldCacheImmediately: true] as CFDictionary) }
            DispatchQueue.main.async { [weak self] in
                guard let self = self, !self.closed, self.generation == generation else { return }
                self.task = nil
                guard let image = image else {
                    if source == nil, url.pathExtension.lowercased() == "svg", let preview = QLPreviewView(frame: self.scroll.bounds, style: .normal) {
                        preview.autoresizingMask = [.width, .height]; preview.autostarts = true
                        preview.previewItem = url as NSURL
                        self.quickLook = preview; self.scroll.contentView.addSubview(preview)
                        self.status.stringValue = L("Read-only preview")
                    } else { self.status.stringValue = L("Cannot open image") }
                    self.controls(false); return
                }
                self.count = count
                let scale = max(width, height) / Double(max(image.width, image.height))
                let size = NSSize(width: Double(image.width) * scale, height: Double(image.height) * scale)
                self.imageView.image = NSImage(cgImage: image, size: size)
                self.imageView.frame = NSRect(origin: .zero, size: size)
                let viewport = self.scroll.contentView.bounds.size
                let fit = min(viewport.width / max(1, size.width), viewport.height / max(1, size.height), 1)
                self.scroll.magnification = original ? 1 : max(self.scroll.minMagnification, fit)
                self.status.stringValue = "\(page + 1) / \(count) · \(Int(width)) × \(Int(height))"
                self.controls(false)
            }
        }
        task = work; DispatchQueue.global(qos: .userInitiated).async(execute: work)
    }
    func pause() { task?.cancel(); task = nil; generation += 1; imageView.image = nil }
    func resume() { if !closed && task == nil && imageView.image == nil && quickLook == nil { load() } }
    func close() { closed = true; pause(); quickLook?.previewItem = nil; quickLook?.close(); quickLook = nil }
}
