# Debug Tools for cmd2ai

This directory contains a debugging tool to help understand raw API responses from OpenRouter.

## Rust Binary: check-raw

A Rust implementation that closely matches the cmd2ai internals, useful for debugging streaming issues.

### Build:
```bash
cargo build --bin check-raw
```

### Usage:
```bash
# Basic usage
./target/debug/check-raw "Hello, how are you?"

# With reasoning
./target/debug/check-raw "Explain quantum computing" --reasoning

# Set model via environment variable
export AI_MODEL="google/gemini-2.5-flash-lite-preview-06-17"
./target/debug/check-raw "Hi" --reasoning
```

## What This Tool Shows

The tool displays:
1. The exact request payload sent to OpenRouter
2. Raw SSE (Server-Sent Events) stream lines
3. Parsed JSON for each data chunk
4. Highlighted content and reasoning fields

This helps debug:
- How different models structure their responses
- Whether reasoning tokens are sent in the `reasoning` field or mixed with content
- SSE streaming format variations
- Any unexpected response patterns

## Example Output

```
Using model: openai/gpt-4o-mini
Prompt: Hi
Reasoning: Enabled
--------------------------------------------------------------------------------
Request payload:
{
  "model": "openai/gpt-4o-mini",
  "messages": [...],
  "stream": true,
  "reasoning": {
    "effort": "high",
    "enabled": true
  }
}
--------------------------------------------------------------------------------
Raw SSE stream:
--------------------------------------------------------------------------------
data: {"id":"...","choices":[{"delta":{"reasoning":"Thinking..."},"index":0}]}

Parsed JSON:
{
  "id": "...",
  "choices": [
    {
      "delta": {
        "reasoning": "Thinking..."
      },
      "index": 0
    }
  ]
}
Reasoning: Thinking...

data: {"id":"...","choices":[{"delta":{"content":"Hello!"},"index":0}]}
...
```