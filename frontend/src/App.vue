<script setup lang="ts">
import { useWebSocket } from './composables/useWebSocket'

const { isConnected, lastMessage } = useWebSocket()
</script>

<template>
  <div>
    <h1>Constellation Dashboard</h1>
    <div class="status">
      <p>WebSocket Status: <strong :class="{ connected: isConnected, disconnected: !isConnected }">
        {{ isConnected ? 'Connected' : 'Disconnected' }}
      </strong></p>
      <div v-if="lastMessage" class="message">
        <h3>Latest Message:</h3>
        <pre>{{ JSON.stringify(lastMessage, null, 2) }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.status {
  margin: 2rem;
  padding: 1rem;
  border: 1px solid #ccc;
  border-radius: 8px;
}

.connected {
  color: #42b883;
}

.disconnected {
  color: #f56c6c;
}

.message {
  margin-top: 1rem;
  padding: 1rem;
  background: #f5f5f5;
  border-radius: 4px;
}

pre {
  text-align: left;
  overflow-x: auto;
  font-size: 0.9em;
}
</style>
