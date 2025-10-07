<script setup lang="ts">
import ServiceHealthIndicator from './ServiceHealthIndicator.vue'
import StatusBadge from './StatusBadge.vue'
import type { ServiceCardData } from '../types'

interface Props {
  service: ServiceCardData
}

defineProps<Props>()

const formatTimestamp = (timestamp: string): string => {
  const date = new Date(timestamp)
  return date.toLocaleTimeString()
}

const formatLatency = (latency: number): string => {
  return `${latency}ms`
}
</script>

<template>
  <div class="bg-gray-100 border-[3px] border-gray-900 rounded-2xl p-5 flex flex-col gap-4 min-w-[280px] transition-transform duration-200 hover:scale-[1.02]">
    <div class="flex justify-between items-start gap-3">
      <div class="flex flex-col gap-1.5 min-w-0 flex-1">
        <div class="text-base font-semibold text-gray-900 overflow-hidden text-ellipsis whitespace-nowrap" :title="service.name">
          {{ service.name }}
        </div>
        <div class="text-sm text-gray-900">
          {{ service.namespace }}
        </div>
      </div>
      <div class="flex-shrink-0">
        <StatusBadge :status="service.status" />
      </div>
    </div>

    <div class="flex gap-1.5 min-h-8">
      <ServiceHealthIndicator :service-health="service.serviceHealth" />
    </div>

    <div class="flex flex-col gap-1.5 text-sm text-gray-900">
      <div class="flex justify-between gap-2">
        <span>Last Update: {{ formatTimestamp(service.lastUpdate) }}</span>
        <span>Latency: {{ formatLatency(service.latency) }}</span>
      </div>
      <div class="flex justify-between gap-2">
        <span class="break-all text-xs">{{ service.url }}</span>
      </div>
    </div>
  </div>
</template>
