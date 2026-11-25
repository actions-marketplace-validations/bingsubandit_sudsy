package a

import (
	. "fmt"
 ^^^^^^^ no-wildcard-imports
	"os"
	_ "embed"
	m "math"
)

import . "strings"
//     ^^^^^^^^^^^ no-wildcard-imports
