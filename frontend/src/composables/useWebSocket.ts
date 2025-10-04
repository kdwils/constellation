import { ref, onMounted, onUnmounted } from 'vue'

export function useWebSocket() {
  const ws = ref<WebSocket | null>(null)
  const isConnected = ref(false)
  const lastMessage = ref<any>(null)

  const connect = () => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${protocol}//${window.location.host}/ws`

    console.log(`[WebSocket] Attempting to connect to ${wsUrl}`)

    ws.value = new WebSocket(wsUrl)

    ws.value.onopen = () => {
      isConnected.value = true
      console.log('[WebSocket] Connected successfully')
    }

    ws.value.onmessage = (event) => {
      console.log('[WebSocket] Message received:', event.data)
      try {
        const data = JSON.parse(event.data)
        lastMessage.value = data
        console.log('[WebSocket] Parsed message:', data)
      } catch (error) {
        console.error('[WebSocket] Failed to parse message:', error)
      }
    }

    ws.value.onerror = (error) => {
      console.error('[WebSocket] Error occurred:', error)
    }

    ws.value.onclose = (event) => {
      isConnected.value = false
      console.log('[WebSocket] Disconnected', {
        code: event.code,
        reason: event.reason,
        wasClean: event.wasClean
      })

      // Attempt reconnection after 3 seconds
      setTimeout(() => {
        if (!isConnected.value) {
          console.log('[WebSocket] Attempting to reconnect...')
          connect()
        }
      }, 3000)
    }
  }

  const disconnect = () => {
    if (ws.value) {
      console.log('[WebSocket] Closing connection')
      ws.value.close()
      ws.value = null
    }
  }

  onMounted(() => {
    connect()
  })

  onUnmounted(() => {
    disconnect()
  })

  return {
    isConnected,
    lastMessage,
    disconnect,
    reconnect: connect
  }
}
