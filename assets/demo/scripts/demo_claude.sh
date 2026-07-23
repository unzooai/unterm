cd /tmp/work/auth-service; clear
snooze() { read -rt "$1" _ < /dev/tty || :; }
printf '\033]0;⠼ Refactor auth middleware\007'
printf '\033[38;5;208m✻\033[0m \033[1mClaude Code\033[0m  ·  ~/work/auth-service\n\n'
printf '\033[2m> refactor the auth middleware to use the new session store\033[0m\n\n'
printf '\033[38;5;75m⏺\033[0m Read \033[2msrc/middleware/auth.ts\033[0m\n'
printf '\033[38;5;114m⏺\033[0m Update \033[2msrc/middleware/auth.ts\033[0m — swap legacy cookie parse for SessionStore.get\n'
i=0
while true; do
  snooze 5; i=$((i+1))
  case $((i % 8)) in
    0) printf '\033[38;5;75m⏺\033[0m Read \033[2msrc/routes/login.ts\033[0m\n';;
    1) printf '\033[38;5;114m⏺\033[0m Update \033[2msrc/session/store.ts\033[0m — expire refresh tokens on logout\n';;
    2) printf '\033[38;5;180m⏺\033[0m Bash \033[2mnpm test -- auth\033[0m\n';;
    3) printf '  \033[32m✓\033[0m 47 passing\n';;
    4) printf '\033[38;5;75m⏺\033[0m Read \033[2msrc/middleware/rate_limit.ts\033[0m\n';;
    5) printf '\033[38;5;114m⏺\033[0m Update \033[2msrc/middleware/auth.ts\033[0m — attach session to request context\n';;
    6) printf '\033[38;5;180m⏺\033[0m Bash \033[2mtsc --noEmit\033[0m\n';;
    7) printf '  \033[32m✓\033[0m no type errors\n';;
  esac
done
