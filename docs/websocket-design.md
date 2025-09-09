# WebSocket Real-time Updates Design

## Overview
Design document for implementing real-time WebSocket updates to eliminate the need for manual page refreshes in the Constellation dashboard.

## Current Architecture Analysis

**Current Flow:**
1. Kubernetes watchers → Store updates → `build_initial_relationships()` → Update `State.hierarchy`
2. HTTP `/state` endpoint reads from `State.hierarchy` 
3. Frontend polls or manually refreshes

## 1. WebSocket Message Protocol Design

```rust
#[derive(Serialize, Deserialize)]
pub enum WebSocketMessage {
    // Full state snapshot (on connect)
    FullState { 
        hierarchy: Vec<HierarchyNode> 
    },
    
    // Incremental updates
    ResourceAdded { 
        namespace: String,
        resource: HierarchyNode 
    },
    ResourceUpdated { 
        namespace: String,
        resource: HierarchyNode 
    },
    ResourceDeleted { 
        namespace: String,
        resource_name: String,
        resource_kind: ResourceKind 
    },
    
    // Relationship changes
    RelationshipUpdated { 
        namespace: String,
        hierarchy: Vec<HierarchyNode>  // Just affected namespace
    },
    
    // Connection management
    Ping,
    Pong,
    
    // Pod churn handling
    NamespaceSummaryUpdate {
        namespace: String,
        pod_summary: PodSummary,
        individual_pods: Option<Vec<HierarchyNode>>, // Only if churn is low
    },
    
    HighChurnWarning {
        namespace: String,
        affected_services: Vec<String>,
        churn_rate: f64, // pods/minute
    },
}

#[derive(Serialize)]
pub struct PodSummary {
    pub total: u32,
    pub ready: u32,
    pub pending: u32,
    pub terminating: u32,
    pub failed: u32,
}
```

## 2. Backend Changes (Rust)

### Dependencies to Add
```toml
# Cargo.toml
axum-websockets = "0.1"
tokio-tungstenite = "0.20"
tokio = { version = "1.0", features = ["sync"] }
futures-util = "0.3"
```

### New WebSocket State Management
```rust
// src/websocket.rs (new file)
use tokio::sync::broadcast;

pub struct WebSocketState {
    pub sender: broadcast::Sender<WebSocketMessage>,
    pub debouncer: Debouncer,
}

impl WebSocketState {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { 
            sender,
            debouncer: Debouncer::new(),
        }
    }
    
    pub fn broadcast(&self, message: WebSocketMessage) {
        let _ = self.sender.send(message);
    }
}
```

### Watcher Modifications
```rust
// src/watcher.rs changes
pub struct Context {
    state: State,
    websocket_state: WebSocketState,  // Add this
    // ... existing fields
}

// In each watcher function (pod_watcher, service_watcher, etc.)
// After updating the store, broadcast the change:
ctx.websocket_state.broadcast(WebSocketMessage::ResourceUpdated {
    namespace: resource.namespace,
    resource: updated_hierarchy_node,
});
```

### Router WebSocket Endpoint
```rust
// src/router.rs changes
use axum_websockets::{WebSocket, WebSocketUpgrade};

pub async fn new_router(app_state: AppState, ws_state: WebSocketState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/state", get(state))
        .route("/ws", get(websocket_handler))  // Add this
        .with_state((app_state, ws_state))
        // ... rest unchanged
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State((app_state, ws_state)): State<(AppState, WebSocketState)>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, app_state, ws_state))
}

async fn handle_websocket(socket: WebSocket, app_state: AppState, ws_state: WebSocketState) {
    // Send initial full state
    // Subscribe to broadcast channel
    // Handle client disconnection
}
```

## 3. Frontend Changes (React/TypeScript)

### WebSocket Hook
```typescript
// src/hooks/useWebSocket.ts (new file)
export function useWebSocket(url: string) {
    const [socket, setSocket] = useState<WebSocket | null>(null);
    const [connectionState, setConnectionState] = useState<'connecting' | 'connected' | 'disconnected'>('disconnected');
    const [data, setData] = useState<ResourceNode[]>([]);
    
    useEffect(() => {
        const ws = new WebSocket(url);
        
        ws.onopen = () => setConnectionState('connected');
        ws.onclose = () => setConnectionState('disconnected');
        ws.onmessage = (event) => {
            const message = JSON.parse(event.data);
            handleWebSocketMessage(message, setData);
        };
        
        setSocket(ws);
        return () => ws.close();
    }, [url]);
    
    return { data, connectionState, socket };
}

function handleWebSocketMessage(message: WebSocketMessage, setData: Function) {
    switch (message.type) {
        case 'FullState':
            setData(message.hierarchy);
            break;
        case 'ResourceUpdated':
            setData(prev => updateResource(prev, message));
            break;
        case 'RelationshipUpdated':
            setData(prev => updateNamespace(prev, message));
            break;
        // ... other cases
    }
}
```

