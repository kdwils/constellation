<script setup lang="ts">
import { computed } from 'vue'
import ServiceCard from './ServiceCard.vue'
import { websocket } from '../composables/websocket'
import { transformToServiceCards } from '../composables/useHealthData'
import { getStatusColor } from '../utils/statusColors'
import type { ServiceCardData, HealthStatus } from '../types'

const { lastMessage } = websocket()
const healthData = computed(() => lastMessage.value || [])
const isLoading = computed(() => !lastMessage.value)

const serviceCards = computed<ServiceCardData[]>(() => {
  return transformToServiceCards(healthData.value)
})

const overallStatus = computed<HealthStatus>(() => {
  const cards = serviceCards.value

  if (cards.length === 0) {
    return 'unknown'
  }

  const hasUnhealthy = cards.some(card => card.status === 'unhealthy')
  if (hasUnhealthy) {
    return 'unhealthy'
  }

  const allHealthy = cards.every(card => card.status === 'healthy')
  if (allHealthy) {
    return 'healthy'
  }

  return 'unknown'
})
</script>

<template>
  <div class="h-screen bg-gray-900 overflow-hidden">
    <!-- Outer Container -->
    <div class="h-full flex flex-col">
      <!-- Header -->
      <header class="mb-8 border-[3px] border-gray-700 rounded-2xl p-6 bg-gray-800 flex-shrink-0 mx-6 mt-6">
        <div class="flex justify-between items-center">
          <div>
            <h1 class="text-4xl font-bold text-gray-100">
              Constellation
            </h1>
            <p class="text-sm text-gray-400 mt-2">A simple health check dashboard</p>
          </div>
          <div
            class="w-8 h-8 rounded-full border-2 border-gray-700 transition-colors duration-300"
            :style="{ backgroundColor: getStatusColor(overallStatus) }"
            :title="`Overall status: ${overallStatus}`"
          />
        </div>
      </header>

      <!-- Scrollable Content Area -->
      <div class="flex-1 overflow-y-auto px-6 pt-6">
        <!-- Loading State -->
        <div v-if="isLoading" class="text-center py-20">
          <p class="text-gray-400">Loading health data...</p>
        </div>

        <!-- Empty State -->
        <div v-else-if="serviceCards.length === 0" class="text-center py-20">
          <p class="text-gray-400">No services found</p>
        </div>

        <!-- Service Cards Grid -->
        <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 pb-6">
          <ServiceCard
            v-for="service in serviceCards"
            :key="`${service.namespace}-${service.name}`"
            :service="service"
          />
        </div>
      </div>
    </div>
  </div>
</template>
