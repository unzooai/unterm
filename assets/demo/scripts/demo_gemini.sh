cd /tmp/work/auth-service; clear
snooze() { read -rt "$1" _ < /dev/tty || :; }
printf '\033]0;⏲ Updating docs\007'
printf '\033[1mgemini\033[0m  ·  ~/work/auth-service\n\n'
printf '\033[2mtask: update API docs for the new session endpoints\033[0m\n\n'
printf '\033[38;5;75m→\033[0m scanning docs/api/*.md\n'
snooze 20
printf '\033[38;5;114m→\033[0m rewriting docs/api/sessions.md\n'
snooze 600
