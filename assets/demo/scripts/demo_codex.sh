cd /tmp/work/auth-service; clear
snooze() { read -rt "$1" _ < /dev/tty || :; }
printf '\033]0;⠙ Add integration tests\007'
printf '\033[1mcodex\033[0m  ·  ~/work/auth-service\n\n'
printf '\033[2mtask: add integration tests for the session refresh path\033[0m\n\n'
printf '\033[38;5;75m•\033[0m exploring tests/integration/\n'
snooze 8
printf '\033[38;5;114m•\033[0m writing tests/integration/session_refresh.test.ts\n'
snooze 14
printf '\n\033[48;5;178m\033[30m ACTION REQUIRED \033[0m codex wants to run: \033[1mgit push --force-with-lease\033[0m\n'
printf '  Allow? \033[2m[y/N]\033[0m '
printf '\033]9;approval-requested: codex needs your confirmation\007'
printf '\a'
read -r ans
printf '\033]0;⠙ Add integration tests\007'
printf '\n\033[38;5;75m•\033[0m pushing branch codex/session-refresh-tests\n'
snooze 3
printf '\033[32m✓\033[0m done — 3 tests added, branch pushed\n'
printf '\033]9;agent-turn-complete\007'
snooze 600
