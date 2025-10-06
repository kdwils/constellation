<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import ServiceCard from './components/ServiceCard.vue'
import { websocket } from './composables/websocket'
import { transformToServiceCards } from './composables/useHealthData'
import type { HierarchyNode, ServiceCardData } from './types'

const hierarchyData = ref<HierarchyNode[]>([])
const isLoading = ref(true)
const { isConnected, lastMessage } = websocket()

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

onMounted(async () => {
  try {
    const response = await fetch('/state')
    const data = await response.json()
    hierarchyData.value = data
  } catch (error) {
    console.error('Failed to fetch initial state:', error)
  } finally {
    isLoading.value = false
  }
})

watch(lastMessage, (newMessage) => {
  if (newMessage) {
    hierarchyData.value = newMessage
  }
})
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

    <!-- Connection Status -->
    <div class="fixed bottom-4 right-4 flex items-center gap-2 bg-white border-2 border-gray-900 rounded-full px-4 py-2 text-sm">
      <div
        class="w-2 h-2 rounded-full"
        :class="isConnected ? 'bg-green-400' : 'bg-red-400'"
      />
      <span>{{ isConnected ? 'Connected' : 'Disconnected' }}</span>
    </div>
  </div>
</template>

