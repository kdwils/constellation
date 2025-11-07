package server

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/gorilla/websocket"
	"github.com/kdwils/constellation/internal/types"
)

const (
	writeWait      = 10 * time.Second
	pongWait       = 60 * time.Second
	pingPeriod     = (pongWait * 9) / 10
	maxMessageSize = 512
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true
	},
	HandshakeTimeout: 5 * time.Second,
}

type HealthDataProvider interface {
	GetAllHealthData() []*types.ServiceHealthInfo
	Subscribe() chan []*types.ServiceHealthInfo
	Unsubscribe(chan []*types.ServiceHealthInfo)
}

type Server struct {
	healthProvider HealthDataProvider
	staticDir      string
	port           int
}

func NewServer(healthProvider HealthDataProvider, staticDir string, port int) *Server {
	return &Server{
		healthProvider: healthProvider,
		staticDir:      staticDir,
		port:           port,
	}
}

func (s *Server) Serve(ctx context.Context) error {
	mux := http.NewServeMux()

	mux.HandleFunc("/state", s.handleState)
	mux.HandleFunc("/ws", s.handleWebSocket)
	mux.HandleFunc("/healthz", s.handleHealth)

	if s.staticDir != "" {
		fileServer := http.FileServer(http.Dir(s.staticDir))
		mux.Handle("/", s.staticFileHandler(fileServer))
	}

	httpServer := &http.Server{
		Addr:    fmt.Sprintf(":%d", s.port),
		Handler: mux,
	}

	go func() {
		<-ctx.Done()
		httpServer.Shutdown(context.Background())
	}()

	if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		return fmt.Errorf("HTTP server failed: %v", err)
	}
	return nil
}

func (s *Server) handleState(w http.ResponseWriter, r *http.Request) {
	healthData := s.healthProvider.GetAllHealthData()

	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(healthData); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
}

func (s *Server) handleWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		http.Error(w, fmt.Sprintf("WebSocket upgrade error: %v", err), http.StatusBadRequest)
		return
	}
	defer func() {
		fmt.Printf("WebSocket connection closed\n")
		conn.Close()
	}()

	fmt.Printf("WebSocket connection established\n")

	conn.SetReadLimit(maxMessageSize)
	conn.SetReadDeadline(time.Now().Add(pongWait))
	conn.SetPongHandler(func(string) error {
		conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	healthChan := s.healthProvider.Subscribe()
	defer s.healthProvider.Unsubscribe(healthChan)

	if err := s.writeMessage(conn, s.healthProvider.GetAllHealthData()); err != nil {
		fmt.Printf("WebSocket initial write error: %v\n", err)
		return
	}

	go func() {
		for {
			_, _, err := conn.ReadMessage()
			if err != nil {
				fmt.Printf("WebSocket read error: %v\n", err)
				return
			}
		}
	}()

	pingTicker := time.NewTicker(pingPeriod)
	defer pingTicker.Stop()

	for {
		select {
		case data := <-healthChan:
			if err := s.writeMessage(conn, data); err != nil {
				fmt.Printf("WebSocket write error: %v\n", err)
				return
			}
		case <-pingTicker.C:
			conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				fmt.Printf("WebSocket ping error: %v\n", err)
				return
			}
		case <-r.Context().Done():
			return
		}
	}
}

func (s *Server) writeMessage(conn *websocket.Conn, data any) error {
	conn.SetWriteDeadline(time.Now().Add(writeWait))
	return conn.WriteJSON(data)
}

func (s *Server) handleHealth(w http.ResponseWriter, r *http.Request) {
	healthData := s.healthProvider.GetAllHealthData()
	ready := len(healthData) > 0

	if !ready {
		w.WriteHeader(http.StatusServiceUnavailable)
		json.NewEncoder(w).Encode(map[string]string{
			"message": "waiting for kubernetes resources",
		})
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"message": "ready",
	})
}

func (s *Server) staticFileHandler(fileServer http.Handler) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		fileServer.ServeHTTP(w, r)
	}
}
