import AppKit
import ApplicationServices

enum DriverFailure: Error, CustomStringConvertible {
  case message(String)
  var description: String {
    switch self { case .message(let value): return value }
  }
}

func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success else { return nil }
  return value
}

func stringAttribute(_ element: AXUIElement, _ name: String) -> String {
  attribute(element, name) as? String ?? ""
}

func descendants(_ root: AXUIElement) -> [AXUIElement] {
  var queue = [root]
  var result: [AXUIElement] = []
  while !queue.isEmpty && result.count < 2_000 {
    let element = queue.removeFirst()
    result.append(element)
    if let children = attribute(element, kAXChildrenAttribute) as? [AXUIElement] {
      queue.append(contentsOf: children)
    }
  }
  return result
}

func namedElement(_ root: AXUIElement, role: String, name: String) -> AXUIElement? {
  descendants(root).first { element in
    stringAttribute(element, kAXRoleAttribute) == role
      && [kAXTitleAttribute, kAXDescriptionAttribute, kAXHelpAttribute, kAXIdentifierAttribute]
        .map { stringAttribute(element, $0) }
        .contains(name)
  }
}

func namedElement(_ root: AXUIElement, name: String) -> AXUIElement? {
  descendants(root).first { element in
    [kAXTitleAttribute, kAXDescriptionAttribute, kAXHelpAttribute, kAXIdentifierAttribute]
      .map { stringAttribute(element, $0) }
      .contains(name)
  }
}

func waitForElement(_ root: AXUIElement, role: String, name: String, timeout: TimeInterval = 10) throws -> AXUIElement {
  let deadline = Date().addingTimeInterval(timeout)
  repeat {
    if let element = namedElement(root, role: role, name: name) { return element }
    RunLoop.current.run(until: Date().addingTimeInterval(0.1))
  } while Date() < deadline
  throw DriverFailure.message("Accessibility element missing: \(role) \(name)")
}

func runningApp() throws -> NSRunningApplication {
  guard let app = NSRunningApplication.runningApplications(withBundleIdentifier: "ai.desktoplab.desktop").first else {
    throw DriverFailure.message("DesktopLab process is not running")
  }
  return app
}

func appElement(_ app: NSRunningApplication) -> AXUIElement {
  AXUIElementCreateApplication(app.processIdentifier)
}

func activate(_ app: NSRunningApplication) throws {
  app.activate(options: [.activateAllWindows, .activateIgnoringOtherApps])
  let deadline = Date().addingTimeInterval(5)
  repeat {
    if app.isActive { return }
    RunLoop.current.run(until: Date().addingTimeInterval(0.05))
  } while Date() < deadline
  throw DriverFailure.message("DesktopLab did not become active")
}

func focus(_ element: AXUIElement, label: String) throws {
  let result = AXUIElementSetAttributeValue(element, kAXFocusedAttribute as CFString, kCFBooleanTrue)
  guard result == .success else { throw DriverFailure.message("Accessibility focus failed for \(label): \(result.rawValue)") }
  let deadline = Date().addingTimeInterval(5)
  repeat {
    if (attribute(element, kAXFocusedAttribute) as? Bool) == true { return }
    RunLoop.current.run(until: Date().addingTimeInterval(0.05))
  } while Date() < deadline
  throw DriverFailure.message("Accessibility focus did not reach \(label)")
}

func perform(_ action: String, on element: AXUIElement, label: String) throws {
  let result = AXUIElementPerformAction(element, action as CFString)
  guard result == .success else { throw DriverFailure.message("Accessibility action failed for \(label): \(result.rawValue)") }
}

func typeText(_ app: NSRunningApplication, value: String) throws {
  for character in value {
    let units = Array(String(character).utf16)
    guard let down = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: true),
          let up = CGEvent(keyboardEventSource: nil, virtualKey: 0, keyDown: false) else {
      throw DriverFailure.message("text event creation failed")
    }
    units.withUnsafeBufferPointer { buffer in
      down.keyboardSetUnicodeString(stringLength: buffer.count, unicodeString: buffer.baseAddress)
      up.keyboardSetUnicodeString(stringLength: buffer.count, unicodeString: buffer.baseAddress)
    }
    down.postToPid(app.processIdentifier)
    up.postToPid(app.processIdentifier)
  }
}