### Dashboard Component Update
```typescript
// src/Dashboard.tsx changes
export function Dashboard() {
    const { data: nodes, connectionState } = useWebSocket(`ws://${window.location.host}/ws`);
    
    return (
        <div>
            <ConnectionIndicator state={connectionState} />
            <ResourceTree nodes={nodes} />
        </div>
    );
}
```

## 4. Pod Churn Handling Strategy

### Problem
Rapid pod creation/deletion from VPA, HPA, rolling updates creates UI noise:
```
2025-09-09T13:33:48.476507Z  INFO constellation::watcher: pod applied: blog-58d84b9659-vcpkw
2025-09-09T13:33:48.477274Z  INFO constellation::watcher: pod applied: blog-58d84b9659-mdf29
2025-09-09T13:33:48.490615Z  INFO constellation::watcher: pod applied: blog-58d84b9659-vcpkw
2025-09-09T13:33:48.584823Z  INFO constellation::watcher: pod applied: blog-58d84b9659-vcpkw
2025-09-09T13:33:48.716931Z  INFO constellation::watcher: pod applied: blog-58d84b9659-mdf29
2025-09-09T13:33:49.275375Z  INFO constellation::watcher: pod applied: blog-58d84b9659-mdf29
2025-09-09T13:33:49.276493Z  INFO constellation::watcher: pod deleted: blog-58d84b9659-mdf29
2025-09-09T13:33:51.200789Z  INFO constellation::watcher: pod applied: blog-58d84b9659-vcpkw
```

### Solution 1: Event Debouncing/Batching

```rust
// src/websocket.rs
struct Debouncer {
    pending_updates: Arc<RwLock<HashMap<String, PendingUpdate>>>,
    timer: tokio::time::Interval,
}

struct PendingUpdate {
    namespace: String,
    last_update: Instant,
    needs_rebuild: bool,
}

impl Debouncer {
    // Batch updates for 500ms before sending
    pub async fn schedule_update(&self, namespace: &str) {
        let mut pending = self.pending_updates.write().await;
        pending.insert(namespace.to_string(), PendingUpdate {
            namespace: namespace.to_string(),
            last_update: Instant::now(),
            needs_rebuild: true,
        });
    }
    
    // Send batched updates every 500ms
    pub async fn flush_pending(&self, ws_state: &WebSocketState) {
        let mut pending = self.pending_updates.write().await;
        for (namespace, update) in pending.drain() {
            if update.last_update.elapsed() > Duration::from_millis(500) {
                // Send namespace-level update
                ws_state.broadcast_namespace_update(&namespace).await;
            }
        }
    }
}
```

### Solution 2: Pod Lifecycle State Tracking

```rust
#[derive(Debug, Clone)]
pub enum PodState {
    Pending,
    Running,
    Terminating,
    Failed,
    Succeeded,
}

// Track pod states to reduce UI flicker
pub struct PodTracker {
    pods: HashMap<String, (PodState, Instant)>,
}

impl PodTracker {
    pub fn should_show_pod(&self, pod_name: &str, state: PodState) -> bool {
        match state {
            PodState::Terminating => {
                // Show terminating pods for at least 2 seconds
                // to avoid flicker during rolling updates
                if let Some((prev_state, timestamp)) = self.pods.get(pod_name) {
                    timestamp.elapsed() > Duration::from_secs(2)
                } else { true }
            }
            PodState::Pending => {
                // Only show pending pods after 1 second
                // to avoid showing pods that immediately transition
                if let Some((_, timestamp)) = self.pods.get(pod_name) {
                    timestamp.elapsed() > Duration::from_secs(1)
                } else { false }
            }
            _ => true
        }
    }
}
```

### Solution 3: UI-Level Smoothing

```typescript
// Frontend: Smooth pod transitions
interface PodTransition {
    action: 'appearing' | 'disappearing' | 'stable';
    timestamp: number;
}

