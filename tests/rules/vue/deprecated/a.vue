<template>
  <C :title.sync="t" @click.native="go" @keyup.13="k" inline-template v-is="c" />
//   ^^^^^^^^^^^ no-deprecated-v-bind-sync
//                   ^^^^^^^^^^^^^ no-deprecated-v-on-native-modifier
//                                      ^^^^^^^^^ no-deprecated-v-on-number-modifiers
//                                                    ^^^^^^^^^^^^^^^ no-deprecated-inline-template
//                                                                    ^^^^ no-deprecated-v-is
  <p slot="head" is="row"></p><slot name="x" />
//   ^^^^ no-deprecated-slot-attribute
//               ^^ no-deprecated-html-element-is
  <template scope="p" slot-scope="q" functional></template>
//          ^^^^^ no-deprecated-scope-attribute
//                    ^^^^^^^^^^ no-deprecated-slot-scope-attribute
//                                   ^^^^^^^^^^ no-deprecated-functional-template
  <tr is="vue:row" v-model:title="t" @keyup.enter="k"></tr>
</template>