func visualInspect(_ root: AXUIElement) throws {
  guard let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement], let window = windows.first else {
    throw DriverFailure.message("DesktopLab window is unavailable")
  }
  let parts = try windowBounds(root).split(separator: ",").compactMap { Int($0) }
  guard parts.count == 4 else { throw DriverFailure.message("DesktopLab window bounds are invalid") }
  let screen = NSScreen.main?.frame.size ?? .zero
  let payload: [String: Any] = ["viewport": ["width": Int(screen.width), "height": Int(screen.height)], "nodes": [["id": "front-window", "name": stringAttribute(window, kAXTitleAttribute), "bounds": ["x": parts[0], "y": parts[1], "width": parts[2], "height": parts[3]], "exclusive": false]]]
  let data = try JSONSerialization.data(withJSONObject: payload)
  print(String(data: data, encoding: .utf8) ?? "{}")
}

func diagnostics(_ root: AXUIElement, app: NSRunningApplication) throws {
  let elements = descendants(root)
  let buttons = elements.filter { stringAttribute($0, kAXRoleAttribute) == "AXButton" }.compactMap { element -> String? in
    [kAXTitleAttribute, kAXDescriptionAttribute, kAXHelpAttribute, kAXIdentifierAttribute]
      .map { stringAttribute(element, $0) }.first { !$0.isEmpty }
  }
  let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement] ?? []
  let payload: [String: Any] = [
    "active": app.isActive,
    "windowCount": windows.count,
    "webAreaCount": elements.filter { stringAttribute($0, kAXRoleAttribute) == "AXWebArea" }.count,
    "promptCount": elements.filter { stringAttribute($0, kAXRoleAttribute) == "AXTextArea" && stringAttribute($0, kAXDescriptionAttribute) == "Prompt" }.count,
    "buttons": Array(buttons.prefix(100)),
  ]
  let data = try JSONSerialization.data(withJSONObject: payload)
  print(String(data: data, encoding: .utf8) ?? "{}")
}

func clickPoint(_ value: String) throws {
  let parts = value.split(separator: ",").compactMap { Double($0) }
  guard parts.count == 2 else { throw DriverFailure.message("Accessibility click coordinates are invalid") }
  let point = CGPoint(x: parts[0], y: parts[1])
  guard let down = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: point, mouseButton: .left),
        let up = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left) else {
    throw DriverFailure.message("mouse event creation failed")
  }
  down.post(tap: .cghidEventTap)
  up.post(tap: .cghidEventTap)
}

func replaceText(_ app: NSRunningApplication, element: AXUIElement, value: String, label: String) throws {
  try activate(app)
  try focus(element, label: label)
  let clear = AXUIElementSetAttributeValue(element, kAXValueAttribute as CFString, "" as CFTypeRef)
  guard clear == .success else { throw DriverFailure.message("Accessibility clear failed for \(label): \(clear.rawValue)") }
  try typeText(app, value: value)
  let deadline = Date().addingTimeInterval(5)
  repeat {
    if stringAttribute(element, kAXValueAttribute) == value { return }
    RunLoop.current.run(until: Date().addingTimeInterval(0.05))
  } while Date() < deadline
  throw DriverFailure.message("Accessibility text input did not reach \(label)")
}

func windowBounds(_ root: AXUIElement) throws -> String {
  guard let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement], let window = windows.first,
        let rawPosition = attribute(window, kAXPositionAttribute),
        let rawSize = attribute(window, kAXSizeAttribute),
        CFGetTypeID(rawPosition) == AXValueGetTypeID(), CFGetTypeID(rawSize) == AXValueGetTypeID() else {
    throw DriverFailure.message("DesktopLab window bounds are unavailable")
  }
  var point = CGPoint.zero
  var size = CGSize.zero
  AXValueGetValue(rawPosition as! AXValue, .cgPoint, &point)
  AXValueGetValue(rawSize as! AXValue, .cgSize, &size)
  return "\(Int(point.x)),\(Int(point.y)),\(Int(size.width)),\(Int(size.height))"
}

