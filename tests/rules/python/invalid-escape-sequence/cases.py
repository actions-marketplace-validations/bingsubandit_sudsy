x = "\d"
#    ^^ invalid-escape-sequence
y = r"\d"
z = b"\d"
#     ^^ invalid-escape-sequence
f = f"\d {x} \w {x:\d} \{x}"
#     ^^ invalid-escape-sequence
#            ^^ invalid-escape-sequence
#                  ^^ invalid-escape-sequence
#                      ^^ invalid-escape-sequence
g = "\u1234 \x41 \101 \n \\ \' \N{EM DASH}"
h = "\8" "\_"
#    ^^ invalid-escape-sequence
#         ^^ invalid-escape-sequence
i = """
\d
^^ invalid-escape-sequence
"""
