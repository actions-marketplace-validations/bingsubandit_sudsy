import re

a = re.compile("\x1f")
#   ^^^^^^^^^^^^^^^^^^ no-control-regex
b = re.search(r"a  b", "x")
#   ^^^^^^^^^^^^^^^^^^^^^^^ no-regex-spaces
c = re.sub("[a" "́]", "", "x")
#   ^^^^^^^^^^^^^^^^^^^^^^^^^^ no-misleading-character-class
d = re.compile("a b")
e = compile("a  b")
