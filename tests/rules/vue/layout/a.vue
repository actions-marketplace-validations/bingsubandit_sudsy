<template>
  <div>
    <div></div>
//  ^^^^^ html-self-closing
    <img />
//  ^^^^^^^ html-self-closing
    <img>
    <div />
    <div>text</div>
    <MyCard></MyCard>
//  ^^^^^^^^ html-self-closing
    <svg><path d="M0"></path></svg>
//       ^^^^^^^^^^^^^ html-self-closing
    <a @click="go" :href="u" v-if="ok">x</a>
//                 ^^^^^ attributes-order
//                           ^^^^ attributes-order
    <a v-if="ok" :href="u" @click="go">x</a>
    <input v-bind="props" :id="i" v-model="m">
    <MyCard @cardOpen="go" @card-close="go" @click="go" />
//          ^^^^^^^^^ v-on-event-hyphenation
  </div>
</template>
