<script setup lang="ts">
import PodHealthIndicator from './PodHealthIndicator.vue'
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
    <div class="flex justify-between items-center gap-3">
      <div class="text-base font-semibold text-gray-900 flex-1 overflow-hidden text-ellipsis whitespace-nowrap">
        &lt;{{ service.name }}&gt;
      </div>
      <StatusBadge :status="service.status" />
    </div>

    <div class="flex gap-1.5 flex-wrap min-h-8">
      <PodHealthIndicator
        v-for="(podHealth, index) in service.podHealthStates"
        :key="index"
        :pod-health="podHealth"
      />
    </div>

    <div class="flex flex-col gap-1.5 text-sm text-gray-900">
      <div class="flex justify-between gap-2">
        <span>Last Update: &lt;{{ formatTimestamp(service.lastUpdate) }}&gt;</span>
        <span>Latency: &lt;{{ formatLatency(service.latency) }}&gt;</span>
      </div>
      <div class="flex justify-between gap-2">
        <span>&lt;{{ service.namespace }}&gt;</span>
      </div>
      <div class="flex justify-between gap-2">
        <span class="break-all text-xs">&lt;{{ service.url }}&gt;</span>
      </div>
    </div>
  </div>
</template>
