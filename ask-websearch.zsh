#!/usr/bin/env zsh

# ZSH function to query OpenRouter with a websearch-enabled model (:online)
# Usage:
#   ask your question here
# Requirements:
#   - OPENROUTER_API_KEY must be set in the environment
#   - curl and jq must be installed
#
# Note:
#   This function prints the assistant message content only.
#   Change MODEL or headers below if desired.

ASK_WEBSEARCH_MODEL=${ASK_WEBSEARCH_MODEL:-"google/gemini-2.5-flash-lite:online"}

ask() {
    local Q
    Q="$*"

    if [[ -z "$Q" ]]; then
        echo "Usage: ask <your question>"
        return 1
    fi
    if [[ -z "$OPENROUTER_API_KEY" ]]; then
        echo "Error: OPENROUTER_API_KEY is not set"
        return 1
    fi
    if ! command -v curl >/dev/null 2>&1; then
        echo "Error: curl not found"
        return 1
    fi
    if ! command -v jq >/dev/null 2>&1; then
        echo "Error: jq not found"
        return 1
    fi

    curl -sS "https://openrouter.ai/api/v1/chat/completions" \
        -H "Authorization: Bearer $OPENROUTER_API_KEY" \
        -H "Content-Type: application/json" \
        -d "$(jq -nc --arg q "$Q" --arg m "$ASK_WEBSEARCH_MODEL" \
              '{model:$m,messages:[{role:"user",content:$q}],stream:false}')" \
    | jq -r '.choices[0].message.content'
}


