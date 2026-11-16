<template>
  <div v-if v-bind v-model v-html v-text v-show v-once="x" v-pre="x" v-cloak="x" v-memo v-is />
//     ^^^^ valid-v-if
//          ^^^^^^ valid-v-bind
//                 ^^^^^^^ valid-v-model
//                         ^^^^^^ valid-v-html
//                                ^^^^^^ valid-v-text
//                                       ^^^^^^ valid-v-show
//                                              ^^^^^^ valid-v-once
//                                                         ^^^^^ valid-v-pre
//                                                                   ^^^^^^^ valid-v-cloak
//                                                                               ^^^^^^ valid-v-memo
//                                                                                      ^^^^ no-deprecated-v-is
//                                                                                      ^^^^ valid-v-is
  <b v-for="xs" :key="1" :a="" :a.x="1" #a.m v-if:a="1" @click />
//   ^^^^^ valid-v-for
//                       ^^ valid-v-bind
//                             ^^^^ valid-v-bind
//                                      ^^^^ valid-v-slot
//                                           ^^^^^^ valid-v-if
//                                                      ^^^^^^ valid-v-on
  <b v-for="x in xs" :key="x" v-on:click.once="x" :b.camel="x" @click.prevent @keyup="k" />
//                            ^^^^^ v-on-style
  <input type="file" v-model="f" /><div v-if="a" #item></div>
//                   ^^^^^^^ valid-v-model
//                                               ^^^^^ valid-v-slot
  <p v-if="ok" :a>
    <template #default="p"><C #item="{ a }" /></template>
  </p>
</template>
