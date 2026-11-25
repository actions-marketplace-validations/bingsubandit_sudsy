<?php
function a() { function b() {} }
//             ^^^^^^^^ no-inner-function
function c() { $f = function () {}; }
class D { function m() { function n() {} } }
//                       ^^^^^^^^ no-inner-function
function e() {}
