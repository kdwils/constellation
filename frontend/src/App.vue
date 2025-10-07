<script setup lang="ts">
import { computed } from 'vue'
import ServiceCard from './components/ServiceCard.vue'
import { websocket } from './composables/websocket'
import { transformToServiceCards } from './composables/useHealthData'
import type { ServiceCardData } from './types'

const { lastMessage } = websocket()
const hierarchyData = computed(() => lastMessage.value || [])
const isLoading = computed(() => !lastMessage.value)

const serviceCards = computed<ServiceCardData[]>(() => {
  return transformToServiceCards(hierarchyData.value)
})

const overallStatus = computed(() => {
  const cards = serviceCards.value
  if (cards.length === 0) return 'unknown'

  const hasUnhealthy = cards.some(card => card.status === 'unhealthy')
  if (hasUnhealthy) return 'unhealthy'

  const allHealthy = cards.every(card => card.status === 'healthy')
  if (allHealthy) return 'healthy'

  return 'unknown'
})

const getStatusColor = (status: string): string => {
  if (status === 'healthy') return '#4ade80'
  if (status === 'unhealthy') return '#f87171'
  return '#fbbf24'
}
</script>

<template>
  <div class="min-h-screen bg-gray-100 p-8">
    <!-- Outer Container -->
    <div class="border-[3px] border-gray-900 rounded-3xl bg-white p-8 min-h-[calc(100vh-4rem)]">
      <!-- Header -->
      <header class="mb-8 border-[3px] border-gray-900 rounded-2xl p-6 bg-gray-50">
        <div class="flex justify-between items-center">
          <div>
            <h1 class="text-4xl font-bold text-gray-900">
              Constellation
            </h1>
            <p class="text-sm text-gray-600 mt-2">A simple health check dashboard</p>
          </div>
          <div
            class="w-8 h-8 rounded-full border-2 border-gray-900 transition-colors duration-300"
            :style="{ backgroundColor: getStatusColor(overallStatus) }"
            :title="`Overall status: ${overallStatus}`"
          />
        </div>
      </header>

      <!-- Loading State -->
      <div v-if="isLoading" class="text-center py-20">
        <p class="text-gray-600">Loading health data...</p>
      </div>

      <!-- Empty State -->
      <div v-else-if="serviceCards.length === 0" class="text-center py-20">
        <p class="text-gray-600">No services found</p>
      </div>

      <!-- Service Cards Grid -->
      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <ServiceCard
          v-for="service in serviceCards"
          :key="`${service.namespace}-${service.name}`"
          :service="service"
        />
      </div>
    </div>
  </div>
</template>

