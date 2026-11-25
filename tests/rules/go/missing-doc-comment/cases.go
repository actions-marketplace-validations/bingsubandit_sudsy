// Package a is the test package.
package a

func Run() {}
//   ^^^ missing-doc-comment

// Detached comment.

func Detached() {}
//   ^^^^^^^^ missing-doc-comment

//go:noinline
func Pragma() {}
//   ^^^^^^ missing-doc-comment

// Deprecated: use Run.
func Old() {}
//   ^^^ missing-doc-comment

type T struct{}
//   ^ missing-doc-comment

func (T) Method() {}
//       ^^^^^^ missing-doc-comment

func (T) String() string { return "" }

type hidden struct{}

func (hidden) Method() {}

func unexported() {}

const A, b = 1, 2
//    ^ missing-doc-comment

const (
    C = 1
//  ^ missing-doc-comment
    D = 2
)

// Doc of the group.
var (
    V = 1
    W = 2
)

var (
    X = 1
//  ^ missing-doc-comment
    // Y has a comment.
    Y = 2
    Z = 3
)

type S []int
//   ^ missing-doc-comment

func (s S) Len() int { return 0 }
func (s S) Less(i, j int) bool { return false }
func (s S) Swap(i, j int) {}
