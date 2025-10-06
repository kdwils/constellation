import type { HierarchyNode, ServiceCardData, PodHealthData } from '../types'

export function transformToServiceCards(hierarchyNodes: HierarchyNode[]): ServiceCardData[] {
  const serviceCards: ServiceCardData[] = []

  for (const node of hierarchyNodes) {
    if (node.kind === 'Service' && node.health_info) {
      const podHealthStates = extractPodHealthStates(node)

      serviceCards.push({
        name: node.name,
        namespace: node.namespace || 'default',
        status: node.health_info.status,
        lastUpdate: node.health_info.last_check,
        latency: calculateAverageLatency(node.health_info.history),
        url: node.health_info.url,
        podHealthStates
      })
    }

    if (node.relatives && node.relatives.length > 0) {
      serviceCards.push(...transformToServiceCards(node.relatives))
    }
  }

  return serviceCards
}

function extractPodHealthStates(serviceNode: HierarchyNode): PodHealthData[] {
  const podStates: PodHealthData[] = []

  if (!serviceNode.relatives) {
    return podStates
  }

  const serviceHealthInfo = serviceNode.health_info
  if (!serviceHealthInfo) {
    return podStates
  }

  const latestEntry = serviceHealthInfo.history && serviceHealthInfo.history.length > 0
    ? serviceHealthInfo.history[serviceHealthInfo.history.length - 1]
    : undefined

  for (const relative of serviceNode.relatives) {
    if (relative.kind === 'Pod') {
      podStates.push({
        status: serviceHealthInfo.status,
        healthCheckEntry: latestEntry
      })
    }
  }

  return podStates
}

function calculateAverageLatency(history: Array<{ latency: number }>): number {
  if (!history || history.length === 0) {
    return 0
  }

  const recentHistory = history.slice(-10)
  const sum = recentHistory.reduce((acc, entry) => acc + entry.latency, 0)
  return Math.round(sum / recentHistory.length)
}
