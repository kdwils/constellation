# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Constellation is a simple, self-hosted health check dashboard for Kubernetes clusters that requires minimal configuration.

**Purpose**: Provide an out-of-the-box view of pod health and traffic routing paths with zero or minimal setup.

**Key Goals**:
* Simple health monitoring dashboard that works immediately after deployment
* Visualize how traffic reaches pods (ingress → service → pod, httproute → service → pod, or direct service → pod)
* Minimal configuration required - works with sensible defaults
* Optional configuration via annotations on resources for customization
* Clear visibility into pod health status and routing paths

**Architecture**: Written in Go with a Vue frontend.

## Project Structure

The frontend and backend are served from the same server using interface-based dependency injection.

The cluster state is served at `/state/`.

Static files are served at `/` and default to `index.html` if no static file is found.

## Architecture

This is a simple Kubernetes health check dashboard with:
- **Backend**: Go controller-runtime based server that watches Kubernetes API and serves pod health and routing state
- **Frontend**: Vue application that displays health status and traffic routing paths
- **Data Flow**:
  - Passive state: Kubernetes API → Controllers (reconcile) → StateManager → `/state` endpoint + WebSocket
  - Active health: HealthChecker (periodic HTTP checks) → StateManager → WebSocket updates
- **Configuration**: Works out-of-the-box with sensible defaults, optional annotation-based customization

### Core Components

**Main Entry Point**: `cmd/main.go` using controller-runtime manager

**Controllers**: Individual reconcilers for each Kubernetes resource type:
- `internal/controller/namespace_controller.go` - Namespace monitoring
- `internal/controller/service_controller.go` - Service monitoring
- `internal/controller/pod_controller.go` - Pod monitoring
- `internal/controller/httproute_controller.go` - HTTPRoute monitoring

**State Management**: `internal/controller/state_manager.go` orchestrates cluster health state
- Maintains in-memory cache of pod health and traffic routing paths
- Provides real-time health updates via WebSocket
- Builds initial health state on startup with zero configuration required

**Health Monitoring**: `internal/controller/health_checker.go` performs active health checks
- Executes HTTP health checks based on pod readiness/liveness probe configurations
- Periodically polls service endpoints to verify availability
- Stores health history and status per service
- Integrates with StateManager to push health status updates
- Configurable check intervals and timeouts
- Supports annotation-based opt-out via `constellation.kyledev.co/ignore`

**HTTP Server**: `internal/server/server.go` provides dual-mode server
- JSON API endpoint for cluster health state at `/state`
- WebSocket endpoint for real-time health updates at `/ws`
- Static file serving for Vue dashboard frontend
- Health check endpoint at `/healthz`

**Types**: `internal/types/resources.go` defines the data structures for health state and traffic routing paths

### Interface-based Design

All components use interfaces for dependency injection and testability:

**StateProvider Interface**: Used by the server to interact with state management
- `GetHierarchy()` - Returns current cluster health state and routing paths
- `Subscribe()` - Get update channel for real-time health notifications
- `PushUpdate()` - Send health updates via WebSocket

**Controller Interfaces**: Each controller implements reconcile.Reconciler
- Dependency injection of StateManager through constructor functions
- Context propagation for cancellation and logging
- Clean separation of concerns between resource watching and state management

### Key Patterns

**Interface-based Design**: All major components use interfaces for testability
**Dependency Injection**: Components injected through constructor functions  
**Context Propagation**: All operations use context.Context for cancellation and logging
**Controller-Runtime**: Built on Kubernetes controller-runtime for robust resource watching

## Key File Locations

- Main backend entry: `cmd/main.go`
- Server implementation: `internal/server/server.go`
- State management: `internal/controller/state_manager.go`
- Health checker: `internal/controller/health_checker.go`
- Resource controllers: `internal/controller/*_controller.go`
- Type definitions: `internal/types/resources.go`
- Cache utilities: `internal/cache/cache.go`
- Frontend components: `frontend/src/components/`
- Frontend entry: `frontend/src/main.ts`
- Module definition: `go.mod`

