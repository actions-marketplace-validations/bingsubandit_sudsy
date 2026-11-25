package a

type T struct{}
type S[K any] struct{}

func (t T) A()   {}
func (u *T) B()  {}
^^^^^^^^^^^^^^^^^^^ receiver-name-consistent
func (t *T) C()  {}
func (self T) D() {}
//    ^^^^ receiver-naming
func (_ T) E()   {}
//    ^ receiver-naming
func (T) F()     {}
func (this *T) G() {}
//    ^^^^ receiver-naming

func (s *S[K]) H() {}
func (x *S[K]) I() {}
^^^^^^^^^^^^^^^^^^^^^ receiver-name-consistent
func (s S[K]) J()  {}

type U struct{}

func (u U) A()  {}
func (U) B()    {}
func (v *U) C() {}
^^^^^^^^^^^^^^^^^^ receiver-name-consistent
func f(t T)     {}
