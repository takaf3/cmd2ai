#!/usr/bin/env zsh

# ZSH widget to intercept commands starting with capital letters
# and pass them to the AI tool with automatic MCP server detection

# Function to check if command starts with capital letter and process it
function _ai_capital_interceptor() {
    local buffer="$BUFFER"
    
    # Check if buffer starts with a capital letter
    # but exclude environment variable assignments (VAR=value command)
    if [[ "$buffer" =~ ^[A-Z] && ! "$buffer" =~ ^[^[:space:]]+= ]]; then
        # Build the ai command with --auto-tools for MCP server auto-detection
        local cmd="ai --auto-tools \"${buffer}\""
        
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
# echo "AI capital letter interceptor loaded. Commands starting with capital letters will be sent to AI with MCP auto-detection."
# echo "Note: Configure MCP servers in ~/.config/cmd2ai/cmd2ai.json or .cmd2ai.json"