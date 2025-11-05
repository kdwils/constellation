package healthcheck

import (
	"context"
	"net/http"
	"sync"
	"time"

	"sigs.k8s.io/controller-runtime/pkg/log"

	"github.com/kdwils/constellation/internal/cache"
	"github.com/kdwils/constellation/internal/types"
)

// HTTPClient interface for dependency injection during tests
type HTTPClient interface {
	Do(*http.Request) (*http.Response, error)
}

// HealthTarget represents a service or endpoint being monitored
type HealthTarget struct {
	Name   string
	Checks []CheckConfig
}

// CheckConfig represents a single health check endpoint
type CheckConfig struct {
	Name     string
	URL      string
	Interval time.Duration
	Timeout  time.Duration
	Protocol string // "http", "tcp", "grpc"
}

// HealthChecker manages health checks for in-cluster services based on pod probes
type HealthChecker struct {
	mu            sync.RWMutex
	healthData    *cache.Cache[*types.ServiceHealthInfo]
	healthTargets *cache.Cache[HealthTarget]
	subscribers   map[chan []*types.ServiceHealthInfo]bool
	subMu         sync.RWMutex
}

// NewHealthChecker creates a new health checker
func NewHealthChecker() *HealthChecker {
	return &HealthChecker{
		healthData:    cache.New[*types.ServiceHealthInfo](),
		healthTargets: cache.New[HealthTarget](),
		subscribers:   make(map[chan []*types.ServiceHealthInfo]bool),
	}
}

// Start begins the health checking routine
func (hc *HealthChecker) Start(ctx context.Context) error {
	logger := log.FromContext(ctx)
	logger.Info("Starting health checker")
		
	// TODO: wait on healthtargets to be sent via channel and perform check
	// TODO: add case for ctx.Done to return

	return nil
}

// RegisterHealthTarget registers or updates a health target
func (hc *HealthChecker) RegisterHealthTarget(name string, checks []CheckConfig) {
	target := HealthTarget{
		Name:   name,
		Checks: checks,
	}
	hc.healthTargets.Set(name, target)
	hc.notifySubscribers()
}

// UnregisterHealthTarget removes a health target
func (hc *HealthChecker) UnregisterHealthTarget(name string) {
	hc.healthData.Delete(name)
	hc.healthTargets.Delete(name)
	hc.notifySubscribers()
}

// GetAllHealthData returns all current health data
func (hc *HealthChecker) GetAllHealthData() []*types.ServiceHealthInfo {
	return hc.healthData.List()
}

// Subscribe creates a new subscription channel for health data updates
func (hc *HealthChecker) Subscribe() chan []*types.ServiceHealthInfo {
	hc.subMu.Lock()
	defer hc.subMu.Unlock()

	ch := make(chan []*types.ServiceHealthInfo, 1)
	hc.subscribers[ch] = true
	return ch
}

// Unsubscribe removes a subscription channel
func (hc *HealthChecker) Unsubscribe(ch chan []*types.ServiceHealthInfo) {
	hc.subMu.Lock()
	defer hc.subMu.Unlock()

	delete(hc.subscribers, ch)
	close(ch)
}

// notifySubscribers sends current health data to all subscribers
func (hc *HealthChecker) notifySubscribers() {
	hc.subMu.RLock()
	defer hc.subMu.RUnlock()

	data := hc.GetAllHealthData()

	for ch := range hc.subscribers {
		select {
		case ch <- data:
		default:
		}
	}
}
