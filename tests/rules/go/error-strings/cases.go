package a

import (
	"errors"
	"fmt"
)

var a = errors.New("failed.")
//                 ^^^^^^^^^ error-strings
var b = fmt.Errorf("failed: %w", err)
var c = fmt.Errorf("failed:")
//                 ^^^^^^^^^ error-strings
var d = errors.Wrap(err, "could not read!")
//                       ^^^^^^^^^^^^^^^^^ error-strings
var e = errors.New("line\n")
//                 ^^^^^^^^ error-strings
var f = errors.New("Capitalized")
var g = errors.New("quoted \\n")
var h = errors.New(`raw.`)
//                 ^^^^^^ error-strings
var i = errors.New(msg + ".")
var j = log.Printf("failed.")
var k = errors.New("")
