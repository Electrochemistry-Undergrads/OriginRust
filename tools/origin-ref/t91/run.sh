#!/bin/bash
# usage: run.sh tag stepsfile afterfile [setup] [menu-downs]
S=C:/Users/RenLab/Downloads/OriginRust/.claude/worktrees/fit-surface-more/tools/origin-ref
cd C:/Users/RenLab/Downloads/OriginRust/re-workspace/scratch/t91
tag=$1; steps=$2; after=$3; setup=${4:-setup.txt}; n=${5:-8}
D=""; for i in $(seq $n); do D="$D{DOWN}"; done
rm -rf d-$tag; mkdir d-$tag
timeout 200 powershell -File $S/origin-dialog.ps1 -Setup $setup -MenuClicks "300,300;K:^a;K:%a;K:F;K:$D;K:{ENTER}" -OutDir d-$tag -StepsFile $steps -AfterFile $after > out-$tag.txt 2>&1
grep -v "^\[True\]" out-$tag.txt | tr '\n' ' '; echo; cat d-$tag/log.txt
