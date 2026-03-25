#!/bin/bash

# https://tmuxai.dev/tmux-startup-script/

S=nextless_playground

tmux new -d -s $S

# https://stackoverflow.com/a/37602055
tmux set -t $s -g pane-border-status top
tmux set -t $s -g mouse on

tmux rename-window -t $S:0 main

tmux split-window -h -t $S:main.0
tmux select-pane -t $S:main.1 -T node
tmux send-keys -t $S:0.1 "edgeless_node_d" C-m

tmux split-window -v -t $S:main.1
tmux select-pane -t $S:main.2 -T controller
tmux send-keys -t $S:main.2 "edgeless_con_d" C-m

tmux select-pane -t $S:main.0 -T development_environment
tmux select-pane -t $S:main.0
tmux send-keys -t $S:main.0 "cat README.md" C-m

tmux attach -t $S
