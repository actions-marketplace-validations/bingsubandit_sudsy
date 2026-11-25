package a

var len = 1
//  ^^^ redefines-builtin-id

const iota = 2
//    ^^^^ redefines-builtin-id

func new() {}
//   ^^^ redefines-builtin-id

func f(copy int, s string) (error int) {
//     ^^^^ redefines-builtin-id
//                          ^^^^^ redefines-builtin-id
	max := 1
 ^^^ redefines-builtin-id
	min = 2
 ^^^ redefines-builtin-id
	_, print := 1, 2
//  ^^^^^ redefines-builtin-id
	return
}

func (string T) method() {}

func g[any int]() {}
//     ^^^ redefines-builtin-id
