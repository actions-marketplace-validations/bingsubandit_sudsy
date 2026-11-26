for /f %%a in ('dir /b') do echo %%a
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ for-f-options
for /f "tokens=1" %%a in ('dir /b') do echo %%a
