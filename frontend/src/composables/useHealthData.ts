import type { HierarchyNode, ServiceCardData, ServiceHealthData } from '../types'

export function transformToServiceCards(hierarchyNodes: HierarchyNode[]): ServiceCardData[] {
  const serviceCards: ServiceCardData[] = []

  for (const node of hierarchyNodes) {
    if (node.kind === 'Service' && node.health_info) {
      const serviceHealth = extractServiceHealth(node)

      serviceCards.push({
        name: node.name,
        namespace: node.namespace || 'default',
        status: node.health_info.status,
        lastUpdate: node.health_info.last_check,
        latency: calculateAverageLatency(node.health_info.history),
        url: node.health_info.url,
        serviceHealth
      })
    }

    if (node.relatives && node.relatives.length > 0) {
      serviceCards.push(...transformToServiceCards(node.relatives))
    }
  }

  return serviceCards
}

function extractServiceHealth(serviceNode: HierarchyNode): ServiceHealthData {
  const serviceHealthInfo = serviceNode.health_info

  if (!serviceHealthInfo) {
    return {
      status: 'unknown',
      healthCheckHistory: []
    }
  }

  return {
    status: serviceHealthInfo.status,
    healthCheckHistory: serviceHealthInfo.history || []
  }
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
