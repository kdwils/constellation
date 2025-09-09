# Constellation

A simple dashboard for visualizing Kubernetes traffic relationships. Shows how your ingresses, services, and pods connect to each other.

## What it does

Constellation watches your Kubernetes cluster and builds a hierarchy showing how traffic flows between resources:

- **Namespace** → **HTTPRoute/Ingress** → **Service** → **Pod(s)**
- **Namespace** → **Service** → **Pod(s)** (for internal traffic)
- **Namespace** → **Pod(s)** (orphaned pods with no service)

No configuration needed - just point it at your cluster and it figures out the relationships automatically.

## Running it

### Local development

Backend:
```bash
cargo run
```

Frontend:
```bash
cd frontend && npm run dev
```

### Production

```bash
# Build everything
cd frontend && npm run build && cd ..
cargo build --release

# Run the server
./target/release/constellation
```

The server serves both the API and static files on the same port. Visit `/` for the dashboard, `/state` for raw JSON.

### Docker

```bash
docker build -t constellation .
docker run -p 8080:8080 constellation
```

You'll need to mount your kubeconfig or run it in-cluster with proper RBAC.

## Architecture

- **Backend**: Rust server that watches the Kubernetes API using the kube-rs client
- **Frontend**: React app that fetches from `/state` and renders the hierarchy
- **Data flow**: K8s API → Rust watchers → shared state → JSON endpoint → React UI

The backend keeps an in-memory representation of your cluster's traffic relationships and updates it in real-time as resources change.

## Development

Run tests:
```bash
cargo test
```

Check formatting:
```bash
cargo fmt --check
cargo clippy
```

## What's supported

- Namespaces, Services, Pods (core resources)
- HTTPRoutes (Gateway API)
- Basic Ingress support planned

Currently focused on getting the MVP working with clean resource relationships.