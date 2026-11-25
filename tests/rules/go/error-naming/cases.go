package a

import "errors"

var failed = errors.New("x")
//  ^^^^^^^^^^^^^^^^^^^^^^^^ error-naming
var Bad = errors.New("x")
//  ^^^^^^^^^^^^^^^^^^^^^ error-naming
var errOK = errors.New("x")
var ErrOK = fmt.Errorf("x")
var _ = errors.New("x")
var (
	oops error = errors.New("x")
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ error-naming
	one, two     = errors.New("x"), errors.New("y")
	other        = other()
)

func f() {
	var local = errors.New("x")
}
