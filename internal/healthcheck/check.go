package healthcheck

import (
	"context"
	"net/http"
	"sort"
	"strings"
	"sync"
	"time"

	"sigs.k8s.io/controller-runtime/pkg/log"

	"github.com/kdwils/constellation/internal/cache"
	"github.com/kdwils/constellation/internal/types"
)

// HTTPClient interface for dependency injection during tests
//
//go:generate mockgen -destination=mocks/mock_http_client.go -package=mocks github.com/kdwils/constellation/internal/healthcheck HTTPClient
type HTTPClient interface {
	Do(*http.Request) (*http.Response, error)
}

// HealthTarget represents a service or endpoint being monitored
type HealthTarget struct {
	Name   string
	Checks []CheckConfig
	cancel context.CancelFunc
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
	registerCh    chan HealthTarget
	unregisterCh  chan string
	checkCh       chan CheckConfig
	httpClient    HTTPClient
}

// NewHealthChecker creates a new health checker
func NewHealthChecker(opts ...HealthCheckerOpt) *HealthChecker {
	hc := &HealthChecker{
		healthData:    cache.New[*types.ServiceHealthInfo](),
		healthTargets: cache.New[HealthTarget](),
		subscribers:   make(map[chan []*types.ServiceHealthInfo]bool),
		registerCh:    make(chan HealthTarget, 100),
		unregisterCh:  make(chan string, 100),
		checkCh:       make(chan CheckConfig, 100),
		httpClient:    http.DefaultClient,
	}

	for _, opt := range opts {
		opt(hc)
	}

	return hc
}

type HealthCheckerOpt func(*HealthChecker)

func WithHTTPClient(client HTTPClient) HealthCheckerOpt {
	return func(hc *HealthChecker) {
		hc.httpClient = client
	}
}

// Start begins the health checking routine
func (hc *HealthChecker) Start(ctx context.Context) error {
	logger := log.FromContext(ctx)
	logger.Info("Starting health checker")

	go hc.listenForRegistrations(ctx)
	go hc.listenForUnregistrations(ctx)

	for {
		select {
		case cfg := <-hc.checkCh:
			go hc.executeCheck(ctx, cfg)
		case <-ctx.Done():
			return nil
		}
	}
}

func (hc *HealthChecker) listenForRegistrations(parentCtx context.Context) {
	for {
		select {
		case target := <-hc.registerCh:
			existing, exists := hc.healthTargets.Get(target.Name)

			if exists {
				target.cancel = existing.cancel
				hc.healthTargets.Set(target.Name, target)
				hc.notifySubscribers()
				continue
			}

			ctx, cancel := context.WithCancel(parentCtx)
			target.cancel = cancel
			hc.healthTargets.Set(target.Name, target)

			for _, check := range target.Checks {
				go hc.runCheckTicker(ctx, check)
			}

			hc.notifySubscribers()

		case <-parentCtx.Done():
			return
		}
	}
}

func (hc *HealthChecker) listenForUnregistrations(ctx context.Context) {
	for {
		select {
		case name := <-hc.unregisterCh:
			target, exists := hc.healthTargets.Get(name)
			if !exists {
				continue
			}

			if target.cancel != nil {
				target.cancel()
			}

			hc.healthTargets.Delete(name)
			hc.healthData.Delete(name)
			hc.notifySubscribers()

		case <-ctx.Done():
			return
		}
	}
}

func (hc *HealthChecker) runCheckTicker(ctx context.Context, cfg CheckConfig) {
	ticker := time.NewTicker(cfg.Interval)
	defer ticker.Stop()

	select {
	case hc.checkCh <- cfg:
	case <-ctx.Done():
		return
	}

	for {
		select {
		case <-ticker.C:
			select {
			case hc.checkCh <- cfg:
			case <-ctx.Done():
				return
			}
		case <-ctx.Done():
			return
		}
	}
}

func (hc *HealthChecker) executeCheck(ctx context.Context, cfg CheckConfig) {
	log := log.FromContext(ctx)
	log.Info("firing check", "cfg", cfg)
	startTime := time.Now()

	reqCtx, cancel := context.WithTimeout(ctx, cfg.Timeout)
	defer cancel()

	req, err := http.NewRequestWithContext(reqCtx, "GET", cfg.URL, nil)
	if err != nil {
		hc.recordCheckResult(cfg, startTime, 0, err)
		return
	}

	resp, err := hc.httpClient.Do(req)
	if err != nil {
		hc.recordCheckResult(cfg, startTime, 0, err)
		return
	}
	defer resp.Body.Close()

	hc.recordCheckResult(cfg, startTime, resp.StatusCode, nil)
}

func (hc *HealthChecker) recordCheckResult(cfg CheckConfig, startTime time.Time, statusCode int, err error) {
	namespace, service := parseTargetName(cfg.Name)
	latency := time.Since(startTime)

	entry := types.HealthCheckEntry{
		Timestamp:    startTime,
		Status:       determineStatus(statusCode, err),
		Latency:      latency,
		Error:        formatError(err),
		URL:          cfg.URL,
		Method:       "GET",
		ResponseCode: statusCode,
	}

	hc.mu.Lock()
	key := namespace + "/" + service
	info, exists := hc.healthData.Get(key)
	if !exists {
		info = &types.ServiceHealthInfo{
			ServiceName: service,
			Namespace:   namespace,
			History:     []types.HealthCheckEntry{},
		}
	}

	info.History = append(info.History, entry)
	if len(info.History) > 100 {
		info.History = info.History[len(info.History)-100:]
	}

	info.LastCheck = startTime
	info.Status = entry.Status
	info.URL = cfg.URL
	info.Uptime = calculateUptime(info.History)

	hc.healthData.Set(key, info)
	hc.mu.Unlock()

	hc.notifySubscribers()
}

func parseTargetName(name string) (string, string) {
	parts := strings.Split(name, "/")
	if len(parts) == 2 {
		return parts[0], parts[1]
	}
	return "default", name
}

func determineStatus(statusCode int, err error) types.HealthStatus {
	if err != nil {
		return "unhealthy"
	}
	if statusCode >= 200 && statusCode < 300 {
		return "healthy"
	}
	return "unhealthy"
}

func formatError(err error) string {
	if err == nil {
		return ""
	}
	return err.Error()
}

func calculateUptime(history []types.HealthCheckEntry) float64 {
	if len(history) == 0 {
		return 0.0
	}

	healthy := 0
	for _, entry := range history {
		if entry.Status == "healthy" {
			healthy++
		}
	}

	return (float64(healthy) / float64(len(history))) * 100.0
}

// RegisterHealthTarget registers or updates a health target
func (hc *HealthChecker) RegisterHealthTarget(name string, checks []CheckConfig) {
	target := HealthTarget{
		Name:   name,
		Checks: checks,
	}
	hc.registerCh <- target
}

// UnregisterHealthTarget removes a health target
func (hc *HealthChecker) UnregisterHealthTarget(name string) {
	hc.unregisterCh <- name
}

// GetAllHealthData returns all current health data
func (hc *HealthChecker) GetAllHealthData() []*types.ServiceHealthInfo {
	keys := hc.healthData.Keys()
	sort.Strings(keys)

	data := make([]*types.ServiceHealthInfo, 0, len(keys))
	for _, key := range keys {
		if info, exists := hc.healthData.Get(key); exists {
			data = append(data, info)
		}
	}

	return data
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
