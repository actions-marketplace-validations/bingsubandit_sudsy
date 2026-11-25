<?php
class A { public $a; private $b; public readonly int $c;
    function f() { unset($this->a); unset($this->b); unset($this->c); unset($this->z); }
//                       ^^^^^^^^ no-unset-hookable-property
}
final class B { public $a; function f() { unset($this->a); } }
