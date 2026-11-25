package a

func f(xs []int) {
	for i, _ := range xs {
//   ^ range
//                    ^ no-empty-block
	}
	for i, v := range xs {
//                    ^ no-empty-block
	}
	for i := range xs {
//                 ^ no-empty-block
	}
	for _, _ = range xs {
//   ^ range
//                   ^ no-empty-block
	}
	for _, v := range xs {
//                    ^ no-empty-block
	}
}