export function useSmoothedPods(rawPods: Pod[]) {
    const [smoothedPods, setSmoothedPods] = useState<Pod[]>([]);
    const [transitions, setTransitions] = useState<Map<string, PodTransition>>(new Map());
    
    useEffect(() => {
        const timer = setTimeout(() => {
            // Apply transitions with delay
            const now = Date.now();
            const filtered = rawPods.filter(pod => {
                const transition = transitions.get(pod.name);
                if (transition?.action === 'disappearing') {
                    // Keep showing for 1 second after deletion
                    return (now - transition.timestamp) < 1000;
                }
                return true;
            });
            
            setSmoothedPods(filtered);
        }, 300); // 300ms debounce
        
        return () => clearTimeout(timer);
    }, [rawPods, transitions]);
    
    return smoothedPods;
}
```

### Solution 4: Message Filtering Strategy

```rust
// Only send websocket updates for "meaningful" changes
pub fn should_broadcast_pod_event(
    event_type: &str,
    pod: &Pod,
    previous_pods: &HashMap<String, Pod>
) -> bool {
    match event_type {
        "ADDED" => {
            // Only broadcast if pod reaches Running state
            // or has been Pending for > 5 seconds
            pod.status.phase == "Running" || 
            pod.metadata.creation_timestamp.elapsed() > Duration::from_secs(5)
        }
        "DELETED" => {
            // Always broadcast deletions, but debounce them
            true
        }
        "MODIFIED" => {
            // Only broadcast if meaningful fields changed
            if let Some(prev_pod) = previous_pods.get(&pod.name) {
                pod.status.phase != prev_pod.status.phase ||
                pod.spec != prev_pod.spec
            } else { true }
        }
        _ => false
    }
}
```

### Solution 5: Aggregation at Service Level

```typescript
// Show pod count instead of individual pods during churn
interface ServiceSummary {
    name: string;
    podCount: number;
    readyPods: number;
    pendingPods: number;
    terminatingPods: number;
    showIndividualPods: boolean; // Toggle based on churn rate
}

function shouldShowIndividualPods(pods: Pod[]): boolean {
    const recentChanges = pods.filter(p => 
        Date.now() - p.lastModified < 30000 // 30 seconds
    ).length;
    
    // If more than 3 pods changed recently, show summary
    return recentChanges <= 3;
}
```

## 5. Error Handling & Reconnection Strategy

### Backend Resilience
```rust
// Handle client disconnections gracefully
// Implement connection heartbeat (ping/pong)
// Buffer messages during temporary disconnections
// Rate limiting for broadcast messages
```

### Frontend Reconnection Logic
```typescript
// Exponential backoff reconnection
// Detect network issues vs server issues
// Show connection status to user
// Fallback to HTTP polling if websocket fails
// Request full state refresh on reconnect
```

## 6. Migration Strategy

### Phase 1: Add WebSocket Support
- Keep existing HTTP `/state` endpoint
- Add WebSocket endpoint alongside
- WebSocket sends full state snapshots initially

### Phase 2: Incremental Updates  
- Implement granular update messages
- Optimize for minimal bandwidth usage

### Phase 3: Remove HTTP Polling
- Make WebSocket the primary mechanism
- Keep HTTP endpoint as fallback

## Implementation Complexity: **Medium**

**Why it's manageable:**
- Kubernetes watcher infrastructure already exists
- Changes are mostly additive (keeping HTTP endpoint)
- Well-defined message protocol
- React state updates are straightforward

**Main challenges:**
- WebSocket connection management
- Handling partial updates correctly
- Testing reconnection scenarios
- Managing multiple concurrent connections
- Pod churn noise in UI

## Recommended Implementation Order:

1. **Start with debouncing** - Easiest win, reduces message frequency
2. **Add pod state tracking** - Prevents showing very short-lived pods
3. **Implement UI smoothing** - Makes transitions less jarring
4. **Add service-level aggregation** - For extreme churn scenarios
5. **Basic WebSocket infrastructure** - Full state snapshots
6. **Incremental updates** - Optimize bandwidth
7. **Advanced reconnection logic** - Production readiness

This approach will make the dashboard much more usable during deployments, scaling events, and other operational activities that cause rapid pod changes.