## Development Commands

### Building & Running
```bash
go build -o constellation cmd/main.go  # Build the binary
./constellation                       # Run with default settings
```

### Development Mode
```bash
go run cmd/main.go                    # Run directly with Go
go run cmd/main.go --static-dir frontend/dist --server-port 8080
```

### Frontend Development
```bash
cd frontend && npm run dev            # Run frontend development server

# For full stack development, run both:
# Terminal 1: Backend with hot reload
go run cmd/main.go --static-dir frontend/dist --server-port 8080

# Terminal 2: Frontend dev server (auto-proxies /ws and /state to backend)
cd frontend && npm run dev
```

**Note**: The Vite dev server is configured to proxy WebSocket (`/ws`) and API (`/state`) requests to the backend server at `localhost:8080`.

### Building
```bash
# Build frontend for production
cd frontend && npm run build

# Build backend for production
go build -o constellation cmd/main.go

# Full production build
cd frontend && npm run build && cd .. && go build -o constellation cmd/main.go
```

### Testing
```bash
# Run backend tests
go test ./...

# Run backend tests with coverage
go test -cover ./...

# Run specific package tests
go test ./internal/controller/

# Run frontend tests
cd frontend && npm test
```

### Code Quality
```bash
# Format Go code
go fmt ./...

# Vet Go code for issues
go vet ./...

# Check frontend types and linting
cd frontend && npm run type-check
cd frontend && npm run lint
```

### Testing Strategy

The project uses Go's built-in testing framework with interface-based mocking:

**Unit Testing**: Each component has dedicated test files
- Controllers test resource reconciliation logic
- StateManager tests state building and update propagation  
- Server tests HTTP endpoints and WebSocket handling

**Interface Mocking**: Use interfaces to mock dependencies
- StateProvider interface for testing server components
- Kubernetes client interfaces for testing controllers
- Avoid external dependencies during testing

**Test Organization**:
- Test files follow `*_test.go` naming convention
- Use table-driven tests for multiple scenarios
- Test exact values, not partial matches

### Coding Standards

**NEVER use `else` statements. This is strictly forbidden.**

Structure code to avoid `else` by:
- Returning early from functions
- Using guard clauses
- Inverting conditions when needed
- Using functional programming patterns

#### Examples

**❌ BAD - Using else statements:**
```go
// Example 1: Nested if-else hell (FORBIDDEN)
if service.Status != nil {
    if service.Status.LoadBalancer != nil {
        if service.Status.LoadBalancer.Ingress != nil {
            // do something
        } else {
            // error case
        }
    } else {
        // error case  
    }
} else {
    // error case
}

// Example 2: Simple if-else (FORBIDDEN)
func processUser(user *User) string {
    if user != nil {
        return user.Name
    } else {
        return "Unknown"
    }
}
```

**✅ GOOD - No else statements:**
```go
// Example 1: Early returns and guard clauses
func extractLoadBalancerIPs(service *v1.Service) []string {
    if service.Status.LoadBalancer == nil {
        return []string{}
    }
    
    if service.Status.LoadBalancer.Ingress == nil {
        return []string{}
    }
    
    var ips []string
    for _, ingress := range service.Status.LoadBalancer.Ingress {
        if ingress.IP != "" {
            ips = append(ips, ingress.IP)
            continue
        }
        if ingress.Hostname != "" {
            ips = append(ips, ingress.Hostname)
        }
    }
    return ips
}

// Example 2: Using default values and overwriting
func processUser(user *User) string {
    name := "Unknown"
    if user != nil {
        name = user.Name
    }
    return name
}

// Example 3: Early returns with error handling
func handleOperation() error {
    if err := validateInput(); err != nil {
        return fmt.Errorf("validation failed: %w", err)
    }
    
    if err := performOperation(); err != nil {
        return fmt.Errorf("operation failed: %w", err)
    }
    
    return nil
}
```

