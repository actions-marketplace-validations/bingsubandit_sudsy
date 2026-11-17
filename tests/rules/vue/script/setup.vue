<script lang="ts">
export interface Props {
  as?: any
//^^^^^^^^ require-default-prop
  size?: Size
//^^^^^^^^^^^ require-default-prop
  open?: boolean
  flag?: boolean | undefined
//^^^^^^^^^^^^^^^^^^^^^^^^^^ require-default-prop
  need: string
  ui?: Record<string, string>
//^^^^^^^^^^^^^^^^^^^^^^^^^^^ require-default-prop
  tag?: string
}
type Size = 'sm' | 'md'
</script>

<script setup lang="ts">
const _props = withDefaults(defineProps<Props>(), { tag: 'div' })
const props = useThing(_props)
const ui = computed(() => props.size)
//    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no-dupe-keys
const as2 = _props.as
const emit = defineEmits<{ go: [] }>()
const model = defineModel<string>('size')
//                                ^^^^^^ no-dupe-keys
</script>

<template>
  <div />
</template>
