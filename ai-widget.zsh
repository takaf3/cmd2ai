#!/usr/bin/env zsh

# ZSH widget to intercept commands starting with capital letters
# and pass them to the AI tool

# Function to check if command starts with capital letter and process it
function _ai_capital_interceptor() {
    local buffer="$BUFFER"
    
    # Check if buffer starts with a capital letter
    if [[ "$buffer" =~ ^[A-Z] ]]; then
        # Build the ai command safely by quoting the buffer
        local cmd="ai \"${buffer}\""
        
        # Clear buffer and put the ai command in it
        BUFFER="$cmd"
        
        # Execute the command
        zle accept-line
    else
        # If not starting with capital, execute normally
        zle accept-line
    fi
}

# Create the widget
zle -N _ai_capital_interceptor

# Bind to Enter key
bindkey '^M' _ai_capital_interceptor

# Optional: Add a message to indicate the widget is loaded
# echo "AI capital letter interceptor loaded. Commands starting with capital letters will be sent to AI."
# echo "Note: Make sure 'ai' is installed globally (npm install -g .) or in your PATH."