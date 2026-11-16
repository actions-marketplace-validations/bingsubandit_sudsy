<template>
  <MyCard v-bind:title="t" v-on:click="go" :cardTitle="c" cardSub="s" my-prop="p"
//        ^^^^^^^ v-bind-style
//                         ^^^^^ v-on-style
//                                         ^^^^^^^^^^ attribute-hyphenation
//                                                        ^^^^^^^ attribute-hyphenation
    :ok="1"
    :cardOther="o">
//  ^^^^^^^^^^ attribute-hyphenation
    <template v-slot:head>h</template>
//            ^^^^^^^ v-slot-style
    <template v-slot="p">d</template>
  </MyCard>
  <svg :viewBox="v"></svg>
</template>
