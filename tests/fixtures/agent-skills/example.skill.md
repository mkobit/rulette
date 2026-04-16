---
name: file-system-operations
description: Perform basic file system operations safely
version: 1.0.0
license: MIT
compatibility: agent-skills-1.0
allowed-tools: ["bash", "read", "write"]
---
# File System Operations

This skill allows an agent to perform file system operations safely by constraining the scope of operations to the current directory and disallowing destructive operations like `rm -rf`.

## Instructions

1. Use standard bash commands to explore the filesystem.
2. Read files using the `read` tool.
3. Write files using the `write` tool.
4. Do NOT execute any `rm` commands.