**Additional rules:**
* Return early when possible
* Only add comments that are impactful
* NEVER write obvious comments that restate what the code does
* **Create reusable components instead of repeating code patterns** - If you find yourself repeating the same conditional logic or styling patterns multiple times, extract it into a reusable component

## Vue Development Standards

### Component Communication Principles

**Props Down, Events Up**: Always follow unidirectional data flow
- Parent components pass data to children via **props**
- Child components communicate with parents via **emitted events**
- Never mutate props directly in child components

### Props Guidelines

**Define Props Explicitly**:
```typescript
// Use TypeScript with proper type definitions
interface Props {
  userId: string
  isActive?: boolean
  items: Array<Item>
}

const props = defineProps<Props>()
// Or with defaults:
const props = withDefaults(defineProps<Props>(), {
  isActive: false
})
```

**Props Best Practices**:
- Use descriptive, specific names (prefer `userName` over `name`)
- Make props readonly - never reassign or mutate
- Use computed properties or local refs to transform prop values
- Validate prop types in TypeScript interfaces
- Provide default values for optional props

### Event Emission

**Emit Events for Child-to-Parent Communication**:
```typescript
// Define emits explicitly
const emit = defineEmits<{
  update: [value: string]
  delete: [id: number]
  submit: [data: FormData]
}>()

// Emit with type safety
emit('update', newValue)
```

**Event Naming**:
- Use kebab-case for event names in templates: `@user-updated`
- Use camelCase in TypeScript: `emit('userUpdated')`
- Be descriptive and action-oriented: `submit`, `delete`, `update`

### State Management

**Local State**:
- Use `ref()` or `reactive()` for component-local state
- Keep state as close to where it's used as possible
- Don't lift state up unless multiple components need it

**Computed Properties**:
- Use `computed()` for derived state
- Never mutate data inside computed properties
- Prefer computed over methods for values that depend on reactive state

**Avoid Direct Mutations**:
```typescript
// ❌ BAD - Mutating prop
const props = defineProps<{ user: User }>()
props.user.name = 'New Name'

// ✅ GOOD - Emit event to parent
const emit = defineEmits<{ updateUser: [user: User] }>()
emit('updateUser', { ...props.user, name: 'New Name' })
```

### Component Structure

**Composition API (Script Setup)**:
- Always use `<script setup lang="ts">`
- Order: imports, props, emits, composables, computed, methods, lifecycle hooks
- Extract reusable logic into composables

**Template Best Practices**:
- Keep templates simple and readable
- Extract complex logic to computed properties or methods
- Use v-bind shorthand (`:`) and v-on shorthand (`@`)
- Prefer `v-if` over `v-show` for conditional rendering unless toggling frequently

### Avoid These Anti-Patterns

**❌ Never use `$parent` or `$refs` for component communication**
- Use props and events instead

**❌ Don't use `v-model` on props directly**
- Create a local ref or use `defineModel()` (Vue 3.4+)

**❌ Don't pass callbacks as props**
- Emit events instead

**❌ Avoid deeply nested prop drilling**
- Use provide/inject or a store for deeply nested data

### TypeScript Integration

- Define interfaces for all props, emits, and component-specific types
- Use generic types for reusable components
- Enable strict mode in TypeScript config
- Avoid `any` types - use `unknown` if type is truly unknown

### Composition Over Inheritance

- Use composables to share logic between components
- Prefer small, focused components over large multi-purpose ones
- Extract reusable UI patterns into components

<!-- BACKLOG.MD GUIDELINES START -->
# Instructions for the usage of Backlog.md CLI Tool

## Backlog.md: Comprehensive Project Management Tool via CLI

### Assistant Objective

Efficiently manage all project tasks, status, and documentation using the Backlog.md CLI, ensuring all project metadata
remains fully synchronized and up-to-date.

### Core Capabilities

