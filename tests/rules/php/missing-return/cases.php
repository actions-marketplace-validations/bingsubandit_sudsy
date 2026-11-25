<?php
function a($x): int { if ($x) { return 1; } }
//                    ^^^^^^^^^^^^^^^^^^^^^ missing-return
function b($x): int { if ($x) { return 1; } elseif ($x > 1) { return 2; } else { return 3; } }
function c($x): int { if ($x): return 1; else: throw new E(); endif; }
function d($x): int { if ($x) { return 1; } exit; }
function d2($x): int { if ($x) { return 1; } die(1); }
function e($x): void { if ($x) { return; } }
function f($x): iterable { if ($x) { yield 1; } }
/** @return int */
function g($x) { if ($x) { return 1; } }
//               ^^^^^^^^^^^^^^^^^^^^^ missing-return
/** @return int|null */
function h($x) { if ($x) { return 1; } }
class K { function __construct() { } }
