<?php
class A { function make() { return new static(); } }
//                                 ^^^ unsafe-new-static
final class B { function make() { return new static(); } }
class C { final public function __construct() {} function make() { return new static(); } }
trait T { function make() { return new static; } }
//                                 ^^^ unsafe-new-static
class D { function make() { return new self(); } }
/** @phpstan-consistent-constructor */
class E { function make() { return new static(); } }
