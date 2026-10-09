# Restore normal startup locations before any user startup file or cache.
if [[ ${AUTOMEXIA_ORIGINAL_ZDOTDIR_SET:-0} == 1 ]]; then
  export ZDOTDIR=$AUTOMEXIA_ORIGINAL_ZDOTDIR
else
  unset ZDOTDIR
fi
unset AUTOMEXIA_ORIGINAL_ZDOTDIR AUTOMEXIA_ORIGINAL_ZDOTDIR_SET
[[ -r ${ZDOTDIR-$HOME}/.zshenv ]] && source "${ZDOTDIR-$HOME}/.zshenv"
[[ -o interactive && ${AUTOMEXIA_SHELL_INTEGRATION:-} != 0 ]] || return 0

# Load after normal .zshrc/login files, immediately before the first prompt.
# This hook is local to this shell and removes itself after one invocation.
autoload -Uz add-zsh-hook
__automexia_session_start() {
  add-zsh-hook -d precmd __automexia_session_start
  if [[ -z ${AUTOMEXIA_ZSH_INTEGRATION_LOADED:-} &&
        -r $AUTOMEXIA_SHELL_INTEGRATION_ROOT/zsh/automexia.zsh ]]; then
    source "$AUTOMEXIA_SHELL_INTEGRATION_ROOT/zsh/automexia.zsh"
    __automexia_precmd
  fi
  unfunction __automexia_session_start
}
add-zsh-hook precmd __automexia_session_start
