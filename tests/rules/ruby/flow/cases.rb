def a(x)
  return 1
  b
# ^ unreachable-code
  c
end

def d(x, y)
  while y
    next
    e
#   ^ unreachable-code
  end
  loop_done = false
  while true
    break if x
  end
  f
  while true
    break
  end
  g
  if x
    return 1
  else
    return 2
  end
  h
# ^ unreachable-code
end

def k(x)
  return x ? 1 : 2
end

def p(x)
  until true
    x
  end
  y
end
