# Rulette 0.1 capability matrix

> **Superseded by OpenSpec.**
> This legacy capability matrix is superseded. All active capability specifications reside in [OpenSpec](../openspec/specs/frontends-and-backends/spec.md) and [`compilation-graph-kernel`](../openspec/changes/compilation-graph-kernel/design.md).
> D16 loss-permission and D17 plugin scope remain human decisions recorded under bead `rulette-w6d`; do not infer or invent product approval.

## Core harness domains

| Harness domain | Graph frontend | Graph lowering target | Portable package kinds | Native-only content |
| --- | --- | --- | --- | --- |
| Codex | Yes | Yes | Rules and skills | Configuration becomes unsupported content |
| Claude | Yes | Yes | Rules and skills | Agents, settings, MCP, hooks, and permissions become unsupported content |
| Cursor | Yes | Yes | Rules and skills | MCP, agents, and configuration become unsupported content |
| OpenCode | Yes | Yes | Rules and skills | Agents, MCP, permissions, and configuration become unsupported content |
| Antigravity | Yes | Yes | Rules and skills | Agents and configuration become unsupported content |

## Capability policy

The compiler preserves opaque resources with their owning package and provenance.
Lowering reports whether each selected package and resource is supported, lossy, or dropped for the named target.
Loss blocks staging by default and remains recorded when explicitly accepted.

## Operational boundary

Rulette operates on explicit local inputs only.
It does not fetch remote content, manage lockfiles, resolve package registries, install skills, execute agents, or write directly to arbitrary native destinations.

The compilation graph, lowering plan, and staged publication plan are deterministic for the same inputs and options.
