<template>
  <div v-html="comment"></div>
//^^^^^^^^^^^^^^^^^^^^^^ no-v-html
  <div>{{ comment }}</div>
</template>
