if a == a
#  ^^^^^^ binary-operator-with-identical-operands
  b
end
h = { a: 1, a: 2 }
#           ^^ no-duplicate-keys
case x
when 1 then y
when 1 then z
^^^^^^^^^^^^^ no-duplicate-case
end
if a && b = c
#  ^^^^^^^^^^ no-assignment-in-logical-condition
  d
end
begin
  run
rescue
^^^^^^ no-empty-catch
end
x = 1
# not part of the clause
begin
  run
rescue
  # ignored on purpose
end
begin
  run
rescue # trails the keyword
^^^^^^ no-empty-catch
end
def f
  run
  # before the clause
rescue NameError
^^^^^^^^^^^^^^^^ no-empty-catch
end
