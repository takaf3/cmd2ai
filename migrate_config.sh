#!/bin/bash

# migrate_config_yaml.sh - Migrate environment variables to cmd2ai YAML configuration
# Usage: ./migrate_config_yaml.sh [--output FILE] [--merge] [--dry-run] [--force]

set -e

# Default values
OUTPUT_FILE="$HOME/.config/cmd2ai/cmd2ai.yaml"
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
            echo "Migrate cmd2ai environment variables to YAML configuration"
            echo ""
            echo "Options:"
            echo "  -o, --output FILE   Output file path (default: ~/.config/cmd2ai/cmd2ai.yaml)"
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
    local value=$(echo "$1" | tr '[:upper:]' '[:lower:]')
    if [[ "$value" == "true" || "$value" == "1" || "$value" == "yes" ]]; then
        echo "true"
    elif [[ "$value" == "false" || "$value" == "0" || "$value" == "no" ]]; then
        echo "false"
    else
        echo ""
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

# Start building YAML config
YAML_CONFIG="# cmd2ai Configuration File
# Generated from environment variables
# $(date)
"

# API configuration
if [[ -n "${AI_API_ENDPOINT:-}" ]] || [[ -n "${AI_STREAM_TIMEOUT:-}" ]]; then
    YAML_CONFIG+="
# API Configuration
api:"
    
    if [[ -n "${AI_API_ENDPOINT:-}" ]]; then
        YAML_CONFIG+="
  endpoint: \"${AI_API_ENDPOINT}\""
    fi
    
    if [[ -n "${AI_STREAM_TIMEOUT:-}" ]]; then
        YAML_CONFIG+="
  stream_timeout: ${AI_STREAM_TIMEOUT}"
    fi
fi

# Model configuration
if [[ -n "${AI_MODEL:-}" ]] || [[ -n "${AI_SYSTEM_PROMPT:-}" ]]; then
    YAML_CONFIG+="

# Model Configuration
model:"
    
    if [[ -n "${AI_MODEL:-}" ]]; then
        YAML_CONFIG+="
  default_model: ${AI_MODEL}"
    fi
    
    if [[ -n "${AI_SYSTEM_PROMPT:-}" ]]; then
        # Escape quotes in system prompt
        ESCAPED_PROMPT=$(echo "$AI_SYSTEM_PROMPT" | sed 's/"/\\"/g')
        YAML_CONFIG+="
  system_prompt: \"${ESCAPED_PROMPT}\""
    fi
fi

# Session configuration
if [[ -n "${AI_VERBOSE:-}" ]]; then
    VERBOSE_BOOL=$(convert_bool "$AI_VERBOSE")
    if [[ -n "$VERBOSE_BOOL" ]]; then
        YAML_CONFIG+="

# Session Configuration
session:
  verbose: ${VERBOSE_BOOL}"
    fi
fi

# Reasoning configuration
if [[ -n "${AI_REASONING_ENABLED:-}" ]] || [[ -n "${AI_REASONING_EFFORT:-}" ]] || \
   [[ -n "${AI_REASONING_MAX_TOKENS:-}" ]] || [[ -n "${AI_REASONING_EXCLUDE:-}" ]]; then
    YAML_CONFIG+="

# Reasoning Configuration
reasoning:"
    
    if [[ -n "${AI_REASONING_ENABLED:-}" ]]; then
        ENABLED_BOOL=$(convert_bool "$AI_REASONING_ENABLED")
        if [[ -n "$ENABLED_BOOL" ]]; then
            YAML_CONFIG+="
  enabled: ${ENABLED_BOOL}"
        fi
    fi
    
    if [[ -n "${AI_REASONING_EFFORT:-}" ]]; then
        EFFORT_LOWER=$(echo "$AI_REASONING_EFFORT" | tr '[:upper:]' '[:lower:]')
        if [[ "$EFFORT_LOWER" == "high" ]] || [[ "$EFFORT_LOWER" == "medium" ]] || [[ "$EFFORT_LOWER" == "low" ]]; then
            YAML_CONFIG+="
  effort: ${EFFORT_LOWER}"
        fi
    fi
    
    if [[ -n "${AI_REASONING_MAX_TOKENS:-}" ]]; then
        YAML_CONFIG+="
  max_tokens: ${AI_REASONING_MAX_TOKENS}"
    fi
    
    if [[ -n "${AI_REASONING_EXCLUDE:-}" ]]; then
        EXCLUDE_BOOL=$(convert_bool "$AI_REASONING_EXCLUDE")
        if [[ -n "$EXCLUDE_BOOL" ]]; then
            YAML_CONFIG+="
  exclude: ${EXCLUDE_BOOL}"
        fi
    fi
