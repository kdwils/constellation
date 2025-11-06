import { ref, onUnmounted } from 'vue'
import type { ServiceHealthInfo } from '../types'

export function websocket() {
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const wsUrl = `${protocol}//${window.location.host}/ws`

  const isConnected = ref(false)
  const lastMessage = ref<ServiceHealthInfo[] | null>(null)

  const websocket = new WebSocket(wsUrl)

  websocket.onopen = () => {
    isConnected.value = true
  }

  websocket.onmessage = (event) => {
    try {
      lastMessage.value = JSON.parse(event.data)
    } catch {
      lastMessage.value = null
    }
  }

  websocket.onclose = () => {
    isConnected.value = false
  }

  websocket.onerror = (error) => {
    console.error('[WebSocket] Error:', error)
  }

  onUnmounted(() => {
    websocket.close()
  })

  return {
    isConnected,
    lastMessage
  }
}
