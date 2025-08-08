#!/bin/bash

# migrate_config.sh - Migrate environment variables to cmd2ai JSON configuration
# Usage: ./migrate_config.sh [--output FILE] [--merge] [--dry-run] [--force]

set -e

# Default values
OUTPUT_FILE="$HOME/.config/cmd2ai/cmd2ai.json"
MERGE=false
DRY_RUN=false
FORCE=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -o|--output)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --merge)
            MERGE=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --force)
            FORCE=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Migrate cmd2ai environment variables to JSON configuration"
            echo ""
            echo "Options:"
            echo "  -o, --output FILE   Output file path (default: ~/.config/cmd2ai/cmd2ai.json)"
            echo "  --merge            Merge with existing config file if it exists"
            echo "  --dry-run          Print the config without writing to file"
            echo "  --force            Overwrite existing config without prompting"
            echo "  -h, --help         Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Function to convert boolean env vars
convert_bool() {
    local value=$(echo "$1" | tr '[:upper:]' '[:lower:]')  # Convert to lowercase
    if [[ "$value" == "true" || "$value" == "1" || "$value" == "yes" ]]; then
        echo "true"
    elif [[ "$value" == "false" || "$value" == "0" || "$value" == "no" ]]; then
        echo "false"
    else
        echo "null"
    fi
}

# Check for AI environment variables
AI_VARS=$(env | grep "^AI_" | cut -d= -f1 || true)

if [[ -z "$AI_VARS" ]]; then
    echo "No AI_* environment variables found."
    echo ""
    echo "Available environment variables:"
    echo "  AI_API_ENDPOINT        - Custom API endpoint"
    echo "  AI_MODEL               - Default model"
    echo "  AI_SYSTEM_PROMPT       - System prompt"
    echo "  AI_VERBOSE             - Enable verbose mode"
    echo "  AI_STREAM_TIMEOUT      - Stream timeout in seconds"
    echo "  AI_REASONING_ENABLED   - Enable reasoning"
    echo "  AI_REASONING_EFFORT    - Reasoning effort (high/medium/low)"
    echo "  AI_REASONING_MAX_TOKENS - Max reasoning tokens"
    echo "  AI_REASONING_EXCLUDE   - Exclude reasoning from output"
    echo "  AI_DISABLE_TOOLS       - Disable MCP tools"
    exit 1
fi

