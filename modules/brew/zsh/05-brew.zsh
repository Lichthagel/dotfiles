for brew_prefix in /home/linuxbrew/.linuxbrew /home/linuxbrew/.linuxbrew/bin; do
    [[ -x "$brew_prefix/bin/brew" ]] && export PATH="$brew_prefix/bin:$brew_prefix/sbin:$PATH"
done

if (( $+commands[brew] )); then
    eval "$(brew shellenv)"
fi
