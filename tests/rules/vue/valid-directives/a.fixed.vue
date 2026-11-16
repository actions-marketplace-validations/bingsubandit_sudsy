<template>
  <div v-if v-bind v-model v-html v-text v-show v-once="x" v-pre="x" v-cloak="x" v-memo v-is />
  <b v-for="xs" :key="1" :a="" :a.x="1" #a.m v-if:a="1" @click />
  <b v-for="x in xs" :key="x" @click.once="x" :b.camel="x" @click.prevent @keyup="k" />
  <input type="file" v-model="f" /><div v-if="a" #item></div>
  <p v-if="ok" :a>
    <template #default="p"><C #item="{ a }" /></template>
  </p>
</template>
