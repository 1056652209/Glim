import Cocoa

// Opaque handles are retained by one Rust reader, and used only on the UI thread.
@_cdecl("glim_preview_create")
func glimPreviewCreate(_ bytes: UnsafePointer<UInt8>, _ length: Int) -> UnsafeMutableRawPointer {
    precondition(Thread.isMainThread)
    let preview = NativePreviewController()
    preview.host.isHidden = true
    let data = Data(bytes: bytes, count: length)
    // AppKit/PDFKit/Quick Look loading occurs outside GPUI render/layout callbacks.
    DispatchQueue.main.async { [weak preview] in
        guard let preview = preview, !preview.closed else { return }
        do {
            try preview.load(JSONDecoder().decode(Playlist.self, from: data))
        } catch {
            preview.host.subviews.forEach { $0.removeFromSuperview() }
            let label = NSTextField(wrappingLabelWithString: error.localizedDescription)
            label.frame = preview.host.bounds.insetBy(dx: 24, dy: 24)
            label.autoresizingMask = [.width, .height]
            preview.host.addSubview(label)
        }
    }
    return Unmanaged.passRetained(preview).toOpaque()
}

@_cdecl("glim_preview_layout")
func glimPreviewLayout(_ handle: UnsafeMutableRawPointer, _ parentPointer: UnsafeMutableRawPointer,
                       _ x: Double, _ y: Double, _ width: Double, _ height: Double) {
    precondition(Thread.isMainThread)
    let preview = Unmanaged<NativePreviewController>.fromOpaque(handle).takeUnretainedValue()
    let gpuiView = Unmanaged<NSView>.fromOpaque(parentPointer).takeUnretainedValue()
    // Share the content view with GPUI. Register native accessibility children
    // at the window level because AccessKit owns the content view's AX tree.
    let parent = gpuiView.superview ?? gpuiView
    if preview.host.superview !== parent {
        preview.host.removeFromSuperview()
        parent.addSubview(preview.host)
        preview.host.gpuiView = gpuiView
        preview.host.nextResponder = gpuiView
    }
    // GPUI uses top-left logical pixels; AppKit views may use bottom-left points.
    let rect = NSRect(x: gpuiView.bounds.minX + x,
        y: gpuiView.isFlipped ? gpuiView.bounds.minY + y : gpuiView.bounds.maxY - y - height,
        width: max(0, width), height: max(0, height))
    preview.host.frame = parent.convert(rect, from: gpuiView)
    let hidden = width <= 0 || height <= 0
    if preview.host.isHidden != hidden {
        preview.host.isHidden = hidden
        preview.host.updateAccessibility()
    }
    preview.resumePendingPlayback()
}

@_cdecl("glim_preview_hide")
func glimPreviewHide(_ handle: UnsafeMutableRawPointer) {
    precondition(Thread.isMainThread)
    let preview = Unmanaged<NativePreviewController>.fromOpaque(handle).takeUnretainedValue()
    guard !preview.host.isHidden else { return }
    if let responder = preview.host.window?.firstResponder as? NSView,
       responder.isDescendant(of: preview.host) {
        preview.host.window?.makeFirstResponder(preview.host.gpuiView)
    }
    preview.host.isHidden = true
    preview.host.updateAccessibility()
    preview.pendingPlayback = false
    preview.player.pause()
}

@_cdecl("glim_preview_release")
func glimPreviewRelease(_ handle: UnsafeMutableRawPointer) {
    precondition(Thread.isMainThread)
    let preview = Unmanaged<NativePreviewController>.fromOpaque(handle).takeRetainedValue()
    preview.close()
}
