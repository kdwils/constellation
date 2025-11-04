<script setup lang="ts">
import { ref, computed } from 'vue'
import Tooltip from './Tooltip.vue'
import { getStatusColor, getStatusTextColor, EMPTY_STATUS_COLOR } from '../utils/statusColors'
import type { HealthCheckEntry } from '../types'

interface Props {
  entry?: HealthCheckEntry
  isEmpty?: boolean
}

const props = defineProps<Props>()
const showTooltip = ref(false)
const ovalRef = ref<HTMLElement | null>(null)

const backgroundColor = computed(() => {
  if (props.isEmpty) {
    return EMPTY_STATUS_COLOR
  }

  if (!props.entry?.status) {
    return EMPTY_STATUS_COLOR
  }

  return getStatusColor(props.entry.status)
})

const isInteractive = computed(() => {
  return !props.isEmpty && !!props.entry
})

const statusTextColor = computed(() => {
  if (!props.entry?.status) {
    return ''
  }

  return getStatusTextColor(props.entry.status)
})

const formatTimestamp = (timestamp: string): string => {
  const date = new Date(timestamp)
  return date.toLocaleString()
}

const formatLatency = (latency: number): string => {
  const ms = Math.round(latency / 1_000_000)
  return `${ms}ms`
}
</script>

<template>
  <div
    class="relative"
    :class="{ 'cursor-pointer': isInteractive }"
    @mouseenter="showTooltip = isInteractive"
    @mouseleave="showTooltip = false"
  >
    <div
      ref="ovalRef"
      class="w-3 h-10 rounded-full border-2 border-gray-700 transition-colors duration-300"
      :style="{ backgroundColor }"
    />

    <Tooltip v-if="isInteractive && entry" :show="showTooltip" :reference-el="ovalRef">
      <div class="flex flex-col gap-2">
        <div class="font-semibold text-gray-100 border-b border-gray-600 pb-1">
          Service Connectivity
        </div>

        <div class="flex flex-col gap-1 text-xs">
          <div class="flex justify-between gap-4">
            <span class="text-gray-400">Status:</span>
            <span class="font-medium" :class="statusTextColor">
              {{ entry.status }}
            </span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-400">Timestamp:</span>
            <span class="font-mono text-gray-200">{{ formatTimestamp(entry.timestamp) }}</span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-400">Method:</span>
            <span class="font-mono text-gray-200">{{ entry.method }}</span>
          </div>

          <div class="flex justify-between gap-4">
            <span class="text-gray-400">Latency:</span>
            <span class="font-mono text-gray-200">{{ formatLatency(entry.latency) }}</span>
          </div>

          <div v-if="entry.response_code" class="flex justify-between gap-4">
            <span class="text-gray-400">Status Code:</span>
            <span class="font-mono text-gray-200">{{ entry.response_code }}</span>
          </div>

          <div class="flex flex-col gap-1">
            <span class="text-gray-400">Service Endpoint:</span>
            <span class="font-mono text-xs break-all text-gray-200">{{ entry.url }}</span>
          </div>

          <div v-if="entry.error" class="flex flex-col gap-1 mt-1 pt-2 border-t border-gray-600">
            <span class="text-gray-400">Error:</span>
            <span class="font-mono text-xs text-red-400 break-words">{{ entry.error }}</span>
          </div>
        </div>
      </div>
    </Tooltip>
  </div>
</template>
