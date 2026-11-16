<template>
  <p v-else>no if before</p>
//   ^^^^^^ valid-v-else
  <p v-if="a">a</p>
  <!-- a comment may sit between -->
  <p v-else-if="b">b</p>
  <p v-else>c</p>
  <p v-else-if="d">after else</p>
//   ^^^^^^^^^ valid-v-else-if
  <p v-if="a">a</p>
  text
  <p v-else-if="x">b</p>
//   ^^^^^^^^^ valid-v-else-if
  <p v-else="y">value</p>
//   ^^^^^^ valid-v-else
  <p v-if="a" v-else-if="b">both</p>
//   ^^^^ valid-v-if
//            ^^^^^^^^^ valid-v-else-if
</template>
