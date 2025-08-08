#!/usr/bin/env zsh

# ZSH widget to intercept commands starting with capital letters
# and pass them to the AI tool (MCP tools are now enabled by default)

# Function to check if command starts with capital letter and process it
function _ai_capital_interceptor() {
    local buffer="$BUFFER"
    
    # Check if buffer starts with a capital letter
    # but exclude environment variable assignments (VAR=value command)
    if [[ "$buffer" =~ ^[A-Z] && ! "$buffer" =~ ^[^[:space:]]+= ]]; then
        # Build the ai command (tools are now enabled by default)
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
# echo "Note: MCP tools are enabled by default. Configure servers in ~/.config/cmd2ai/cmd2ai.json or .cmd2ai.json"
# echo "Use --no-tools flag to disable MCP tools if needed"