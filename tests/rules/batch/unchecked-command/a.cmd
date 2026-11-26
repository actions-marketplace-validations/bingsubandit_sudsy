reg add HKCU\X /f
reg add HKCU\Y /f
if errorlevel 1 exit /b 1
copy a b
^^^^ unchecked-command
echo one
echo two
echo three
if errorlevel 1 exit /b 1
if exist x copy a b
reg query HKCU\Z
^^^ unchecked-command
echo a
echo b
echo c
echo d