- ✅ **Task Management**: Create, edit, assign, prioritize, and track tasks with full metadata
- ✅ **Search**: Fuzzy search across tasks, documents, and decisions with `backlog search`
- ✅ **Acceptance Criteria**: Granular control with add/remove/check/uncheck by index
- ✅ **Board Visualization**: Terminal-based Kanban board (`backlog board`) and web UI (`backlog browser`)
- ✅ **Git Integration**: Automatic tracking of task states across branches
- ✅ **Dependencies**: Task relationships and subtask hierarchies
- ✅ **Documentation & Decisions**: Structured docs and architectural decision records
- ✅ **Export & Reporting**: Generate markdown reports and board snapshots
- ✅ **AI-Optimized**: `--plain` flag provides clean text output for AI processing

### Why This Matters to You (AI Agent)

1. **Comprehensive system** - Full project management capabilities through CLI
2. **The CLI is the interface** - All operations go through `backlog` commands
3. **Unified interaction model** - You can use CLI for both reading (`backlog task 1 --plain`) and writing (
   `backlog task edit 1`)
4. **Metadata stays synchronized** - The CLI handles all the complex relationships

### Key Understanding

- **Tasks** live in `backlog/tasks/` as `task-<id> - <title>.md` files
- **You interact via CLI only**: `backlog task create`, `backlog task edit`, etc.
- **Use `--plain` flag** for AI-friendly output when viewing/listing
- **Never bypass the CLI** - It handles Git, metadata, file naming, and relationships

---

# ⚠️ CRITICAL: NEVER EDIT TASK FILES DIRECTLY. Edit Only via CLI

**ALL task operations MUST use the Backlog.md CLI commands**

- ✅ **DO**: Use `backlog task edit` and other CLI commands
- ✅ **DO**: Use `backlog task create` to create new tasks
- ✅ **DO**: Use `backlog task edit <id> --check-ac <index>` to mark acceptance criteria
- ❌ **DON'T**: Edit markdown files directly
- ❌ **DON'T**: Manually change checkboxes in files
- ❌ **DON'T**: Add or modify text in task files without using CLI

**Why?** Direct file editing breaks metadata synchronization, Git tracking, and task relationships.

---

## 1. Source of Truth & File Structure

### 📖 **UNDERSTANDING** (What you'll see when reading)

- Markdown task files live under **`backlog/tasks/`** (drafts under **`backlog/drafts/`**)
- Files are named: `task-<id> - <title>.md` (e.g., `task-42 - Add GraphQL resolver.md`)
- Project documentation is in **`backlog/docs/`**
- Project decisions are in **`backlog/decisions/`**

### 🔧 **ACTING** (How to change things)

- **All task operations MUST use the Backlog.md CLI tool**
- This ensures metadata is correctly updated and the project stays in sync
- **Always use `--plain` flag** when listing or viewing tasks for AI-friendly text output

---

## 2. Common Mistakes to Avoid

### ❌ **WRONG: Direct File Editing**

```markdown
# DON'T DO THIS:

1. Open backlog/tasks/task-7 - Feature.md in editor
2. Change "- [ ]" to "- [x]" manually
3. Add notes directly to the file
4. Save the file
```

### ✅ **CORRECT: Using CLI Commands**

```bash
# DO THIS INSTEAD:
backlog task edit 7 --check-ac 1  # Mark AC #1 as complete
backlog task edit 7 --notes "Implementation complete"  # Add notes
backlog task edit 7 -s "In Progress" -a @agent-k  # Multiple commands: change status and assign the task when you start working on the task
```

---

## 3. Understanding Task Format (Read-Only Reference)

⚠️ **FORMAT REFERENCE ONLY** - The following sections show what you'll SEE in task files.
**Never edit these directly! Use CLI commands to make changes.**

### Task Structure You'll See

```markdown
---
id: task-42
title: Add GraphQL resolver
status: To Do
assignee: [@sara]
labels: [backend, api]
---

## Description

Brief explanation of the task purpose.

## Acceptance Criteria

<!-- AC:BEGIN -->

- [ ] #1 First criterion
- [x] #2 Second criterion (completed)
- [ ] #3 Third criterion

<!-- AC:END -->

## Implementation Plan

1. Research approach
2. Implement solution

## Implementation Notes

Summary of what was done.
```

