#!/usr/bin/env bash
# Deletes build folders under target/ by name and checks that each one is gone.
# Usage (from the repository root, in Git Bash): tools/clean-target.sh wt-foo wt-bar
# Windows' rd is much faster than rm -rf on large build folders. Bash quoting of a
# backslash before $name broke an earlier loop silently, so the Windows path is built
# with cygpath and the result is checked.
set -u
status=0
for name in "$@"; do
  dir="target/$name"
  if [ ! -d "$dir" ]; then
    echo "not there: $dir"
    continue
  fi
  cmd //c rd //s //q "$(cygpath -w "$dir")" >/dev/null 2>&1
  if [ -d "$dir" ]; then
    echo "STILL THERE: $dir"
    status=1
  else
    echo "deleted $dir"
  fi
done
df -h . | tail -1
exit $status