echo "Found AI environment variables:"
for var in $AI_VARS; do
    value="${!var}"
    # Mask sensitive values
    if [[ "$var" == *"KEY"* ]] || [[ "$var" == *"TOKEN"* ]]; then
        if [[ ${#value} -gt 4 ]]; then
            value="${value:0:4}..."
        else
            value="***"
        fi
    fi
    echo "  $var=$value"
done
echo ""

# Start building JSON config
JSON_CONFIG='{'
FIRST_SECTION=true

# API configuration
if [[ -n "${AI_API_ENDPOINT:-}" ]] || [[ -n "${AI_STREAM_TIMEOUT:-}" ]]; then
    [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
    JSON_CONFIG+='"api":{'
    FIRST_FIELD=true
    
    if [[ -n "${AI_API_ENDPOINT:-}" ]]; then
        JSON_CONFIG+='"endpoint":"'$AI_API_ENDPOINT'"'
        FIRST_FIELD=false
    fi
    
    if [[ -n "${AI_STREAM_TIMEOUT:-}" ]]; then
        [[ "$FIRST_FIELD" == false ]] && JSON_CONFIG+=','
        JSON_CONFIG+='"stream_timeout":'$AI_STREAM_TIMEOUT
    fi
    
    JSON_CONFIG+='}'
    FIRST_SECTION=false
fi

# Model configuration
if [[ -n "${AI_MODEL:-}" ]] || [[ -n "${AI_SYSTEM_PROMPT:-}" ]]; then
    [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
    JSON_CONFIG+='"model":{'
    FIRST_FIELD=true
    
    if [[ -n "${AI_MODEL:-}" ]]; then
        JSON_CONFIG+='"default_model":"'$AI_MODEL'"'
        FIRST_FIELD=false
    fi
    
    if [[ -n "${AI_SYSTEM_PROMPT:-}" ]]; then
        [[ "$FIRST_FIELD" == false ]] && JSON_CONFIG+=','
        # Escape quotes in system prompt
        ESCAPED_PROMPT=$(echo "$AI_SYSTEM_PROMPT" | sed 's/"/\\"/g')
        JSON_CONFIG+='"system_prompt":"'$ESCAPED_PROMPT'"'
    fi
    
    JSON_CONFIG+='}'
    FIRST_SECTION=false
fi

# Session configuration
if [[ -n "${AI_VERBOSE:-}" ]]; then
    [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
    VERBOSE_BOOL=$(convert_bool "$AI_VERBOSE")
    if [[ "$VERBOSE_BOOL" != "null" ]]; then
        JSON_CONFIG+='"session":{"verbose":'$VERBOSE_BOOL'}'
        FIRST_SECTION=false
    fi
fi

# Reasoning configuration
if [[ -n "${AI_REASONING_ENABLED:-}" ]] || [[ -n "${AI_REASONING_EFFORT:-}" ]] || \
   [[ -n "${AI_REASONING_MAX_TOKENS:-}" ]] || [[ -n "${AI_REASONING_EXCLUDE:-}" ]]; then
    [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
    JSON_CONFIG+='"reasoning":{'
    FIRST_FIELD=true
    
    if [[ -n "${AI_REASONING_ENABLED:-}" ]]; then
        ENABLED_BOOL=$(convert_bool "$AI_REASONING_ENABLED")
        if [[ "$ENABLED_BOOL" != "null" ]]; then
            JSON_CONFIG+='"enabled":'$ENABLED_BOOL
            FIRST_FIELD=false
        fi
    fi
    
    if [[ -n "${AI_REASONING_EFFORT:-}" ]]; then
        EFFORT_LOWER=$(echo "$AI_REASONING_EFFORT" | tr '[:upper:]' '[:lower:]')
        if [[ "$EFFORT_LOWER" == "high" ]] || [[ "$EFFORT_LOWER" == "medium" ]] || [[ "$EFFORT_LOWER" == "low" ]]; then
            [[ "$FIRST_FIELD" == false ]] && JSON_CONFIG+=','
            JSON_CONFIG+='"effort":"'$EFFORT_LOWER'"'
            FIRST_FIELD=false
        fi
    fi
    
    if [[ -n "${AI_REASONING_MAX_TOKENS:-}" ]]; then
        [[ "$FIRST_FIELD" == false ]] && JSON_CONFIG+=','
        JSON_CONFIG+='"max_tokens":'$AI_REASONING_MAX_TOKENS
        FIRST_FIELD=false
    fi
    
    if [[ -n "${AI_REASONING_EXCLUDE:-}" ]]; then
        EXCLUDE_BOOL=$(convert_bool "$AI_REASONING_EXCLUDE")
        if [[ "$EXCLUDE_BOOL" != "null" ]]; then
            [[ "$FIRST_FIELD" == false ]] && JSON_CONFIG+=','
            JSON_CONFIG+='"exclude":'$EXCLUDE_BOOL
        fi
    fi
    
    JSON_CONFIG+='}'
    FIRST_SECTION=false
fi

# MCP configuration
MCP_SECTION_NEEDED=false
MCP_CONFIG=""

if [[ -n "${AI_DISABLE_TOOLS:-}" ]]; then
    DISABLE_BOOL=$(convert_bool "$AI_DISABLE_TOOLS")
    if [[ "$DISABLE_BOOL" != "null" ]]; then
        MCP_CONFIG='"disable_tools":'$DISABLE_BOOL
        MCP_SECTION_NEEDED=true
    fi
fi

# Always preserve existing MCP configuration when merging
if [[ "$MERGE" == true ]] && [[ -f "$OUTPUT_FILE" ]]; then
    if command -v jq >/dev/null 2>&1; then
        # Get existing MCP configuration
        EXISTING_MCP=$(jq '.mcp // {}' "$OUTPUT_FILE" 2>/dev/null || echo "{}")
        if [[ "$EXISTING_MCP" != "{}" ]] && [[ "$EXISTING_MCP" != "null" ]]; then
            # If we have new MCP config, merge it
            if [[ "$MCP_SECTION_NEEDED" == true ]]; then
                # Create a temporary JSON with our new values
                TEMP_MCP='{"disable_tools":'$(convert_bool "${AI_DISABLE_TOOLS:-false}")'}'
                # Merge existing with new (new values override)
                MERGED_MCP=$(echo "$EXISTING_MCP" | jq ". * $TEMP_MCP")
                [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
                JSON_CONFIG+='"mcp":'$MERGED_MCP
                FIRST_SECTION=false
                MCP_SECTION_NEEDED=false
            else
                # Just preserve existing MCP config as-is
                [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
                JSON_CONFIG+='"mcp":'$EXISTING_MCP
                FIRST_SECTION=false
            fi
        elif [[ "$MCP_SECTION_NEEDED" == true ]]; then
            # No existing MCP config, just add our new one
            [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
            JSON_CONFIG+='"mcp":{'$MCP_CONFIG'}'
            FIRST_SECTION=false
        fi
    elif [[ "$MCP_SECTION_NEEDED" == true ]]; then
        # No jq and we have MCP config to add
        echo "Warning: jq not found, MCP merge may not preserve all settings"
        [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
        JSON_CONFIG+='"mcp":{'$MCP_CONFIG'}'
        FIRST_SECTION=false
    fi
elif [[ "$MCP_SECTION_NEEDED" == true ]]; then
    # Not merging, just add MCP config if we have it
    [[ "$FIRST_SECTION" == false ]] && JSON_CONFIG+=','
    JSON_CONFIG+='"mcp":{'$MCP_CONFIG'}'
    FIRST_SECTION=false
fi

JSON_CONFIG+='}'

# Format JSON using jq if available, otherwise use basic formatting
if command -v jq >/dev/null 2>&1; then
    FORMATTED_JSON=$(echo "$JSON_CONFIG" | jq .)
else
    # Basic formatting without jq
    FORMATTED_JSON=$(echo "$JSON_CONFIG" | sed 's/,/,\n  /g' | sed 's/{/{\n  /g' | sed 's/}/\n}/g')
fi

# Handle merge mode
if [[ "$MERGE" == true ]] && [[ -f "$OUTPUT_FILE" ]]; then
    echo "Merging with existing configuration..."
    if command -v jq >/dev/null 2>&1; then
        # Use jq to merge configs (new values override old)
        MERGED_JSON=$(jq -s '.[0] * .[1]' "$OUTPUT_FILE" <(echo "$FORMATTED_JSON"))
        FORMATTED_JSON="$MERGED_JSON"
    else
        echo "Warning: jq not found, merge may not work correctly"
    fi
fi

# Dry run mode
if [[ "$DRY_RUN" == true ]]; then
    echo "Generated configuration:"
    echo "$FORMATTED_JSON"
    exit 0
fi

# Check if output file exists
if [[ -f "$OUTPUT_FILE" ]] && [[ "$FORCE" == false ]]; then
    read -p "$OUTPUT_FILE already exists. Overwrite? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 1
    fi
fi

# Create directory if it doesn't exist
OUTPUT_DIR=$(dirname "$OUTPUT_FILE")
if [[ ! -d "$OUTPUT_DIR" ]]; then
    mkdir -p "$OUTPUT_DIR"
    echo "Created directory: $OUTPUT_DIR"
fi

# Write config file
echo "$FORMATTED_JSON" > "$OUTPUT_FILE"

echo "✅ Configuration written to $OUTPUT_FILE"
echo ""
echo "Note: OPENROUTER_API_KEY must remain as an environment variable for security."
echo "All other settings can now be managed via the JSON config file."