package a

func f(x int, xs []int, m *map[string]int) {
	x += 1
 ^^^^^^ increment-decrement
	x -= 1
 ^^^^^^ increment-decrement
	xs[0] += 1
 ^^^^^^^^^^ increment-decrement
	(*m)["k"] += 1
 ^^^^^^^^^^^^^^ increment-decrement
	x += 2
	x += 01
	x *= 1
	for i := 0; i < 3; i += 1 {
//                  ^^^^^^ increment-decrement
//                         ^ no-empty-block
	}
}

type U map[string]int

func (s *U) M() error {
	(*s)["called"] += 1
 ^^^^^^^^^^^^^^^^^^^ increment-decrement
	return nil
}
