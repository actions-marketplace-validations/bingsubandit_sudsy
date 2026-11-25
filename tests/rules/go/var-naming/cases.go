package a

const MAX_VALUE = 1
//    ^^^^^^^^^ var-naming
const ok_, maxA = 1, 2
//    ^^^ var-naming

var userId, other_name int
//  ^^^^^^ var-naming
//          ^^^^^^^^^^ var-naming

type Cpu struct {
//   ^^^ var-naming
	Id, name_x int
 ^^ var-naming
//   ^^^^^^ var-naming
	_, fine    int
}

func get_value(url_s, userID string, rest_ ...int) (out_x int) {
//   ^^^^^^^^^ var-naming
//             ^^^^^ var-naming
//                                   ^^^^^ var-naming
//                                                  ^^^^^ var-naming
	x_y := 1
 ^^^ var-naming
	f := func(lit_a int) {}
	z := MAX_VALUE
	return x_y
}

func (r_x *T) Method(a_b int) {}
//                   ^^^ var-naming

type I interface {
	Do_it(arg_a int) (res_a int)
//     ^^^^^ var-naming
//                 ^^^^^ var-naming
}

var LastInsertId, startHttpServer, v1_2 int
//                ^^^^^^^^^^^^^^^ var-naming
