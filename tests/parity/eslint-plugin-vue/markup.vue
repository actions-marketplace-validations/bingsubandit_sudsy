<template>
  <div>
    <ul>
      <li v-for="x in xs">no key</li>
      <li v-for="x in xs" :key="x">keyed</li>
      <template v-for="x in xs">
        <li>no key on the child</li>
      </template>
      <template v-for="x in xs">
        <li :key="x">key on the child</li>
      </template>
    </ul>
    <a href="https://example.org" target="_blank">no rel</a>
    <a href="https://example.org" target="_blank" rel="noreferrer">only noreferrer</a>
    <a href="https://example.org" target="_blank" rel="noopener noreferrer">both</a>
    <a :href="url" target="_blank">dynamic</a>
    <a href="/local" target="_blank">local</a>
    <p v-if="a">a</p>
    <p v-else-if="b">b</p>
    <p v-else-if="b">b again</p>
    <p v-else-if="a || c">covered only in part</p>
    <p v-else>else</p>
    <Child>
      <template v-for="x in xs" #[x]>
        <slot :name="x" />
      </template>
    </Child>
    <Child :someProp="1" @someEvent="go" />
    <Child :some-prop="1" @some-event="go" />
    <div></div>
    <img />
    <Child></Child>
    <Child />
    <input v-model="a" :id="a" v-if="b" />
    <button @click="go" :disabled="b" v-if="a">go</button>
    <Child v-html="h" />
    <div v-html="h"></div>
    <button type="button" v-bind="obj">ok</button>
    <button>no type</button>
    <textarea>{{ text }}</textarea>
    <component />
    <component :is="view" />
    <template><p>lone</p></template>
    <p v-text="t">child</p>
    <li v-else v-for="x in xs"></li>
    <li v-if="a" v-for="x in xs"></li>
  </div>
</template>

<script setup>
const a = 1
const b = 2
const c = 3
const xs = []
const url = ''
const h = ''
const t = ''
const text = ''
const view = null
const obj = {}
function go() {}
</script>
