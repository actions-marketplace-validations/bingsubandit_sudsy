function Remove-Thing { }
#        ^^^^^^^^^^^^ supports-should-process
function Set-Thing { [CmdletBinding(SupportsShouldProcess)] param() }
function Stop-Thing { [CmdletBinding(SupportsShouldProcess = $false)] param() }
#        ^^^^^^^^^^ supports-should-process
function Get-Thing { }
function global:New-Thing { }
function Get-Items { }
#        ^^^^^^^^^ singular-nouns
function Get-Status { }
function Get-Address { }
function Get-Alias { }
function Get-Series { }
function Get-Name {
    param([Parameter(ValueFromPipeline)]$Item)
#                                       ^^^^^ process-block-for-pipeline
    $Item
}
function Get-Name2 {
    param([Parameter(ValueFromPipeline)]$Item)
    process { $Item }
}
filter Get-Name3 { param([Parameter(ValueFromPipeline)]$Item) $Item }
