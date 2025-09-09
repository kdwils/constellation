# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Constellation is a self-hosted dashboard that intends to provide an out- of-the-box view of the current state of traffic to a pod.

Initial MVP state:
* Achieve a relationship between namespaces, ingresses, httproutes, services, and pods.
* Namespaces are always the parent resource
* Example valid relationships
    - namespace -> ingress -> service -> pod(s) traffic can be route to pod via ingress or in cluster dns
    - namepsace -> httproute -> service -> pod(s) traffic can be route to pod via httproute or in cluster dns
    - namespace -> service -> pod(s) traffic can be routed to pod via in cluster dns
    - namespace -> pod(s) has no way to route traffic to a pod

Future state: 
* Provide a healtcheck dashboard that shows the relationship between ingress to a pod.
* Provide configuration via annotations on resources
* The contents of `thoughts.md`

It is written in rust.

## Project structure

The frontend and backend are served from the same server.

The cluster state is served at `/state/`.

Static files are served at `/` and default to `index.html` if no static file is found.

## Architecture

This is a Kubernetes resource monitoring tool with:
- **Backend**: Rust server that watches Kubernetes API and serves JSON state
- **Frontend**: React application that visualizes the resource relationships
- **Data Flow**: Kubernetes API → Rust backend → JSON state endpoint → React frontend

## Key File Locations

- Main backend entry: `src/main.rs`
- Router configuration: `src/router.rs`
- Watcher logic: `src/watcher.rs`
- Frontend components: `frontend/src/`
- State data: `frontend/state.json`

## Development Commands

### Running the backend
```bash
cargo run
```

### Running the frontend
```bash
cd frontend && npm run dev
```

### Building
```bash
# Build frontend for production
cd frontend && npm run build

# Build backend for production
cargo build --release

# Full production build
cd frontend && npm run build && cd .. && cargo build --release
```

### Testing
```bash
# Run backend tests
cargo test

# Run frontend tests
cd frontend && npm test
```

### Code Quality
```bash
# Check Rust formatting and linting
cargo fmt --check
cargo clippy

# Check frontend types and linting
cd frontend && npm run type-check
cd frontend && npm run lint
```

### Coding Standards

**NEVER use `else` statements. This is strictly forbidden.**

Structure code to avoid `else` by:
- Returning early from functions
- Using guard clauses
- Inverting conditions when needed
- Using functional programming patterns

#### Examples

**❌ BAD - Using else statements:**
```rust
// Example 1: Nested if-else hell (FORBIDDEN)
if let Some(status) = &service.status {
    if let Some(load_balancer) = &status.load_balancer {
        if let Some(ingress_list) = &load_balancer.ingress {
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
fn process_user(user: Option<User>) -> String {
    if let Some(u) = user {
        u.name
    } else {
        "Unknown".to_string()
    }
}
```

**✅ GOOD - No else statements:**
```rust
// Example 1: Early returns and guard clauses
fn extract_load_balancer_ips(service: &Service) -> Vec<String> {
    let Some(status) = &service.status else {
        return Vec::new();
    };
    
    let Some(load_balancer) = &status.load_balancer else {
        return Vec::new();
    };
    
    let Some(ingress_list) = &load_balancer.ingress else {
        return Vec::new();
    };
    
    ingress_list
        .iter()
        .filter_map(|ingress| ingress.ip.as_ref().or(ingress.hostname.as_ref()))
        .cloned()
        .collect()
}

// Example 2: Using unwrap_or/unwrap_or_default
fn process_user(user: Option<User>) -> String {
    user.map(|u| u.name).unwrap_or_else(|| "Unknown".to_string())
}

// Example 3: Pattern matching without else
fn handle_result(result: Result<i32, &str>) -> i32 {
    match result {
        Ok(value) => value,
        Err(_) => return 0,
    }
}
```

**Additional rules:**
* Return early when possible
* Only add comments that are impactful
* NEVER write obvious comments that restate what the code does