func run() throws {
  let command = CommandLine.arguments.dropFirst().first ?? ""
  let input = String(data: FileHandle.standardInput.readDataToEndOfFile(), encoding: .utf8) ?? ""
  if command == "trusted" {
    print(AXIsProcessTrusted() ? "true" : "false")
    return
  }
  guard AXIsProcessTrusted() else { throw DriverFailure.message("Accessibility permission is unavailable") }
  let app = try runningApp()
  let root = appElement(app)
  switch command {
  case "ready":
    let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement] ?? []
    print(!windows.isEmpty && descendants(root).contains { stringAttribute($0, kAXRoleAttribute) == "AXWebArea" } ? "true" : "false")
  case "activate":
    try activate(app)
    print("ok")
  case "button-exists":
    print(namedElement(root, role: "AXButton", name: input) == nil ? "false" : "true")
  case "button-enabled":
    guard let button = namedElement(root, role: "AXButton", name: input) else { print("false"); return }
    print((attribute(button, kAXEnabledAttribute) as? Bool) == true ? "true" : "false")
  case "click-button":
    guard let button = namedElement(root, role: "AXButton", name: input) else { throw DriverFailure.message("Accessibility button missing: \(input)") }
    try perform(kAXPressAction, on: button, label: input)
    print("ok")
  case "set-prompt":
    guard let prompt = namedElement(root, role: "AXTextArea", name: "Prompt") else { throw DriverFailure.message("Accessibility prompt field missing") }
    try replaceText(app, element: prompt, value: input, label: "Prompt")
    print("ok")
  case "focus-prompt":
    guard let prompt = namedElement(root, role: "AXTextArea", name: "Prompt") else { throw DriverFailure.message("Accessibility prompt field missing") }
    try activate(app)
    try focus(prompt, label: "Prompt")
    print("ok")
  case "open-project":
    guard let button = namedElement(root, role: "AXButton", name: "Open project") else { throw DriverFailure.message("Accessibility button missing: Open project") }
    try perform(kAXPressAction, on: button, label: "Open project")
    let pathField = try waitForElement(root, role: "AXTextField", name: "Repository path")
    try replaceText(app, element: pathField, value: input, label: "Repository path")
    let openButton = try waitForElement(root, role: "AXButton", name: "Open Repository")
    try perform(kAXPressAction, on: openButton, label: "Open Repository")
    print("ok")
  case "window-bounds":
    print(try windowBounds(root))
  case "visual-inspect":
    try visualInspect(root)
  case "diagnostics":
    try diagnostics(root, app: app)
  case "visual-click-point":
    try clickPoint(input)
    print("ok")
  case "visual-click-named":
    guard let element = namedElement(root, name: input) else { throw DriverFailure.message("Accessibility element missing: \(input)") }
    try perform(kAXPressAction, on: element, label: input)
    print("ok")
  case "visual-type":
    try typeText(app, value: input)
    print("ok")
  case "visual-scroll":
    guard let delta = Int32(input), let event = CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 1, wheel1: delta, wheel2: 0, wheel3: 0) else { throw DriverFailure.message("scroll event creation failed") }
    event.postToPid(app.processIdentifier)
    print("ok")
  case "visual-menu":
    guard let item = namedElement(root, role: "AXMenuItem", name: input) else { throw DriverFailure.message("Accessibility menu item missing: \(input)") }
    try perform(kAXPressAction, on: item, label: input)
    print("ok")
  case "visual-window":
    guard let windows = attribute(root, kAXWindowsAttribute) as? [AXUIElement], let window = windows.first else { throw DriverFailure.message("DesktopLab window is unavailable") }
    try perform(kAXRaiseAction, on: window, label: "DesktopLab window")
    print("ok")
  case "quit":
    _ = app.terminate()
    print("ok")
  default:
    throw DriverFailure.message("unknown native Accessibility command: \(command)")
  }
}

do {
  try run()
} catch {
  FileHandle.standardError.write(Data("\(error)\n".utf8))
  exit(1)
}
