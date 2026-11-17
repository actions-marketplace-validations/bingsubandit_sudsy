<script>
export default {
  props: {
    a: String,
//  ^^^^^^^^^ require-default-prop
    b: { type: String, default: '' },
    c: { required: true, type: Number },
    d: Boolean,
    e: [String, Number],
//  ^^^^^^^^^^^^^^^^^^^ require-default-prop
    f: {},
//  ^^^^^ require-default-prop
//  ^^^^^ require-prop-types
    g: Object,
//  ^^^^^^^^^ require-default-prop
  },
  data() {
    return { a: 1, h: 2 }
//           ^ no-dupe-keys
  },
  computed: {
    h() { return 1 },
//  ^ no-dupe-keys
    get k() { return 1 },
    set k(v) {},
  },
  methods: { go() {} },
}
</script>

<script setup>
defineProps(['x', 'y'])
//           ^^^ require-prop-types
//                ^^^ require-prop-types
const ok = 1
const model = defineModel()
//            ^^^^^^^^^^^^^ require-prop-types
</script>
