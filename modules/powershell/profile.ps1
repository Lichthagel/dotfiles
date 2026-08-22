# User PowerShell additions belong here.
if (-not $env:EDITOR) { $env:EDITOR = "notepad.exe" }

$profileDirectory = Join-Path (Split-Path -Parent $PROFILE) 'Profile.d'
if (Test-Path -LiteralPath $profileDirectory) {
    Get-ChildItem -LiteralPath $profileDirectory -Filter '*.ps1' |
        Sort-Object Name |
        ForEach-Object { . $_.FullName }
}

Invoke-Expression (& { (zoxide init powershell | Out-String) })
Import-Module posh-git

# Hide .exe extension in tab completion
function TabExpansion2 {
    [CmdletBinding(DefaultParameterSetName = 'ScriptInputSet')]
    Param(
        [Parameter(ParameterSetName = 'ScriptInputSet', Mandatory = $true, Position = 0)]
        [string] $inputScript,
        [Parameter(ParameterSetName = 'ScriptInputSet', Mandatory = $true, Position = 1)]
        [int] $cursorColumn,
        [Parameter(ParameterSetName = 'AstInputSet', Mandatory = $true, Position = 0)]
        [System.Management.Automation.Language.Ast] $ast,
        [Parameter(ParameterSetName = 'AstInputSet', Mandatory = $true, Position = 1)]
        [System.Management.Automation.Language.Token[]] $tokens,
        [Parameter(ParameterSetName = 'AstInputSet', Mandatory = $true, Position = 2)]
        [System.Management.Automation.Language.IScriptPosition] $positionOfCursor,
        [Parameter(ParameterSetName = 'ScriptInputSet', Position = 2)]
        [Parameter(ParameterSetName = 'AstInputSet', Position = 3)]
        [Hashtable] $options = $null
    )

    End {
        $result = $null
        if ($psCmdlet.ParameterSetName -eq 'ScriptInputSet') {
            $result = [System.Management.Automation.CommandCompletion]::CompleteInput(
                <#inputScript#>  $inputScript,
                <#cursorColumn#> $cursorColumn,
                <#options#>      $options)
        }
        else {
            $result = [System.Management.Automation.CommandCompletion]::CompleteInput(
                <#ast#>              $ast,
                <#tokens#>           $tokens,
                <#positionOfCursor#> $positionOfCursor,
                <#options#>          $options)
        }

        # Create new completion results without .exe
        if ($result -and $result.CompletionMatches) {
            $newMatches = $result.CompletionMatches | ForEach-Object {
                if ($_.CompletionText -like '*.exe') {
                    $newCompletionText = $_.CompletionText -replace '\.exe$', ''
                    $newListItemText = $_.ListItemText -replace '\.exe$', ''
                    [System.Management.Automation.CompletionResult]::new(
                        $newCompletionText,
                        $newListItemText,
                        $_.ResultType,
                        $_.ToolTip
                    )
                } else {
                    $_
                }
            }

            $result = [System.Management.Automation.CommandCompletion]::new(
                $newMatches,
                $result.CurrentMatchIndex,
                $result.ReplacementIndex,
                $result.ReplacementLength
            )
        }

        return $result
    }
}

gh completion -s powershell | Out-String | Invoke-Expression
docker completion powershell | Out-String | Invoke-Expression
kubectl completion powershell | Out-String | Invoke-Expression
# kind completion powershell | Out-String | Invoke-Expression
# minikube completion powershell | Out-String | Invoke-Expression
msb completion powershell | Out-String | Invoke-Expression

function l {
    param(
        [string]$path = $PWD
    )
    eza -alh $path
}
function la {
    param(
        [string]$path = $PWD
    )
    eza -a $path
}
function ll {
    param(
        [string]$path = $PWD
    )
    eza -l $path
}
function lla {
    param(
        [string]$path = $PWD
    )
    eza -la $path
}
Set-Alias -Name ls -Value "eza"
function lt {
    param(
        [string]$path = $PWD
    )
    eza --tree $path
}
function zh {
    param(
        [string]$path = ''
    )
    z $PWD $path
}
# function zr {
#     param(
#         [string]$path = ''
#     )
#     $repo = git rev-parse --show-toplevel 2>$null
#     z $repo $path
# }

function Update-Software {
    winget upgrade --all
    mise up --bump
    scoop update --all
}

(& uv generate-shell-completion powershell) | Out-String | Invoke-Expression
(& uvx --generate-shell-completion powershell) | Out-String | Invoke-Expression

function Edit-Path {
    <#
    .SYNOPSIS
    Adds or removes paths from the PATH environment variable.

    .DESCRIPTION
    This function allows you to add or remove paths from either the User or System PATH environment variable.
    Uses [System.Environment]::GetEnvironmentVariable and [System.Environment]::SetEnvironmentVariable.
    When removing, displays existing paths for selection by number.

    .EXAMPLE
    Edit-Path
    #>

    # Query for operation type
    $operation = Read-Host "Do you want to (A)dd or (R)emove a path? [A/R]"
    $operation = $operation.ToUpper()

    if ($operation -notin @('A', 'R')) {
        Write-Host "Invalid operation. Please choose 'A' for Add or 'R' for Remove." -ForegroundColor Red
        return
    }

    # Query for scope
    $scope = Read-Host "Do you want to edit the (U)ser or (S)ystem PATH? [U/S]"
    $scope = $scope.ToUpper()

    if ($scope -notin @('U', 'S')) {
        Write-Host "Invalid scope. Please choose 'U' for User or 'S' for System." -ForegroundColor Red
        return
    }

    $target = if ($scope -eq 'U') { 'User' } else { 'Machine' }

    # Get current PATH
    $currentPath = [System.Environment]::GetEnvironmentVariable('PATH', $target)
    $pathArray = $currentPath -split ';' | Where-Object { $_ }

    if ($operation -eq 'A') {
        # Add path
        $pathToModify = Read-Host "Enter the path to add"

        if ([string]::IsNullOrWhiteSpace($pathToModify)) {
            Write-Host "Path cannot be empty." -ForegroundColor Red
            return
        }

        if ($pathArray -contains $pathToModify) {
            Write-Host "Path already exists in PATH variable." -ForegroundColor Yellow
            return
        }

        $pathArray += $pathToModify
        Write-Host "Adding path: $pathToModify" -ForegroundColor Green
    } else {
        # Remove path - display list for selection
        if ($pathArray.Count -eq 0) {
            Write-Host "No paths found in PATH variable." -ForegroundColor Yellow
            return
        }

        Write-Host "`nCurrent paths:" -ForegroundColor Cyan
        for ($i = 0; $i -lt $pathArray.Count; $i++) {
            Write-Host "$($i + 1). $($pathArray[$i])"
        }

        $selection = Read-Host "`nEnter the number of the path to remove"

        if (-not [int]::TryParse($selection, [ref]$null) -or $selection -lt 1 -or $selection -gt $pathArray.Count) {
            Write-Host "Invalid selection." -ForegroundColor Red
            return
        }

        $pathToModify = $pathArray[$selection - 1]
        $pathArray = $pathArray | Where-Object { $_ -ne $pathToModify }
        Write-Host "Removing path: $pathToModify" -ForegroundColor Green
    }

    # Set new PATH
    $newPath = $pathArray -join ';'
    [System.Environment]::SetEnvironmentVariable('PATH', $newPath, $target)

    Write-Host "PATH updated successfully." -ForegroundColor Green
    Write-Host "Note: You may need to restart your terminal for the changes to take effect in new sessions." -ForegroundColor Cyan
}

#f45873b3-b655-43a6-b217-97c00aa0db58 PowerToys CommandNotFound module
Import-Module -Name Microsoft.WinGet.CommandNotFound
#f45873b3-b655-43a6-b217-97c00aa0db58
