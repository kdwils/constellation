<script setup lang="ts">
import { useFloating, flip, shift, offset } from '@floating-ui/vue'
import { ref, computed } from 'vue'

interface Props {
  show: boolean
  referenceEl?: HTMLElement | null
}

const props = defineProps<Props>()

const floating = ref<HTMLElement | null>(null)

const reference = computed(() => props.referenceEl)

const { floatingStyles } = useFloating(reference, floating, {
  placement: 'top',
  middleware: [
    offset(8),
    flip(),
    shift({ padding: 8 })
  ]
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="show"
      ref="floating"
      :style="floatingStyles"
      class="z-50 bg-gray-800 border-2 border-gray-700 rounded-lg p-3 shadow-lg text-sm min-w-[250px] max-w-[400px]"
    >
      <slot />
    </div>
  </Teleport>
</template>
