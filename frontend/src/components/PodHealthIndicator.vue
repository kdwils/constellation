<script setup lang="ts">
import { ref } from 'vue'
import Tooltip from './Tooltip.vue'
import type { PodHealthData, HealthStatus } from '../types'

interface Props {
  podHealth: PodHealthData
}

defineProps<Props>()
const showTooltip = ref(false)

const getHealthColor = (status: HealthStatus): string => {
  if (status === 'healthy') return '#4ade80'
  if (status === 'unhealthy') return '#f87171'
  return '#fbbf24'
}

const formatTimestamp = (timestamp: string): string => {
  const date = new Date(timestamp)
  return date.toLocaleString()
}

const formatLatency = (latency: number): string => {
  return `${latency}ms`
}
</script>

<template>
  <div
    class="relative"
    @mouseenter="showTooltip = true"
    @mouseleave="showTooltip = false"
  >
    <div
      class="w-5 h-8 rounded-full border-2 border-gray-900 transition-colors duration-300 cursor-pointer"
      :style="{ backgroundColor: getHealthColor(podHealth.status) }"
    />

    <Tooltip :show="showTooltip">
      <div v-if="podHealth.healthCheckEntry" class="flex flex-col gap-2">
        <div class="font-semibold text-gray-900 border-b border-gray-300 pb-1">
          Service Connectivity
        </div>

        <div class="flex flex-col gap-1 text-xs">
          <div class="flex justify-between gap-4">
            <span class="text-gray-600">Status:</span>
            <span class="font-medium" :class="{
              'text-green-600': podHealth.status === 'healthy',
              'text-red-600': podHealth.status === 'unhealthy',
              'text-yellow-600': podHealth.status === 'unknown'
            }">
              {{ podHealth.status }}
            </span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-600">Timestamp:</span>
            <span class="font-mono">{{ formatTimestamp(podHealth.healthCheckEntry.timestamp) }}</span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-600">Method:</span>
            <span class="font-mono">{{ podHealth.healthCheckEntry.method }}</span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-600">Latency:</span>
            <span class="font-mono">{{ formatLatency(podHealth.healthCheckEntry.latency) }}</span>
          </div>

          <div v-if="podHealth.healthCheckEntry.response_code" class="flex justify-between gap-4">
            <span class="text-gray-600">Status Code:</span>
            <span class="font-mono">{{ podHealth.healthCheckEntry.response_code }}</span>
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-gray-600">Service Endpoint:</span>
            <span class="font-mono text-xs break-all">{{ podHealth.healthCheckEntry.url }}</span>
          </div>

          <div v-if="podHealth.healthCheckEntry.error" class="flex flex-col gap-1 mt-1 pt-2 border-t border-gray-300">
            <span class="text-gray-600">Error:</span>
            <span class="font-mono text-xs text-red-600 break-words">{{ podHealth.healthCheckEntry.error }}</span>
          </div>
        </div>
      </div>
    </Tooltip>
  </div>
</template>
