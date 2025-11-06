import type { ServiceHealthInfo, ServiceCardData } from '../types'

export function transformToServiceCards(healthData: ServiceHealthInfo[]): ServiceCardData[] {
  return healthData.map(health => ({
    name: health.service_name,
    namespace: health.namespace,
    status: health.status,
    lastUpdate: health.last_check,
    latency: calculateAverageLatency(health.history),
    url: health.url,
    serviceHealth: {
      status: health.status,
      healthCheckHistory: health.history || []
    }
  }))
}

function calculateAverageLatency(history: Array<{ latency: number }>): number {
  if (!history || history.length === 0) {
    return 0
  }

  const recentHistory = history.slice(-10)
  const sum = recentHistory.reduce((acc, entry) => acc + entry.latency, 0)
  const averageNanoseconds = sum / recentHistory.length
  return Math.round(averageNanoseconds / 1_000_000)
}
