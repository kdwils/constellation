<script setup lang="ts">
interface Props {
  show: boolean
}

defineProps<Props>()
</script>

<template>
  <div
    v-if="show"
    class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 z-50 bg-white border-2 border-gray-900 rounded-lg p-3 shadow-lg text-sm min-w-[250px]"
  >
    <slot />
  </div>
</template>
