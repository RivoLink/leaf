if (-not (Get-Command leaf.exe -ErrorAction SilentlyContinue)) { return }

$global:LeafCompleter = {
    param($wordToComplete, $commandAst)

    $words = $commandAst.ToString() -split '\s+'
    if ($wordToComplete -ne '') {
        $prev = if ($words.Count -gt 2) { $words[-2] } else { '' }
    } else {
        $prev = if ($words.Count -gt 1) { $words[-1] } else { '' }
    }

    switch ($prev) {
        '--theme' {
            @('arctic', 'forest', 'ocean-dark', 'solarized-dark') |
                Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
                }
            return
        }
        { $_ -in '--editor', '-e' } {
            @('nano', 'vim', 'vi', 'nvim', 'micro', 'hx', 'emacs', 'jed',
              'code', 'codium', 'subl', 'gedit', 'kate', 'mousepad', 'zed',
              'xjed', 'notepad', 'notepad++') |
                Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
                }
            return
        }
        '--inline' {
            @('ansi', 'plain') |
                Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
                }
            return
        }
        '--config' {
            @('reset', 'remove') |
                Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
                }
            return
        }
        { $_ -in '--history', '-H' } {
            @('edit', 'remove', 'list') |
                Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
                }
            return
        }
        '--auto-complete' {
            @('bash', 'zsh', 'fish', 'powershell', 'dump', 'remove') |
                Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                    [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
                }
            return
        }
    }

    if ($wordToComplete -like '-*') {
        @('--help', '--version', '--watch', '--theme', '--editor', '--inline',
          '--width', '--picker', '--fuzzy', '--history', '--last', '--config', '--update', '--auto-complete',
          '-h', '-V', '-w', '-e', '-H', '-l') |
            Where-Object { $_ -like "$wordToComplete*" } | ForEach-Object {
                [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterName', $_)
            }
    } else {
        Get-ChildItem 2>$null |
            Where-Object { $_.Name -like "$wordToComplete*" } | ForEach-Object {
                $type = if ($_.PSIsContainer) { 'ProviderContainer' } else { 'ProviderItem' }
                $name = if ($_.PSIsContainer) { $_.Name + '\' } else { $_.Name }
                [System.Management.Automation.CompletionResult]::new($name, $_.Name, $type, $_.FullName)
            }
    }
}

function global:leaf {
    param(
        [Parameter(ValueFromRemainingArguments)]
        [ArgumentCompleter({
            param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)
            & $global:LeafCompleter $wordToComplete $commandAst
        })]
        [string[]]$Arguments
    )
    & leaf.exe @Arguments
}

Register-ArgumentCompleter -CommandName leaf.exe -Native -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    & $global:LeafCompleter $wordToComplete $commandAst
}
