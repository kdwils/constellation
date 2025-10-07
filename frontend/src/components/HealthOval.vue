<script setup lang="ts">
import { ref } from 'vue'
import Tooltip from './Tooltip.vue'
import type { HealthCheckEntry, HealthStatus } from '../types'

interface Props {
  entry?: HealthCheckEntry
  isEmpty?: boolean
}

const props = defineProps<Props>()
const showTooltip = ref(false)

const getHealthColor = (status?: HealthStatus): string => {
  if (!status) return '#d1d5db'
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

const backgroundColor = props.isEmpty ? '#d1d5db' : getHealthColor(props.entry?.status)
const isInteractive = !props.isEmpty && props.entry
</script>

<template>
  <div
    class="relative"
    :class="{ 'cursor-pointer': isInteractive }"
    @mouseenter="isInteractive ? showTooltip = true : null"
    @mouseleave="isInteractive ? showTooltip = false : null"
  >
    <div
      class="w-3 h-10 rounded-full border-2 border-gray-900 transition-colors duration-300"
      :style="{ backgroundColor }"
    />

    <Tooltip v-if="isInteractive && entry" :show="showTooltip">
      <div class="flex flex-col gap-2">
        <div class="font-semibold text-gray-900 border-b border-gray-300 pb-1">
          Service Connectivity
        </div>

        <div class="flex flex-col gap-1 text-xs">
          <div class="flex justify-between gap-4">
            <span class="text-gray-600">Status:</span>
            <span class="font-medium" :class="{
              'text-green-600': entry.status === 'healthy',
              'text-red-600': entry.status === 'unhealthy',
              'text-yellow-600': entry.status === 'unknown'
            }">
              {{ entry.status }}
            </span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-600">Timestamp:</span>
            <span class="font-mono">{{ formatTimestamp(entry.timestamp) }}</span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-600">Method:</span>
            <span class="font-mono">{{ entry.method }}</span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-600">Latency:</span>
            <span class="font-mono">{{ formatLatency(entry.latency) }}</span>
          </div>

          <div v-if="entry.response_code" class="flex justify-between gap-4">
            <span class="text-gray-600">Status Code:</span>
            <span class="font-mono">{{ entry.response_code }}</span>
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-gray-600">Service Endpoint:</span>
            <span class="font-mono text-xs break-all">{{ entry.url }}</span>
          </div>

          <div v-if="entry.error" class="flex flex-col gap-1 mt-1 pt-2 border-t border-gray-300">
            <span class="text-gray-600">Error:</span>
            <span class="font-mono text-xs text-red-600 break-words">{{ entry.error }}</span>
          </div>
        </div>
      </div>
    </Tooltip>
  </div>
</template>
