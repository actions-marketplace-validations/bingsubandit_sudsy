x = y = 1
x is "a"
^^^^^^^^ is-literal
x is not 1
^^^^^^^^^^ is-literal
b"" is x
^^^^^^^^ is-literal
x is None
x is True
x is y
x == "a"
x is [1]
^^^^^^^^ is-literal
(1, None) is x
^^^^^^^^^^^^^^ is-literal
x is (1, y)
x is f"a"
#    ^^^^ f-string-missing-placeholders