### How to Modify Each Section

| What You Want to Change | CLI Command to Use                                       |
|-------------------------|----------------------------------------------------------|
| Title                   | `backlog task edit 42 -t "New Title"`                    |
| Status                  | `backlog task edit 42 -s "In Progress"`                  |
| Assignee                | `backlog task edit 42 -a @sara`                          |
| Labels                  | `backlog task edit 42 -l backend,api`                    |
| Description             | `backlog task edit 42 -d "New description"`              |
| Add AC                  | `backlog task edit 42 --ac "New criterion"`              |
| Check AC #1             | `backlog task edit 42 --check-ac 1`                      |
| Uncheck AC #2           | `backlog task edit 42 --uncheck-ac 2`                    |
| Remove AC #3            | `backlog task edit 42 --remove-ac 3`                     |
| Add Plan                | `backlog task edit 42 --plan "1. Step one\n2. Step two"` |
| Add Notes (replace)     | `backlog task edit 42 --notes "What I did"`              |
| Append Notes            | `backlog task edit 42 --append-notes "Another note"` |

---

## 4. Defining Tasks

### Creating New Tasks

**Always use CLI to create tasks:**

```bash
# Example
backlog task create "Task title" -d "Description" --ac "First criterion" --ac "Second criterion"
```

### Title (one liner)

Use a clear brief title that summarizes the task.

### Description (The "why")

Provide a concise summary of the task purpose and its goal. Explains the context without implementation details.

### Acceptance Criteria (The "what")

**Understanding the Format:**

- Acceptance criteria appear as numbered checkboxes in the markdown files
- Format: `- [ ] #1 Criterion text` (unchecked) or `- [x] #1 Criterion text` (checked)

**Managing Acceptance Criteria via CLI:**

⚠️ **IMPORTANT: How AC Commands Work**

- **Adding criteria (`--ac`)** accepts multiple flags: `--ac "First" --ac "Second"` ✅
- **Checking/unchecking/removing** accept multiple flags too: `--check-ac 1 --check-ac 2` ✅
- **Mixed operations** work in a single command: `--check-ac 1 --uncheck-ac 2 --remove-ac 3` ✅

```bash
# Examples

# Add new criteria (MULTIPLE values allowed)
backlog task edit 42 --ac "User can login" --ac "Session persists"

# Check specific criteria by index (MULTIPLE values supported)
backlog task edit 42 --check-ac 1 --check-ac 2 --check-ac 3  # Check multiple ACs
# Or check them individually if you prefer:
backlog task edit 42 --check-ac 1    # Mark #1 as complete
backlog task edit 42 --check-ac 2    # Mark #2 as complete

# Mixed operations in single command
backlog task edit 42 --check-ac 1 --uncheck-ac 2 --remove-ac 3

# ❌ STILL WRONG - These formats don't work:
# backlog task edit 42 --check-ac 1,2,3  # No comma-separated values
# backlog task edit 42 --check-ac 1-3    # No ranges
# backlog task edit 42 --check 1         # Wrong flag name

# Multiple operations of same type
backlog task edit 42 --uncheck-ac 1 --uncheck-ac 2  # Uncheck multiple ACs
backlog task edit 42 --remove-ac 2 --remove-ac 4    # Remove multiple ACs (processed high-to-low)
```

**Key Principles for Good ACs:**

- **Outcome-Oriented:** Focus on the result, not the method.
- **Testable/Verifiable:** Each criterion should be objectively testable
- **Clear and Concise:** Unambiguous language
- **Complete:** Collectively cover the task scope
- **User-Focused:** Frame from end-user or system behavior perspective

Good Examples:

