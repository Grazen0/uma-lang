package tree_sitter_uma_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_uma "github.com/tree-sitter/tree-sitter-uma/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_uma.Language())
	if language == nil {
		t.Errorf("Error loading Uma Lang grammar")
	}
}
