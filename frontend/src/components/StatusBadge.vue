<script setup lang="ts">
import { getStatusColor } from '../utils/statusColors'
import type { HealthStatus } from '../types'

interface Props {
  status: HealthStatus
}

defineProps<Props>()
</script>

<template>
  <div
    class="px-4 py-1.5 rounded-full border-2 border-gray-900 text-sm font-medium capitalize text-gray-900"
    :style="{ backgroundColor: getStatusColor(status) }"
  >
    {{ status }}
  </div>
</template>
