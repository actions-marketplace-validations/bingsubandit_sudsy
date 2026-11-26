if a==b (
  echo %errorlevel%
::     ^^^^^^^^^^^^ parse-time-variable-in-block
  echo %time%
::     ^^^^^^ parse-time-variable-in-block
  echo "x%random%"
::       ^^^^^^^^ parse-time-variable-in-block
)
echo %errorlevel%
