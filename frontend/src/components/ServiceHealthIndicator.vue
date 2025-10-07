<script setup lang="ts">
import { computed } from 'vue'
import HealthOval from './HealthOval.vue'
import type { ServiceHealthData, HealthCheckEntry } from '../types'

interface Props {
  serviceHealth: ServiceHealthData
  maxOvals?: number
}

const props = withDefaults(defineProps<Props>(), {
  maxOvals: 15
})

const ovalSlots = computed<Array<HealthCheckEntry | undefined>>(() => {
  const history = props.serviceHealth.healthCheckHistory
  const slots: Array<HealthCheckEntry | undefined> = []

  for (let i = 0; i < props.maxOvals; i++) {
    const historyIndex = history.length - props.maxOvals + i
    slots.push(historyIndex >= 0 ? history[historyIndex] : undefined)
  }

  return slots
})
</script>

<template>
  <div class="flex gap-1">
    <HealthOval
      v-for="(entry, index) in ovalSlots"
      :key="entry ? entry.timestamp : `empty-${index}`"
      :entry="entry"
      :is-empty="!entry"
    />
  </div>
</template>
