Write-Host "hi"
^^^^^^^^^^^^^^^ no-print
write-host
^^^^^^^^^^ no-print
Write-Output "ok"
Get-Help Write-Host
iex $script
^^^ no-cmdlet-aliases
^^^^^^^^^^^ no-eval
Invoke-Expression -Command $c
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no-eval
ConvertTo-SecureString $p -AsPlainText -Force
^^^^^^^^^^^^^^^^^^^^^^ no-plain-text-secure-string
ConvertTo-SecureString $p
ConvertTo-SecureString `
^^^^^^^^^^^^^^^^^^^^^^ no-plain-text-secure-string
    $p -AsPlainText
Get-WmiObject Win32_Process
^^^^^^^^^^^^^ no-wmi-cmdlets
Get-CimInstance Win32_Process
ConvertTo-SecureString -AsPlainText "x" -Force
^^^^^^^^^^^^^^^^^^^^^^ no-plain-text-secure-string
function Get-Other { Write-Host "y" }
#                    ^^^^^^^^^^^^^^ no-print
function Get-It {
  [Diagnostics.CodeAnalysis.SuppressMessageAttribute("PSAvoidUsingWriteHost", "")]
  param()
  Write-Host "x"
}
[Diagnostics.CodeAnalysis.SuppressMessageAttribute("PSAvoidUsingWriteHost", "")]
class Quiet
{
    [void] Say() { Write-Host "x" }
}
class Mixed {
    [Diagnostics.CodeAnalysis.SuppressMessageAttribute("PSAvoidUsingWriteHost", "")]
    [void] Say() { Write-Host "x" }
    [void] Also() { Write-Host "y" }
}
