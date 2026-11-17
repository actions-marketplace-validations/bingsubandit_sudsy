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
//                                ^^^^ no-v-for-template-key-on-child
    <li v-for="x in xs">k</li>
//  ^^^^^^^^^^^^^^^^^^^^ v-for-key
    <template v-for="x in xs"><li>k</li></template>
//                            ^^^^ v-for-key
    <MyCard v-for="x in xs" />
    <MyCard v-html="h" />
//          ^^^^^^ no-v-text-v-html-on-component
    <b v-if="a" /><i v-else-if="a || b" /><i v-else-if="a" />
//                                           ^^^^^^^^^ no-dupe-v-else-if
    <li v-else v-for="x in xs" :key="x" />
//      ^^^^^^ no-use-v-else-with-v-for
    <template v-for="x in xs" :key="x"><li /></template>
    <p v-html="h">child</p>
//     ^^^^^^ no-child-content
    <p v-text="t"> </p>
//     ^^^^^^ no-v-text
    <a :href="u" target="_blank">x</a>
//               ^^^^^^ no-target-blank
  </div>
</template>
