<template>
  <div>
    <template><p>lone</p></template>
//   ^^^^^^^^ no-lone-template
    <template v-if="a"><p>ok</p></template>
    <template #head><p>ok</p></template>
    <textarea>{{ text }}</textarea>
//            ^^^^^^^^^^ no-textarea-mustache
    <component></component>
//   ^^^^^^^^^ require-component-is
    <component :is="view" />
    <template v-for="x in xs"><li :key="x" /></template>
//  ^^^^^^^^^^^^^^^^^^^^^^^^^^ v-for-key
//                                ^^^^ no-v-for-template-key-on-child
    <template v-for="x in xs" :key="x"><li /></template>
    <p v-html="h">child</p>
//     ^^^^^^ no-child-content
    <p v-text="t"> </p>
  </div>
</template>
