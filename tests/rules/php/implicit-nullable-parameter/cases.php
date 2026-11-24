<?php
function a(int $a = null, ?int $b = null, int|null $c = null, $d = null, mixed $e = null) {}
//         ^^^^^^^^^^^^^ implicit-nullable-parameter
class P { function __construct(public $x = null, public string $y = null) {} }
//                                               ^^^^^^^^^^^^^^^^^^^^^^^ implicit-nullable-parameter