- "User can successfully log in with valid credentials"
- "System processes 1000 requests per second without errors"
- "CLI preserves literal newlines in description/plan/notes; `\\n` sequences are not auto‑converted"

Bad Example (Implementation Step):

- "Add a new function handleLogin() in auth.ts"
- "Define expected behavior and document supported input patterns"

### Task Breakdown Strategy

1. Identify foundational components first
2. Create tasks in dependency order (foundations before features)
3. Ensure each task delivers value independently
4. Avoid creating tasks that block each other

### Task Requirements

- Tasks must be **atomic** and **testable** or **verifiable**
- Each task should represent a single unit of work for one PR
- **Never** reference future tasks (only tasks with id < current task id)
- Ensure tasks are **independent** and don't depend on future work

---

## 5. Implementing Tasks

### 5.1. First step when implementing a task

The very first things you must do when you take over a task are:

* set the task in progress
* assign it to yourself

```bash
# Example
backlog task edit 42 -s "In Progress" -a @{myself}
```

### 5.2. Create an Implementation Plan (The "how")

Previously created tasks contain the why and the what. Once you are familiar with that part you should think about a
plan on **HOW** to tackle the task and all its acceptance criteria. This is your **Implementation Plan**.
First do a quick check to see if all the tools that you are planning to use are available in the environment you are
working in.   
When you are ready, write it down in the task so that you can refer to it later.

```bash
# Example
backlog task edit 42 --plan "1. Research codebase for references\n2Research on internet for similar cases\n3. Implement\n4. Test"
```

## 5.3. Implementation

Once you have a plan, you can start implementing the task. This is where you write code, run tests, and make sure
everything works as expected. Follow the acceptance criteria one by one and MARK THEM AS COMPLETE as soon as you
finish them.

### 5.4 Implementation Notes (PR description)

When you are done implementing a tasks you need to prepare a PR description for it.
Because you cannot create PRs directly, write the PR as a clean description in the task notes.
Append notes progressively during implementation using `--append-notes`:

```
backlog task edit 42 --append-notes "Implemented X" --append-notes "Added tests"
```

```bash
# Example
backlog task edit 42 --notes "Implemented using pattern X because Reason Y, modified files Z and W"
```

**IMPORTANT**: Do NOT include an Implementation Plan when creating a task. The plan is added only after you start the
implementation.

- Creation phase: provide Title, Description, Acceptance Criteria, and optionally labels/priority/assignee.
- When you begin work, switch to edit, set the task in progress and assign to yourself
  `backlog task edit <id> -s "In Progress" -a "..."`.
- Think about how you would solve the task and add the plan: `backlog task edit <id> --plan "..."`.
- Add Implementation Notes only after completing the work: `backlog task edit <id> --notes "..."` (replace) or append progressively using `--append-notes`.

## Phase discipline: What goes where

- Creation: Title, Description, Acceptance Criteria, labels/priority/assignee.
- Implementation: Implementation Plan (after moving to In Progress and assigning to yourself).
- Wrap-up: Implementation Notes (Like a PR description), AC and Definition of Done checks.

**IMPORTANT**: Only implement what's in the Acceptance Criteria. If you need to do more, either:

1. Update the AC first: `backlog task edit 42 --ac "New requirement"`
2. Or create a new follow up task: `backlog task create "Additional feature"`

---

## 6. Typical Workflow

```bash
# 1. Identify work
backlog task list -s "To Do" --plain

# 2. Read task details
backlog task 42 --plain

# 3. Start work: assign yourself & change status
backlog task edit 42 -s "In Progress" -a @myself

# 4. Add implementation plan
backlog task edit 42 --plan "1. Analyze\n2. Refactor\n3. Test"

# 5. Work on the task (write code, test, etc.)

# 6. Mark acceptance criteria as complete (supports multiple in one command)
backlog task edit 42 --check-ac 1 --check-ac 2 --check-ac 3  # Check all at once
# Or check them individually if preferred:
# backlog task edit 42 --check-ac 1
# backlog task edit 42 --check-ac 2
# backlog task edit 42 --check-ac 3

# 7. Add implementation notes (PR Description)
backlog task edit 42 --notes "Refactored using strategy pattern, updated tests"

# 8. Mark task as done
backlog task edit 42 -s Done
```

