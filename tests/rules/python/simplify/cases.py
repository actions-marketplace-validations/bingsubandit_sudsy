a = b = c = d = k = 1
x = k in d.keys()
#   ^^^^^^^^^^^^^ in-dict-keys
x = k not in d.keys()
#   ^^^^^^^^^^^^^^^^^ in-dict-keys
x = k in d.keys(1)
x = [k for k in d.keys()]
#      ^^^^^^^^^^^^^^^^^ in-dict-keys
for k in d.keys():
#        ^^^^^^^^ in-dict-keys
    pass
x = not a == b
#   ^^^^^^^^^^ negate-equal-op
x = not a != b
#   ^^^^^^^^^^ negate-not-equal-op
x = not a < b
x = not a == b == c
x = not not a
#   ^^^^^^^^^ double-negation
x = True if a else False
#   ^^^^^^^^^^^^^^^^^^^^ if-expr-with-true-false
x = False if a else True
#   ^^^^^^^^^^^^^^^^^^^^ if-expr-with-false-true
x = 1 if a else 1
x = b if not a else a
#   ^^^^^^^^^^^^^^^^^ if-expr-with-twisted-arms
x = b if a else a
