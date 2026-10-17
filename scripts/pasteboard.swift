import AppKit
import Foundation

let args = CommandLine.arguments
let board = NSPasteboard.general
func fail(_ message: String) -> Never { fputs(message + "\n", stderr); exit(1) }
guard args.count >= 2 else { fail("snapshot FILE | restore FILE | text | files | types") }
switch args[1] {
case "snapshot":
    let snapshot = (board.pasteboardItems ?? []).map { item -> [String: Data] in
        var values: [String: Data] = [:]
        for type in item.types { if let data = item.data(forType: type) { values[type.rawValue] = data } }
        return values
    }
    let data = try PropertyListSerialization.data(fromPropertyList: snapshot, format: .binary, options: 0)
    try data.write(to: URL(fileURLWithPath: args[2]), options: .atomic)
case "restore":
    let data = try Data(contentsOf: URL(fileURLWithPath: args[2]))
    guard let saved = try PropertyListSerialization.propertyList(from: data, options: [], format: nil) as? [[String: Data]] else { fail("Invalid snapshot") }
    let items = saved.map { values -> NSPasteboardItem in
        let item = NSPasteboardItem()
        for (type, data) in values { item.setData(data, forType: NSPasteboard.PasteboardType(type)) }
        return item
    }
    board.clearContents()
    if !items.isEmpty && !board.writeObjects(items) { fail("Restore failed") }
case "text":
    guard let text = board.string(forType: .string) else { fail("No text") }
    FileHandle.standardOutput.write(text.data(using: .utf8)!)
case "files":
    guard let urls = board.readObjects(forClasses: [NSURL.self], options: [.urlReadingFileURLsOnly: true]) as? [URL], !urls.isEmpty else { fail("No file URLs") }
    let data = try JSONSerialization.data(withJSONObject: urls.map { $0.path }, options: [.sortedKeys])
    FileHandle.standardOutput.write(data)
case "types":
    print((board.types ?? []).map { $0.rawValue }.joined(separator: "\n"))
default: fail("Unknown command")
}
