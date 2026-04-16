import XCTest
import SwiftTreeSitter
import TreeSitterUma

final class TreeSitterUmaTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_uma())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Uma Lang grammar")
    }
}