fi

# MCP configuration
MCP_ADDED=false
if [[ -n "${AI_DISABLE_TOOLS:-}" ]]; then
    DISABLE_BOOL=$(convert_bool "$AI_DISABLE_TOOLS")
    if [[ -n "$DISABLE_BOOL" ]]; then
        YAML_CONFIG+="

# MCP Configuration
mcp:
  disable_tools: ${DISABLE_BOOL}"
        MCP_ADDED=true
    fi
fi

# Handle merge mode - preserve existing MCP servers
if [[ "$MERGE" == true ]] && [[ -f "$OUTPUT_FILE" ]]; then
    echo "Merging with existing configuration..."
    
    # Check if we have Python with PyYAML (more reliable than shell parsing)
    if command -v python3 >/dev/null 2>&1 && python3 -c "import yaml" 2>/dev/null; then
        # Use Python to merge YAML files
        MERGED_YAML=$(python3 -c "
import yaml
import sys

# Read existing config
try:
    with open('$OUTPUT_FILE', 'r') as f:
        existing = yaml.safe_load(f) or {}
except:
    existing = {}

# Parse new config (convert from our text format)
new_config = {}
config_text = '''$YAML_CONFIG'''

# Simple parser for our generated YAML
import re
lines = config_text.strip().split('\n')
current_section = None
current_subsection = None

for line in lines:
    if line.startswith('#') or not line.strip():
        continue
    if not line.startswith(' '):
        # Top-level section
        if ':' in line:
            section = line.split(':')[0].strip()
            new_config[section] = {}
            current_section = section
            current_subsection = None
    elif line.startswith('  ') and not line.startswith('    '):
        # Second-level field
        if ':' in line and current_section:
            parts = line.strip().split(':', 1)
            key = parts[0].strip()
            value = parts[1].strip() if len(parts) > 1 else ''
            # Parse value
            if value.lower() in ('true', 'false'):
                value = value.lower() == 'true'
            elif value.isdigit():
                value = int(value)
            elif value.startswith('\"') and value.endswith('\"'):
                value = value[1:-1]
            new_config[current_section][key] = value

# Merge configurations (new values override old)
for key, value in new_config.items():
    if key in existing and isinstance(existing[key], dict) and isinstance(value, dict):
        existing[key].update(value)
    else:
        existing[key] = value

# Output merged YAML
import yaml
print(yaml.dump(existing, default_flow_style=False, sort_keys=False))
" 2>/dev/null || echo "$YAML_CONFIG")
        
        if [[ -n "$MERGED_YAML" ]]; then
            YAML_CONFIG="$MERGED_YAML"
        fi
    else
        echo "Warning: Python with PyYAML not found, merge may not preserve all settings"
        echo "Consider installing: pip3 install pyyaml"
        
        # Try to preserve MCP servers section if it exists
        if grep -q "^mcp:" "$OUTPUT_FILE" 2>/dev/null; then
            # Extract existing MCP section
            EXISTING_MCP=$(sed -n '/^mcp:/,/^[^ ]/p' "$OUTPUT_FILE" | sed '$d')
            
            if [[ -n "$EXISTING_MCP" ]] && [[ "$MCP_ADDED" == false ]]; then
                YAML_CONFIG+="

$EXISTING_MCP"
            elif [[ -n "$EXISTING_MCP" ]]; then
                # We already added MCP section, need to merge
                echo "Note: Manual merge may be needed for MCP configuration"
            fi
        fi
    fi
fi

# Dry run mode
if [[ "$DRY_RUN" == true ]]; then
    echo "Generated configuration:"
    echo "$YAML_CONFIG"
    exit 0
fi

# Check if output file exists
if [[ -f "$OUTPUT_FILE" ]] && [[ "$FORCE" == false ]] && [[ "$MERGE" == false ]]; then
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
echo "$YAML_CONFIG" > "$OUTPUT_FILE"

echo "✅ Configuration written to $OUTPUT_FILE"
echo ""
echo "Note: OPENROUTER_API_KEY must remain as an environment variable for security."
echo "All other settings can now be managed via the YAML config file."
echo "YAML format supports comments - feel free to add documentation!"