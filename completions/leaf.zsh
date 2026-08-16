#compdef leaf

_leaf() {
    local -a flags

    flags=(
        '-h[Show help message and exit]'
        '--help[Show help message and exit]'
        '-V[Show version information and exit]'
        '--version[Show version information and exit]'
        '-w[Watch file for changes and reload]'
        '--watch[Watch file for changes and reload]'
        '--theme[Set color theme preset]:theme:(arctic forest ocean-dark solarized-dark)'
        '-e[Set external editor]:editor:(nano vim vi nvim micro hx emacs jed code codium subl gedit kate mousepad zed xjed notepad notepad++)'
        '--editor[Set external editor]:editor:(nano vim vi nvim micro hx emacs jed code codium subl gedit kate mousepad zed xjed notepad notepad++)'
        '--inline[Render to stdout (no TUI)]:format:(ansi plain)'
        '--width[Set maximum content width (min: 20)]:width:'
        '--picker[Open the file browser picker]'
        '--fuzzy[Open the fuzzy file picker (KEYWORD pre-fills the filter)]::keyword:'
        '-H[Open picker, or edit/remove/list file history]::action:(edit remove list)'
        '--history[Open picker, or edit/remove/list file history]::action:(edit remove list)'
        '--config[Open, reset or remove configuration file]::action:(reset remove)'
        '--update[Update leaf to the latest version]'
        '--auto-complete[Install, dump or remove shell completions]::shell:(bash zsh fish powershell dump remove)'
    )

    _arguments -s $flags '*:file:_files'
}

compdef _leaf leaf