---

## 7. Definition of Done (DoD)

A task is **Done** only when **ALL** of the following are complete:

### ✅ Via CLI Commands:

1. **All acceptance criteria checked**: Use `backlog task edit <id> --check-ac <index>` for each
2. **Implementation notes added**: Use `backlog task edit <id> --notes "..."`
3. **Status set to Done**: Use `backlog task edit <id> -s Done`

### ✅ Via Code/Testing:

4. **Tests pass**: Run test suite and linting
5. **Documentation updated**: Update relevant docs if needed
6. **Code reviewed**: Self-review your changes
7. **No regressions**: Performance, security checks pass

⚠️ **NEVER mark a task as Done without completing ALL items above**

---

## 8. Finding Tasks and Content with Search

When users ask you to find tasks related to a topic, use the `backlog search` command with `--plain` flag:

```bash
# Search for tasks about authentication
backlog search "auth" --plain

# Search only in tasks (not docs/decisions)
backlog search "login" --type task --plain

# Search with filters
backlog search "api" --status "In Progress" --plain
backlog search "bug" --priority high --plain
```

**Key points:**
- Uses fuzzy matching - finds "authentication" when searching "auth"
- Searches task titles, descriptions, and content
- Also searches documents and decisions unless filtered with `--type task`
- Always use `--plain` flag for AI-readable output

---

## 9. Quick Reference: DO vs DON'T

### Viewing and Finding Tasks

| Task         | ✅ DO                        | ❌ DON'T                         |
|--------------|-----------------------------|---------------------------------|
| View task    | `backlog task 42 --plain`   | Open and read .md file directly |
| List tasks   | `backlog task list --plain` | Browse backlog/tasks folder     |
| Check status | `backlog task 42 --plain`   | Look at file content            |
| Find by topic| `backlog search "auth" --plain` | Manually grep through files |

### Modifying Tasks

| Task          | ✅ DO                                 | ❌ DON'T                           |
|---------------|--------------------------------------|-----------------------------------|
| Check AC      | `backlog task edit 42 --check-ac 1`  | Change `- [ ]` to `- [x]` in file |
| Add notes     | `backlog task edit 42 --notes "..."` | Type notes into .md file          |
| Change status | `backlog task edit 42 -s Done`       | Edit status in frontmatter        |
| Add AC        | `backlog task edit 42 --ac "New"`    | Add `- [ ] New` to file           |

---

## 10. Complete CLI Command Reference

### Task Creation

| Action           | Command                                                                             |
|------------------|-------------------------------------------------------------------------------------|
| Create task      | `backlog task create "Title"`                                                       |
| With description | `backlog task create "Title" -d "Description"`                                      |
| With AC          | `backlog task create "Title" --ac "Criterion 1" --ac "Criterion 2"`                 |
| With all options | `backlog task create "Title" -d "Desc" -a @sara -s "To Do" -l auth --priority high` |
| Create draft     | `backlog task create "Title" --draft`                                               |
| Create subtask   | `backlog task create "Title" -p 42`                                                 |

### Task Modification

| Action           | Command                                     |
|------------------|---------------------------------------------|
| Edit title       | `backlog task edit 42 -t "New Title"`       |
| Edit description | `backlog task edit 42 -d "New description"` |
| Change status    | `backlog task edit 42 -s "In Progress"`     |
| Assign           | `backlog task edit 42 -a @sara`             |
| Add labels       | `backlog task edit 42 -l backend,api`       |
| Set priority     | `backlog task edit 42 --priority high`      |

### Acceptance Criteria Management

