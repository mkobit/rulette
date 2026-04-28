---
name: security-expert
description: Find vulnerabilities
kind: local
tools:
  - grep
mcpServers:
  filesystem:
    command: npx
    args: ["-y", "@modelcontextprotocol/server-filesystem", "."]
model: gemini-2.0-flash
---
# Security Expert System Prompt
Find bugs.
