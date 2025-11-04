import type { HealthStatus } from '../types'

const STATUS_COLORS: Record<HealthStatus, string> = {
  healthy: '#4ade80',
  unhealthy: '#f87171',
  unknown: '#fbbf24'
}

const STATUS_TEXT_COLORS: Record<HealthStatus, string> = {
  healthy: 'text-green-600',
  unhealthy: 'text-red-600',
  unknown: 'text-yellow-600'
}

export function getStatusColor(status: HealthStatus): string {
  return STATUS_COLORS[status]
}

export function getStatusTextColor(status: HealthStatus): string {
  return STATUS_TEXT_COLORS[status]
}

export const EMPTY_STATUS_COLOR = '#d1d5db'
