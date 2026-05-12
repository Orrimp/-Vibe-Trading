// scripts/orch_cursor_move.swift — move the macOS cursor to (x, y) and
// dispatch a CGEvent mouseMoved so the focused window's event loop
// receives a CursorMoved. Run via `swift orch_cursor_move.swift <x> <y>`.
//
// Why this file exists: in the chart-canvas-overhaul session I wrote
// `/tmp/orch-diag/{warp,hover}.swift` ad-hoc. Stashing the helper in
// `scripts/` makes it reusable and version-controlled.
//
// Why CGWarp + CGEvent (not osascript): CGWarpMouseCursorPosition is a
// Core Graphics primitive — no Accessibility, no Automation, no TCC
// authorization required on macOS as of Sonoma. Dispatching a real
// mouseMoved event via CGEvent ensures the iced window's input queue
// sees a CursorMoved event (a bare warp updates the cursor sprite but
// some apps don't track that without an explicit event).
//
// Exit 0 on dispatch; non-zero on usage error.

import Foundation
import CoreGraphics

let args = CommandLine.arguments
guard args.count >= 3 else {
    FileHandle.standardError.write("usage: swift orch_cursor_move.swift <x> <y>\n".data(using: .utf8)!)
    exit(2)
}

guard let x = Double(args[1]), let y = Double(args[2]) else {
    FileHandle.standardError.write("orch_cursor_move: x and y must be numeric\n".data(using: .utf8)!)
    exit(2)
}

let point = CGPoint(x: x, y: y)

// Dispatch a real mouseMoved event so the focused window's input loop
// receives a CursorMoved.
if let event = CGEvent(mouseEventSource: nil,
                      mouseType: .mouseMoved,
                      mouseCursorPosition: point,
                      mouseButton: .left) {
    event.post(tap: .cghidEventTap)
}

// Also warp so the visible cursor sprite catches up.
let err = CGWarpMouseCursorPosition(point)
CGAssociateMouseAndMouseCursorPosition(1)

print("cursor moved to (\(x), \(y)) — CGWarp rc=\(err.rawValue)")
