<template>
  <MyCard :title="t" @click="go" :cardTitle="c" cardSub="s" my-prop="p"
    :ok="1"
    :cardOther="o">
    <template #head>h</template>
    <template v-slot="p">d</template>
  </MyCard>
  <svg :viewBox="v"></svg>
</template>
