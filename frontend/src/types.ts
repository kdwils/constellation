export type HealthStatus = 'healthy' | 'unhealthy' | 'unknown'

export interface HealthCheckEntry {
  timestamp: string
  status: HealthStatus
  latency: number
  error?: string
  url: string
  method: string
  response_code?: number
}

export interface ServiceHealthInfo {
  service_name: string
  namespace: string
  last_check: string
  status: HealthStatus
  uptime: number
  history: HealthCheckEntry[]
  url: string
}

export interface HierarchyNode {
  kind: string
  name: string
  namespace?: string
  relatives?: HierarchyNode[]
  hostnames?: string[]
  health_info?: ServiceHealthInfo
}

export interface ServiceHealthData {
  status: HealthStatus
  healthCheckHistory: HealthCheckEntry[]
}

export interface ServiceCardData {
  name: string
  namespace: string
  status: HealthStatus
  lastUpdate: string
  latency: number
  url: string
  serviceHealth: ServiceHealthData
}
