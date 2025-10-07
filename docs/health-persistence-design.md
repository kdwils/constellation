# Long-Term Health Check Persistence Design

**Status:** Design Proposal
**Created:** 2025-10-06
**Reference:** Inspired by [status.plex.tv](https://status.plex.tv/)

## Problem Statement

Currently, Constellation's health check system has significant limitations:

- **Limited history**: Only keeps last 100 health checks in memory (~50 minutes at 30s intervals)
- **No persistence**: All health data is lost on pod restart
- **No long-term visibility**: Cannot see uptime trends over days/weeks/months
- **No SLA tracking**: Cannot calculate uptime percentages over meaningful time periods

Real-world status dashboards like Plex show 90 days of uptime history with visual trends and multi-period uptime percentages.

## Current State Analysis

### What Works Now

- Health checks run every 30 seconds ([internal/controller/health_checker.go:22](../internal/controller/health_checker.go#L22))
- History kept in memory up to 100 entries ([internal/controller/health_checker.go:24](../internal/controller/health_checker.go#L24))
- When history exceeds 100 entries, old entries are dropped ([internal/controller/health_checker.go:402-404](../internal/controller/health_checker.go#L402-404))
- Uptime percentage calculated from in-memory history ([internal/controller/health_checker.go:412-425](../internal/controller/health_checker.go#L412-425))

### Key Code Locations

- **HealthChecker**: `internal/controller/health_checker.go`
  - Performs HTTP health checks every 30s
  - Stores last 100 checks in `ServiceHealthInfo.History`
  - Calculates uptime percentage from history

- **StateManager**: `internal/controller/state_manager.go`
  - Maintains in-memory health data (`healthData map[string]*ServiceHealthInfo`)
  - Updates hierarchy with health info
  - Serves health state via `/state` endpoint

- **Types**: `internal/types/resources.go`
  - `HealthCheckEntry`: Individual check result
  - `ServiceHealthInfo`: Service health state with history array

## Proposed Solution

### High-Level Architecture

Implement a **multi-tier health check persistence system** with time-bucketed aggregation, similar to Plex's 90-day historical view.

**Key Components:**

1. **Persistent Storage Layer** - Store health data to survive restarts
2. **Time-Bucketed Aggregation** - Compress data into time buckets for efficient long-term storage
3. **Multi-Resolution History** - Different bucket sizes for different time ranges
4. **Visual Historical View** - Frontend components to display long-term trends

### Time-Bucket Strategy

Instead of storing every individual health check indefinitely, aggregate checks into time buckets:

| Time Range | Bucket Size | Data Points | Total Duration |
|------------|-------------|-------------|----------------|
| Last 24 hours | 5 minutes | 288 | 1 day |
| Last 7 days | 1 hour | 168 | 7 days |
| Last 30 days | 6 hours | 120 | 30 days |
| Last 90 days | 1 day | 90 | 90 days |

**Benefits:**
- Efficient storage: ~666 data points per service vs 259,200 raw checks (90 days at 30s intervals)
- Fast queries: Pre-aggregated data ready to serve
- Flexible resolution: Detailed recent data, summarized historical data
- Scalable: Storage requirements don't grow unbounded

### Data Model

#### New Types (Go)

```go
// HealthBucket represents aggregated health data over a time period
type HealthBucket struct {
    StartTime    time.Time    `json:"start_time"`
    EndTime      time.Time    `json:"end_time"`
    SuccessRate  float64      `json:"success_rate"`  // Percentage of healthy checks
    AvgLatency   time.Duration `json:"avg_latency"`   // Average response time
    TotalChecks  int          `json:"total_checks"`  // Number of checks in bucket
    HealthyCount int          `json:"healthy_count"` // Number of successful checks
    Status       HealthStatus `json:"status"`        // Overall bucket status
}

// HistoricalHealthData contains time-bucketed health history
type HistoricalHealthData struct {
    Last24Hours []HealthBucket `json:"last_24_hours"` // 5-min buckets
    Last7Days   []HealthBucket `json:"last_7_days"`   // 1-hour buckets
    Last30Days  []HealthBucket `json:"last_30_days"`  // 6-hour buckets
    Last90Days  []HealthBucket `json:"last_90_days"`  // daily buckets
}

// Update ServiceHealthInfo to include historical data
type ServiceHealthInfo struct {
    ServiceName string             `json:"service_name"`
    Namespace   string             `json:"namespace"`
    LastCheck   time.Time          `json:"last_check"`
    Status      HealthStatus       `json:"status"`
    Uptime      float64            `json:"uptime"`
    History     []HealthCheckEntry `json:"history"`      // Recent raw checks
    URL         string             `json:"url"`
    Historical  *HistoricalHealthData `json:"historical,omitempty"` // NEW: Long-term data
}
```

#### New Types (TypeScript)

```typescript
interface HealthBucket {
  startTime: string
  endTime: string
  successRate: number      // 0-100
  avgLatency: number       // milliseconds
  totalChecks: number
  healthyCount: number
  status: HealthStatus
}

interface HistoricalHealthData {
  last24Hours: HealthBucket[]  // 288 5-min buckets
  last7Days: HealthBucket[]    // 168 1-hour buckets
  last30Days: HealthBucket[]   // 120 6-hour buckets
  last90Days: HealthBucket[]   // 90 daily buckets
}

interface ServiceHealthInfo {
  // ... existing fields ...
  historical?: HistoricalHealthData
}
```

## Implementation Plan

### Phase 1: Persistence Layer (Backend)

**Goal:** Prevent data loss on restart

1. **Create `internal/persistence/` package**
   - `storage.go`: Interface definition
     ```go
     type HealthStorage interface {
         SaveServiceHealth(key string, health *ServiceHealthInfo) error
         LoadServiceHealth(key string) (*ServiceHealthInfo, error)
         LoadAllServiceHealth() (map[string]*ServiceHealthInfo, error)
         DeleteServiceHealth(key string) error
     }
     ```

   - `json_storage.go`: JSON file-based implementation
     - Store health data in `/data/health-data.json`
     - Atomic writes with temp file + rename
     - Periodic background saves (every 5 minutes)
     - Load on startup

2. **Update HealthChecker**
   - Add `storage HealthStorage` field
   - On startup: `hc.storage.LoadAllServiceHealth()` to restore state
   - After each check: async write to storage
   - Graceful shutdown: flush pending writes

3. **Configuration**
   - CLI flag: `--health-storage-path` (default: `/data/health-data.json`)
   - Environment variable: `HEALTH_STORAGE_PATH`

### Phase 2: Time-Bucket Aggregation (Backend)

**Goal:** Compress raw checks into time buckets

1. **Create `internal/persistence/aggregator.go`**
   - `AggregateIntoBuckets(checks []HealthCheckEntry, bucketSize time.Duration) []HealthBucket`
   - Calculate success rate, avg latency, bucket status
   - Handle partial buckets (current time period)

2. **Update HealthChecker**
   - Keep raw checks for last 24 hours in memory
   - Every 5 minutes: aggregate older raw checks into buckets
   - Prune raw data after successful aggregation
   - Store bucketed data in `ServiceHealthInfo.Historical`

3. **Aggregation Logic**
   ```go
   // Pseudo-code
   func aggregateChecks(checks []HealthCheckEntry, bucketSize time.Duration) HealthBucket {
       bucket := HealthBucket{
           StartTime: bucketStart,
           EndTime: bucketStart.Add(bucketSize),
       }

       for _, check := range checks {
           bucket.TotalChecks++
           if check.Status == HealthStatusHealthy {
               bucket.HealthyCount++
           }
           bucket.AvgLatency += check.Latency
       }

       bucket.SuccessRate = float64(bucket.HealthyCount) / float64(bucket.TotalChecks) * 100
       bucket.AvgLatency = bucket.AvgLatency / time.Duration(bucket.TotalChecks)

       // Determine bucket status based on success rate
       if bucket.SuccessRate >= 95.0 {
           bucket.Status = HealthStatusHealthy
       } else if bucket.SuccessRate >= 70.0 {
           bucket.Status = HealthStatusDegraded // NEW status
       } else {
           bucket.Status = HealthStatusUnhealthy
       }

       return bucket
   }
   ```

4. **Bucket Maintenance**
   - Run aggregation on schedule (every 5 minutes)
   - Roll up 5-min buckets into 1-hour buckets after 24 hours
   - Roll up 1-hour buckets into 6-hour buckets after 7 days
   - Roll up 6-hour buckets into daily buckets after 30 days
   - Delete daily buckets older than 90 days

### Phase 3: Frontend Historical View

**Goal:** Display Plex-style long-term health visualization

1. **Create `HealthHistoryBar.vue` component**
   - Props: `buckets: HealthBucket[]`, `timeRange: string`
   - Renders horizontal bar with colored segments
   - Each segment represents one bucket
   - Color coding:
     - Green: Success rate ≥95%
     - Yellow: Success rate 70-95%
     - Red: Success rate <70%
     - Grey: No data
   - Tooltip on hover: time range, success rate, avg latency, total checks
   - Responsive: adjusts segment width based on container size

2. **Create `UptimeStats.vue` component**
   - Props: `historical: HistoricalHealthData`
   - Displays uptime percentages:
     - Last 24 hours
     - Last 7 days
     - Last 30 days
     - Last 90 days
   - Calculate from bucket data: `sum(healthyCount) / sum(totalChecks) * 100`

3. **Update `ServiceCard.vue`**
   - Add toggle: "Recent Checks" vs "Historical View"
   - Recent Checks: Current 10 ovals (last ~5 minutes of raw checks)
   - Historical View: Time-bucketed bar chart
   - Display `UptimeStats` below historical view
   - Default to "Recent Checks" view

4. **Update `ServiceHealthIndicator.vue`**
   - Pass `service.serviceHealth.historical` to child components
   - Render `HealthHistoryBar` when in historical mode

### Phase 4: Polish & Configuration

1. **Add CLI Flags**
   - `--health-retention-days` (default: 90)
   - `--health-storage-path` (default: `/data/health-data.json`)
   - `--health-aggregation-interval` (default: 5m)
   - `--health-check-interval` (default: 30s)

2. **Add Kubernetes Deployment**
   - Update deployment manifest with persistent volume claim
   - Mount volume at `/data` for health storage
   - Configure resource limits based on expected data size

3. **Documentation**
   - Update README with health persistence features
   - Document configuration options
   - Add example deployment with persistent volume

## Storage Format

### JSON File Structure

```json
{
  "version": "1",
  "lastUpdated": "2025-10-06T10:30:00Z",
  "services": {
    "default/my-service": {
      "serviceName": "my-service",
      "namespace": "default",
      "lastCheck": "2025-10-06T10:30:00Z",
      "status": "healthy",
      "uptime": 99.5,
      "url": "http://my-service.default.svc.cluster.local:8080/health",
      "history": [
        {
          "timestamp": "2025-10-06T10:30:00Z",
          "status": "healthy",
          "latency": 15000000,
          "url": "http://my-service.default.svc.cluster.local:8080/health",
          "method": "GET",
          "responseCode": 200
        }
      ],
      "historical": {
        "last24Hours": [
          {
            "startTime": "2025-10-06T10:00:00Z",
            "endTime": "2025-10-06T10:05:00Z",
            "successRate": 100,
            "avgLatency": 15000000,
            "totalChecks": 10,
            "healthyCount": 10,
            "status": "healthy"
          }
        ],
        "last7Days": [],
        "last30Days": [],
        "last90Days": []
      }
    }
  }
}
```

## Open Questions

The following questions have been converted into backlog tasks:

1. **Storage Location** (task-1)
   - Should health data be in a persistent volume mount or ephemeral?
   - Recommendation: Persistent volume for production, with configuration option
   - Trade-offs: Persistence vs complexity, cost, backup requirements

2. **Per-Service Retention** (task-2)
   - Should we allow per-service retention configuration via annotations?
   - Example: `constellation.kyledev.co/health-retention-days: "30"`
   - Trade-offs: Flexibility vs storage management complexity

3. **Export Functionality** (task-3)
   - Should we support exporting historical data as CSV/JSON for analysis?
   - Use cases: SLA reporting, external monitoring integration, auditing
   - Implementation: API endpoint vs CLI tool vs both

4. **Downsampling Strategy** (task-4)
   - When upgrading from 5-min to 1-hour buckets, how to aggregate overlapping data?
   - Should we keep raw data temporarily for recomputation?
   - How to handle gaps in data (missed checks, service downtime)?
   - What aggregation functions: average, min, max, percentiles?

## Success Metrics

After implementation:
- ✅ Health data survives pod restarts
- ✅ Can view service uptime over 24h, 7d, 30d, 90d
- ✅ Storage requirements scale linearly with service count, not time
- ✅ Frontend displays Plex-style historical health visualization
- ✅ Uptime percentages calculated over multiple time periods
- ✅ No performance degradation with historical data enabled

## References

- [Plex Status Page](https://status.plex.tv/) - Inspiration for visual design
- Current implementation: `internal/controller/health_checker.go`
- Current types: `internal/types/resources.go`
