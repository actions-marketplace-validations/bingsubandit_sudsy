package a

func f() {
	_ = errors.New(fmt.Sprintf("x %d", 1))
//   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ errorf
	_ = errors.New(fmt.Sprint("x"), 1)
	_ = errors.New("x")
	_ = fmt.Errorf("x %d", 1)
	_ = errors.New(fmt.Sprintf("x") + "y")
}