| Action              | Command                                                                     |
|---------------------|-----------------------------------------------------------------------------|
| Add AC              | `backlog task edit 42 --ac "New criterion" --ac "Another"`                  |
| Remove AC #2        | `backlog task edit 42 --remove-ac 2`                                        |
| Remove multiple ACs | `backlog task edit 42 --remove-ac 2 --remove-ac 4`                          |
| Check AC #1         | `backlog task edit 42 --check-ac 1`                                         |
| Check multiple ACs  | `backlog task edit 42 --check-ac 1 --check-ac 3`                            |
| Uncheck AC #3       | `backlog task edit 42 --uncheck-ac 3`                                       |
| Mixed operations    | `backlog task edit 42 --check-ac 1 --uncheck-ac 2 --remove-ac 3 --ac "New"` |

### Task Content

| Action           | Command                                                  |
|------------------|----------------------------------------------------------|
| Add plan         | `backlog task edit 42 --plan "1. Step one\n2. Step two"` |
| Add notes        | `backlog task edit 42 --notes "Implementation details"`  |
| Add dependencies | `backlog task edit 42 --dep task-1 --dep task-2`         |

### Multi‑line Input (Description/Plan/Notes)

The CLI preserves input literally. Shells do not convert `\n` inside normal quotes. Use one of the following to insert real newlines:

- Bash/Zsh (ANSI‑C quoting):
  - Description: `backlog task edit 42 --desc $'Line1\nLine2\n\nFinal'`
  - Plan: `backlog task edit 42 --plan $'1. A\n2. B'`
  - Notes: `backlog task edit 42 --notes $'Done A\nDoing B'`
  - Append notes: `backlog task edit 42 --append-notes $'Progress update line 1\nLine 2'`
- POSIX portable (printf):
  - `backlog task edit 42 --notes "$(printf 'Line1\nLine2')"`
- PowerShell (backtick n):
  - `backlog task edit 42 --notes "Line1`nLine2"`

Do not expect `"...\n..."` to become a newline. That passes the literal backslash + n to the CLI by design.

Descriptions support literal newlines; shell examples may show escaped `\\n`, but enter a single `\n` to create a newline.

### Implementation Notes Formatting

- Keep implementation notes human-friendly and PR-ready: use short paragraphs or
  bullet lists instead of a single long line.
- Lead with the outcome, then add supporting details (e.g., testing, follow-up
  actions) on separate lines or bullets.
- Prefer Markdown bullets (`-` for unordered, `1.` for ordered) so Maintainers
  can paste notes straight into GitHub without additional formatting.
- When using CLI flags like `--append-notes`, remember to include explicit
  newlines. Example:

  ```bash
  backlog task edit 42 --append-notes $'- Added new API endpoint\n- Updated tests\n- TODO: monitor staging deploy'
  ```

### Task Operations

| Action             | Command                                      |
|--------------------|----------------------------------------------|
| View task          | `backlog task 42 --plain`                    |
| List tasks         | `backlog task list --plain`                  |
| Search tasks       | `backlog search "topic" --plain`              |
| Search with filter | `backlog search "api" --status "To Do" --plain` |
| Filter by status   | `backlog task list -s "In Progress" --plain` |
| Filter by assignee | `backlog task list -a @sara --plain`         |
| Archive task       | `backlog task archive 42`                    |
| Demote to draft    | `backlog task demote 42`                     |

---

## Common Issues

| Problem              | Solution                                                           |
|----------------------|--------------------------------------------------------------------|
| Task not found       | Check task ID with `backlog task list --plain`                     |
| AC won't check       | Use correct index: `backlog task 42 --plain` to see AC numbers     |
| Changes not saving   | Ensure you're using CLI, not editing files                         |
| Metadata out of sync | Re-edit via CLI to fix: `backlog task edit 42 -s <current-status>` |

---

## Remember: The Golden Rule

**🎯 If you want to change ANYTHING in a task, use the `backlog task edit` command.**
**📖 Use CLI to read tasks, exceptionally READ task files directly, never WRITE to them.**

Full help available: `backlog --help`

<!-- BACKLOG.MD GUIDELINES END -->
