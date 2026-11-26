for %%a in (1 2) do (
  echo !x! !y!
::^^^^ delayed-expansion-in-loop
  echo !z!
)
echo !x! !y!
