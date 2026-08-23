def "nu-complete leaf themes" [] {
  [arctic forest ocean-dark solarized-dark]
}

def "nu-complete leaf editors" [] {
  [nano vim vi nvim micro hx emacs jed code codium subl gedit kate mousepad zed xjed
   notepad "notepad++"]
}

def "nu-complete leaf inline" [] { [ansi plain] }
def "nu-complete leaf config" [] { [reset remove] }
def "nu-complete leaf history" [] { [edit remove list] }
def "nu-complete leaf shells" [] { [bash zsh fish powershell nushell dump remove] }

export extern "leaf" [
  file?: path
  --help(-h)
  --version(-V)
  --watch(-w)
  --theme: string@"nu-complete leaf themes"
  --editor(-e): string@"nu-complete leaf editors"
  --inline: string@"nu-complete leaf inline"
  --width: int
  --picker
  --fuzzy: string
  --history(-H): string@"nu-complete leaf history"
  --last(-l)
  --config: string@"nu-complete leaf config"
  --update
  --auto-complete: string@"nu-complete leaf shells"
]
