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
* Structure code to not use else statements by returning early or checking opposites
* Return early when possible
* Only add comments that are impactful