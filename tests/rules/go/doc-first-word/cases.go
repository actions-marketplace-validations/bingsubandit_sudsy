package a

// Run does it.
func Run() {}

// does it.
^^^^^^^^^^^ doc-first-word
func Wrong() {}

// wrong lowercase.
func lower() {}

//go:noinline
// Pragma is fine.
func Pragma() {}

// A T is fine, with an article.
type T struct{}

// The U is fine.
type U int

// Other
^^^^^^^^ doc-first-word
type V int

// X is the comment of the group.
type (
    // W is fine.
    W int
    X int
)

// Constants.
const (
    // other, not checked: the group has a comment.
    C = 1
    D = 2
)

// Wrong names a variable.
^^^^^^^^^^^^^^^^^^^^^^^^^^ doc-first-word
var E = 1

// Method does it.
func (T) Method() {}

// does it.
^^^^^^^^^^^ doc-first-word
func (T) Other() {}
