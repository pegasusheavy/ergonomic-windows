# Cursor Agents for ergonomic-windows

This directory contains specialized AI agents for different aspects of the `ergonomic-windows` project.

## Available Agents

| Agent | File | Purpose |
|-------|------|---------|
| **Product Manager** | `product-manager.md` | Product vision, roadmap, feature prioritization |
| **Rust Developer** | `rust-developer.mdc` | Code implementation, Rust best practices |
| **Windows API Expert** | `windows-api-expert.mdc` | Windows-specific guidance and patterns |
| **Code Reviewer** | `code-reviewer.mdc` | Code review, quality assurance |
| **Documentation Writer** | `documentation-writer.mdc` | Documentation standards and writing |
| **Performance Engineer** | `performance-engineer.mdc` | Optimization and benchmarking |
| **QA Engineer** | `qa-engineer.mdc` | Testing strategies and coverage |
| **Security Auditor** | `security-auditor.md` | Security review and vulnerability assessment |

## Usage

Reference these agents when working on specific aspects of the project:

### For Product Decisions
```
@product-manager What features should we prioritize for v0.2.0?
```

### For Implementation
```
@rust-developer How should I implement a new file watcher module?
```

### For Windows-Specific Questions
```
@windows-api-expert What's the correct way to handle long file paths?
```

### For Code Reviews
```
@code-reviewer Please review this PR for the registry module.
```

### For Documentation
```
@documentation-writer Help me write docs for the new API.
```

### For Performance
```
@performance-engineer How can I optimize this string conversion?
```

### For Testing
```
@qa-engineer What test cases should I add for this feature?
```

### For Security
```
@security-auditor Review this code for security vulnerabilities.
```

## Agent Collaboration

Agents can work together on complex tasks:

1. **New Feature Development**
   - Product Manager → defines requirements
   - Rust Developer → implements feature
   - Code Reviewer → reviews implementation
   - Documentation Writer → documents API
   - QA Engineer → writes tests

2. **Performance Optimization**
   - Performance Engineer → identifies bottlenecks
   - Rust Developer → implements fixes
   - Code Reviewer → reviews changes
   - QA Engineer → verifies no regressions

3. **Security Hardening**
   - Security Auditor → identifies vulnerabilities
   - Rust Developer → implements fixes
   - Code Reviewer → reviews security changes
   - QA Engineer → adds security tests

## Extending Agents

To add a new agent:

1. Create a `.md` or `.mdc` file in this directory
2. Define the agent's role and expertise
3. Include relevant guidelines and examples
4. Add to this README

